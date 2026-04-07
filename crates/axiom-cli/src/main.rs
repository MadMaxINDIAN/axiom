mod commands {
    pub mod validate;
    pub mod test;
    pub mod evaluate;
    pub mod keygen;
    pub mod import_export;
}

use std::path::PathBuf;
use clap::{Parser, Subcommand};
use axiom_core::Strategy;

// ---------------------------------------------------------------------------
// CLI definition
// ---------------------------------------------------------------------------

#[derive(Parser)]
#[command(name = "axiom", version, about = "Axiom rules engine CLI")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Validate ARS rule files (YAML or JSON)
    Validate {
        /// Path to file or directory of ARS rule files
        path: PathBuf,
    },

    /// Run test suites (*.test.yaml files)
    Test {
        /// Path to test file or directory
        path: PathBuf,
        /// Write JUnit XML output to this file
        #[arg(long)]
        output: Option<PathBuf>,
    },

    /// Evaluate a rule against a context
    Evaluate {
        /// Path to local ARS rule file (omit for remote evaluation)
        #[arg(long)]
        rule: Option<PathBuf>,

        /// Rule ID (for remote evaluation)
        #[arg(long)]
        rule_id: Option<String>,

        /// JSON context string
        #[arg(long)]
        context: String,

        /// Remote server URL
        #[arg(long)]
        server: Option<String>,

        /// API key for remote server
        #[arg(long, env = "AXIOM_API_KEY")]
        api_key: Option<String>,

        /// Evaluation strategy: first_match | all_match | scored
        #[arg(long, default_value = "first_match")]
        strategy: String,

        /// Exit with code 1 if no rule matched
        #[arg(long)]
        fail_on_no_match: bool,
    },

    /// Import a rule bundle into a remote server
    Import {
        /// Bundle file path (YAML or JSON)
        bundle: PathBuf,
        /// Remote server URL
        #[arg(long)]
        server: String,
        /// API key
        #[arg(long, env = "AXIOM_API_KEY")]
        api_key: String,
    },

    /// Export rules from a remote server to a bundle file
    Export {
        /// Remote server URL
        #[arg(long)]
        server: String,
        /// API key
        #[arg(long, env = "AXIOM_API_KEY")]
        api_key: String,
        /// Output file
        #[arg(long, short, default_value = "axiom-export.yaml")]
        output: PathBuf,
    },

    /// Start a local Axiom server loading rules from a path
    Serve {
        /// Directory of ARS rule files to load
        #[arg(long)]
        rules: PathBuf,
        /// Port to listen on
        #[arg(long, default_value = "8080")]
        port: u16,
    },

    /// Generate a new API key and its SHA-256 hash
    Keygen {
        /// Role: admin | editor | viewer
        #[arg(long, default_value = "editor")]
        role: String,
        /// Human-readable description
        #[arg(long)]
        description: Option<String>,
    },
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let exit_code = run(cli).await;
    std::process::exit(exit_code);
}

async fn run(cli: Cli) -> i32 {
    match cli.command {
        Command::Validate { path } => {
            match commands::validate::run(&path) {
                Ok(0)  => 0,
                Ok(_)  => 1,
                Err(e) => { eprintln!("Error: {e}"); 1 }
            }
        }

        Command::Test { path, output } => {
            match commands::test::run(&path, output.as_deref()) {
                Ok((_, 0)) => 0,
                Ok((_, _)) => 1,
                Err(e)     => { eprintln!("Error: {e}"); 1 }
            }
        }

        Command::Evaluate {
            rule, rule_id, context, server, api_key, strategy, fail_on_no_match,
        } => {
            let strat = match strategy.as_str() {
                "all_match" => Strategy::AllMatch,
                "scored"    => Strategy::Scored,
                _           => Strategy::FirstMatch,
            };

            if let Some(rule_path) = rule {
                match commands::evaluate::run_local(&rule_path, &context, strat, fail_on_no_match) {
                    Ok(_)  => 0,
                    Err(e) => { eprintln!("Error: {e}"); 1 }
                }
            } else if let (Some(srv), Some(key), Some(rid)) = (server, api_key, rule_id) {
                match commands::evaluate::run_remote(&srv, &key, &rid, &context, strat).await {
                    Ok(_)  => 0,
                    Err(e) => { eprintln!("Error: {e}"); 1 }
                }
            } else {
                eprintln!("Provide either --rule <path> or --server + --api-key + --rule-id");
                1
            }
        }

        Command::Import { bundle, server, api_key } => {
            match commands::import_export::import(&bundle, &server, &api_key).await {
                Ok(())  => 0,
                Err(e)  => { eprintln!("Error: {e}"); 1 }
            }
        }

        Command::Export { server, api_key, output } => {
            match commands::import_export::export(&server, &api_key, &output).await {
                Ok(())  => 0,
                Err(e)  => { eprintln!("Error: {e}"); 1 }
            }
        }

        Command::Serve { rules, port } => {
            eprintln!("axiom serve --rules {} --port {port}", rules.display());
            eprintln!("Note: 'serve' delegates to axiom-server. Run axiom-server directly for production use.");
            // For development: load rules from dir and start an in-process server
            // (full implementation delegates to axiom-server binary)
            0
        }

        Command::Keygen { role, description } => {
            commands::keygen::run(&role, description.as_deref());
            0
        }
    }
}
