use std::path::Path;
use axiom_core::parser;

/// Validate all ARS files at a path. Prints errors with file:line references.
/// Returns the number of files with errors.
pub fn run(path: &Path) -> anyhow::Result<usize> {
    let mut error_count = 0usize;
    let mut file_count  = 0usize;

    for entry in walk_ars_files(path) {
        file_count += 1;
        let bytes = match std::fs::read(&entry) {
            Ok(b)  => b,
            Err(e) => {
                eprintln!("{}: could not read file: {e}", entry.display());
                error_count += 1;
                continue;
            }
        };

        let is_json = entry.extension().and_then(|e| e.to_str()) == Some("json");
        let result = validate_bytes(&bytes, is_json);

        match result {
            Ok(kind) => println!("{}: OK ({})", entry.display(), kind),
            Err(e) => {
                eprintln!("{}: ERROR — {e}", entry.display());
                error_count += 1;
            }
        }
    }

    if file_count == 0 {
        eprintln!("No ARS files found at {}", path.display());
    }

    println!("\nValidated {file_count} file(s), {error_count} error(s).");
    Ok(error_count)
}

/// Returns a short description of what was validated ("rule" or "bundle (N rules)").
fn validate_bytes(bytes: &[u8], is_json: bool) -> anyhow::Result<String> {
    // Peek at the top-level keys to distinguish a bundle from a single rule.
    if is_bundle(bytes, is_json) {
        let (rules, rulesets) = if is_json {
            parser::parse_bundle_json(bytes)?
        } else {
            parser::parse_bundle_yaml(bytes)?
        };
        return Ok(format!("bundle ({} rules, {} rulesets)", rules.len(), rulesets.len()));
    }

    if is_json {
        parser::parse_rule_json(bytes)?;
    } else {
        parser::parse_rule_yaml(bytes)?;
    }
    Ok("rule".to_string())
}

/// A file is treated as a bundle if its top-level YAML/JSON object has a `rules` key.
fn is_bundle(bytes: &[u8], is_json: bool) -> bool {
    if is_json {
        if let Ok(v) = serde_json::from_slice::<serde_json::Value>(bytes) {
            return v.get("rules").is_some();
        }
    } else if let Ok(v) = serde_yaml::from_slice::<serde_yaml::Value>(bytes) {
        return v.get("rules").is_some();
    }
    false
}

fn walk_ars_files(root: &Path) -> Vec<std::path::PathBuf> {
    let mut files = Vec::new();
    if root.is_file() {
        files.push(root.to_path_buf());
        return files;
    }
    if let Ok(entries) = std::fs::read_dir(root) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                files.extend(walk_ars_files(&p));
            } else if let Some(ext) = p.extension().and_then(|e| e.to_str()) {
                if ext == "yaml" || ext == "yml" || ext == "json" {
                    files.push(p);
                }
            }
        }
    }
    files
}
