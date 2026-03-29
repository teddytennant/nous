use clap::Parser;
use nous_cli::{Cli, CliConfig};
use tracing_subscriber::EnvFilter;

mod executor;

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let filter = if cli.verbose {
        "debug"
    } else {
        "warn"
    };
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| filter.into()))
        .init();

    let config = CliConfig::default();
    let exec = match executor::Executor::new(&config.data_dir, cli.json) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
    };

    if let Err(e) = exec.execute(cli.command).await {
        if cli.json {
            eprintln!(
                "{}",
                serde_json::json!({"status": "error", "message": e})
            );
        } else {
            eprintln!("error: {e}");
        }
        std::process::exit(1);
    }
}
