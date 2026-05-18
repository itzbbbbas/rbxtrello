use clap::{Parser, Subcommand};
use log::info;

use rbxtrello::{Result, api, sync};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Auto-confirm all prompts
    #[arg(short = 'y', long, default_value_t = false, global = true)]
    yes: bool,

    /// Print diff, make no changes
    #[arg(long, default_value_t = false, global = true)]
    dry_run: bool,

    /// Override [metadata].board_id
    #[arg(long, global = true)]
    board_id: Option<String>,

    /// Skip TUI diff in `sync` — auto-confirm all changes
    #[arg(short = 'a', long, default_value_t = false, global = true)]
    auto_confirm: bool,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Write a starter rbxtrello.toml in the current directory
    Init,
    /// Fetch remote board → overwrite rbxtrello.toml (preserves comments)
    Pull,
    /// Diff local toml against remote board, confirm, push changes
    Sync,
}

fn env_truthy(key: &str) -> bool {
    std::env::var(key)
        .map(|v| matches!(v.trim().to_ascii_lowercase().as_str(), "1" | "true" | "yes"))
        .unwrap_or(false)
}

fn init_logging() {
    if std::env::var("RUST_LOG").is_err() {
        let crate_name = env!("CARGO_PKG_NAME");
        let filter = if cfg!(debug_assertions) {
            format!("off,{crate_name}=debug")
        } else {
            format!("{crate_name}=info")
        };
        unsafe { std::env::set_var("RUST_LOG", filter) }
    }
    env_logger::init();
}

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    init_logging();
    let _ = color_eyre::install();

    let key = std::env::var("TRELLO_KEY").ok().unwrap_or_default();
    let token = std::env::var("TRELLO_TOKEN").ok().unwrap_or_default();
    let key_set = !key.trim().is_empty();
    let token_set = !token.trim().is_empty();
    info!(
        "auth: TRELLO_KEY={} TRELLO_TOKEN={}",
        if key_set { "set" } else { "MISSING" },
        if token_set { "set" } else { "MISSING" }
    );
    if key_set && token_set {
        api::set_credentials(key, token).await;
    }

    let args = Args::parse();
    let command = match args.command {
        Some(cmd) => cmd,
        None => {
            eprintln!("No command provided. Use --help for more information.");
            std::process::exit(2);
        }
    };

    let result: Result<()> = match command {
        Commands::Init => init_cmd(),
        Commands::Pull => {
            require_auth();
            sync::pull::run(args.board_id.clone()).await
        }
        Commands::Sync => {
            require_auth();
            let auto_confirm = args.auto_confirm
                || args.yes
                || args.dry_run
                || env_truthy("RBXTRELLO_AUTO_CONFIRM");
            sync::push::run(sync::push::PushOptions {
                board_id_override: args.board_id.clone(),
                auto_confirm,
                dry_run: args.dry_run,
            })
            .await
        }
    };

    if let Err(e) = result {
        log::error!("{e:#}");
        eprintln!("error: {e:#}");
        std::process::exit(1);
    }
}

fn init_cmd() -> Result<()> {
    info!("Initializing rbxtrello.toml...");
    if std::fs::exists("rbxtrello.toml")? {
        anyhow::bail!("rbxtrello.toml already exists in cwd. Aborting.");
    }
    let template = include_str!("templates/rbxtrello.toml.template");
    std::fs::write("rbxtrello.toml", template)?;
    info!("rbxtrello.toml initialized.");
    Ok(())
}

fn require_auth() {
    let key = std::env::var("TRELLO_KEY").unwrap_or_default();
    let token = std::env::var("TRELLO_TOKEN").unwrap_or_default();
    if key.trim().is_empty() || token.trim().is_empty() {
        eprintln!(
            "error: TRELLO_KEY and TRELLO_TOKEN must be set (in .env or environment).\n\
             Get a key: https://trello.com/app-key"
        );
        std::process::exit(1);
    }
}
