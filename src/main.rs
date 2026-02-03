//! Application entry point and CLI handling.

use std::path::PathBuf;

use color_eyre::Result;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use feedo::{App, Config};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize error handling
    color_eyre::install()?;

    // Initialize logging (RUST_LOG=debug for verbose output)
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn")))
        .with(tracing_subscriber::fmt::layer().with_writer(std::io::stderr))
        .init();

    // Parse CLI arguments
    match parse_args()? {
        Command::Run => run_tui().await,
        Command::Import(path) => import_opml(&path).await,
        Command::Export(path) => export_opml(&path),
        Command::Help => {
            print_help();
            Ok(())
        }
        Command::Version => {
            print_version();
            Ok(())
        }
    }
}

/// CLI commands
enum Command {
    Run,
    Import(PathBuf),
    Export(PathBuf),
    Help,
    Version,
}

fn parse_args() -> Result<Command> {
    let args: Vec<String> = std::env::args().collect();

    if args.len() == 1 {
        return Ok(Command::Run);
    }

    match args[1].as_str() {
        "-h" | "--help" => Ok(Command::Help),
        "-v" | "--version" => Ok(Command::Version),
        "-i" | "--import" => {
            let path = args
                .get(2)
                .ok_or_else(|| color_eyre::eyre::eyre!("Missing OPML file path"))?;
            Ok(Command::Import(PathBuf::from(path)))
        }
        "-e" | "--export" => {
            let path = args
                .get(2)
                .ok_or_else(|| color_eyre::eyre::eyre!("Missing output file path"))?;
            Ok(Command::Export(PathBuf::from(path)))
        }
        other => Err(color_eyre::eyre::eyre!(
            "Unknown option: {other}\nRun 'feedo --help' for usage"
        )),
    }
}

fn print_help() {
    println!(
        r#"{}

A stunning terminal RSS reader — your news, your way.

USAGE:
    feedo [OPTIONS]

OPTIONS:
    -i, --import <FILE>    Import feeds from OPML file
    -e, --export <FILE>    Export feeds to OPML file
    -h, --help             Show this help message
    -v, --version          Show version information

KEYBINDINGS:
    Navigation
      j / ↓           Move down
      k / ↑           Move up  
      l / → / Enter   Select / Enter
      h / ←           Go back
      g / G           Jump to top / bottom
      Tab             Switch panel

    Actions  
      r               Refresh all feeds
      o               Open article in browser
      Space           Toggle read / unread
      a               Mark all as read
      /               Search across all feeds
      q / Esc         Quit

CONFIG:
    {config}

HOMEPAGE:
    https://github.com/ricardodantas/feedo
"#,
        feedo::ui::LOGO,
        config = Config::config_path()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "Unknown".to_string())
    );
}

fn print_version() {
    println!(
        r#"
      ██████╗██████╗██████╗██████╗  ██████╗
      ██╔═══╝██╔═══╝██╔═══╝██╔══██╗██╔═══██╗
      █████╗ █████╗ █████╗ ██║  ██║██║   ██║
      ██╔══╝ ██╔══╝ ██╔══╝ ██║  ██║██║   ██║
      ██║    ██████╗██████╗██████╔╝╚██████╔╝
      ╚═╝    ╚═════╝╚═════╝╚═════╝  ╚═════╝ 
                        
      v{}  (◕ᴥ◕)
"#,
        env!("CARGO_PKG_VERSION")
    );
}

async fn run_tui() -> Result<()> {
    let mut app = App::new().await?;
    app.run().await
}

async fn import_opml(path: &PathBuf) -> Result<()> {
    let mut config = Config::load()?;
    let count = feedo::opml::import(path, &mut config)?;
    config.save()?;
    println!("(◕ᴥ◕) Imported {count} feeds from {}", path.display());
    Ok(())
}

fn export_opml(path: &PathBuf) -> Result<()> {
    let config = Config::load()?;
    feedo::opml::export(&config, path)?;
    println!("(◕ᴥ◕) Exported feeds to {}", path.display());
    Ok(())
}
