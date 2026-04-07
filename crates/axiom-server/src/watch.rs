//! Filesystem hot-reload for YAML rule files (§6.5).
//!
//! When `rules_dir` is configured, this module watches that directory with the
//! `notify` crate. Any `.yaml` / `.yml` file that is created or modified is
//! parsed as an ARS rule and upserted into the store; deletes remove the rule
//! whose ID matches the stem of the filename. After each change the in-memory
//! registry is synchronised from the store.

use std::path::{Path, PathBuf};
use std::time::Duration;

use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use tokio::sync::mpsc;
use tracing::{error, info, warn};

use crate::state::AppState;

/// Spawn a background task that watches `rules_dir` and reloads rules on change.
/// Returns immediately; the watcher runs until the process exits.
pub fn spawn_watch(state: AppState, rules_dir: impl AsRef<Path>) {
    let rules_dir: PathBuf = rules_dir.as_ref().to_path_buf();

    let (tx, mut rx) = mpsc::channel::<PathBuf>(64);

    // `RecommendedWatcher` is synchronous; bridge events into a tokio channel
    // via a std closure that sends to the mpsc sender.
    let tx_clone = tx.clone();
    let mut watcher: RecommendedWatcher = match notify::recommended_watcher(
        move |res: notify::Result<Event>| {
            match res {
                Ok(event) => {
                    let is_relevant = matches!(
                        event.kind,
                        EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)
                    );
                    if is_relevant {
                        for path in event.paths {
                            if is_yaml(&path) {
                                let _ = tx_clone.blocking_send(path);
                            }
                        }
                    }
                }
                Err(e) => warn!(error = %e, "notify watcher error"),
            }
        },
    ) {
        Ok(w) => w,
        Err(e) => {
            error!(error = %e, "Failed to create filesystem watcher — hot-reload disabled");
            return;
        }
    };

    if let Err(e) = watcher.watch(&rules_dir, RecursiveMode::NonRecursive) {
        error!(dir = %rules_dir.display(), error = %e, "Cannot watch rules_dir — hot-reload disabled");
        return;
    }

    info!(dir = %rules_dir.display(), "Hot-reload: watching for YAML rule changes");

    tokio::spawn(async move {
        // Keep `watcher` alive for the duration of this task.
        let _watcher = watcher;

        // Debounce: collect events, then wait 200 ms for the burst to settle.
        loop {
            // Wait for the first event.
            let Some(path) = rx.recv().await else { break };

            // Drain any additional events that arrive within the debounce window.
            let mut paths = vec![path];
            let debounce = tokio::time::sleep(Duration::from_millis(200));
            tokio::pin!(debounce);
            loop {
                tokio::select! {
                    () = &mut debounce => break,
                    Some(p) = rx.recv() => paths.push(p),
                }
            }

            // Deduplicate.
            paths.sort();
            paths.dedup();

            for path in paths {
                handle_path_event(&state, &path).await;
            }
        }
    });
}

fn is_yaml(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|e| e.to_str()),
        Some("yaml") | Some("yml")
    )
}

async fn handle_path_event(state: &AppState, path: &Path) {
    if path.exists() {
        upsert_rule_file(state, path).await;
    } else {
        remove_rule_by_path(state, path).await;
    }
}

async fn upsert_rule_file(state: &AppState, path: &Path) {
    let raw = match tokio::fs::read_to_string(path).await {
        Ok(s) => s,
        Err(e) => {
            warn!(path = %path.display(), error = %e, "Hot-reload: failed to read file");
            return;
        }
    };

    let rule: axiom_core::Rule = match serde_yaml::from_str(&raw) {
        Ok(r) => r,
        Err(e) => {
            warn!(path = %path.display(), error = %e, "Hot-reload: YAML parse error — skipping");
            return;
        }
    };

    let rule_id = rule.id.clone();

    if let Err(e) = state.store().upsert_rule(rule.clone()).await {
        error!(rule_id, error = %e, "Hot-reload: store upsert failed");
        return;
    }

    {
        let mut reg = state.registry_write().await;
        let _ = reg.load_rules(vec![rule]);
    }

    info!(rule_id, path = %path.display(), "Hot-reload: rule upserted");
}

async fn remove_rule_by_path(state: &AppState, path: &Path) {
    // Convention: filename stem is the rule ID.
    let rule_id = match path.file_stem().and_then(|s| s.to_str()) {
        Some(id) => id.to_string(),
        None => return,
    };

    if let Err(e) = state.store().disable_rule(&rule_id).await {
        warn!(rule_id, error = %e, "Hot-reload: store disable failed (rule may not exist)");
    }

    {
        let mut reg = state.registry_write().await;
        reg.disable_rule(&rule_id);
    }

    info!(rule_id, path = %path.display(), "Hot-reload: rule disabled (file deleted)");
}
