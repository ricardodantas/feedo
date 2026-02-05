//! Application entry point and CLI handling.

use std::path::{Path, PathBuf};

use color_eyre::Result;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

use feedo::{App, Config, GReaderClient, SyncConfig, SyncProvider};

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
        Command::Import(path) => import_opml(&path),
        Command::Export(path) => export_opml(&path),
        Command::Sync => sync_feeds().await,
        Command::SyncLogin {
            server,
            username,
            password,
            provider,
        } => sync_login(&server, &username, &password, provider).await,
        Command::SyncStatus => sync_status().await,
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
    Sync,
    SyncLogin {
        server: String,
        username: String,
        password: String,
        provider: SyncProvider,
    },
    SyncStatus,
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
        "sync" => {
            if args.len() > 2 {
                match args[2].as_str() {
                    "login" => {
                        // Parse: feedo sync login <server> <username> <password> [--provider freshrss|miniflux|greader]
                        let server = args.get(3)
                            .ok_or_else(|| color_eyre::eyre::eyre!("Missing server URL\nUsage: feedo sync login <server> <username> <password>"))?
                            .clone();
                        let username = args
                            .get(4)
                            .ok_or_else(|| color_eyre::eyre::eyre!("Missing username"))?
                            .clone();
                        let password = args
                            .get(5)
                            .ok_or_else(|| color_eyre::eyre::eyre!("Missing password"))?
                            .clone();

                        // Check for --provider flag
                        let mut provider = SyncProvider::GReader;
                        for (i, arg) in args.iter().enumerate() {
                            if arg == "--provider" {
                                if let Some(p) = args.get(i + 1) {
                                    provider = match p.to_lowercase().as_str() {
                                        "freshrss" => SyncProvider::FreshRSS,
                                        "miniflux" => SyncProvider::Miniflux,
                                        _ => SyncProvider::GReader,
                                    };
                                }
                            }
                        }

                        Ok(Command::SyncLogin {
                            server,
                            username,
                            password,
                            provider,
                        })
                    }
                    "status" => Ok(Command::SyncStatus),
                    _ => Ok(Command::Sync),
                }
            } else {
                Ok(Command::Sync)
            }
        }
        other => Err(color_eyre::eyre::eyre!(
            "Unknown option: {other}\nRun 'feedo --help' for usage"
        )),
    }
}

fn print_help() {
    let config_path =
        Config::config_path().map_or_else(|| "Unknown".to_string(), |p| p.display().to_string());

    println!(
        r"{}

A stunning terminal RSS reader — your news, your way.

USAGE:
    feedo [OPTIONS]
    feedo sync [COMMAND]

OPTIONS:
    -i, --import <FILE>    Import feeds from OPML file
    -e, --export <FILE>    Export feeds to OPML file
    -h, --help             Show this help message
    -v, --version          Show version information

SYNC COMMANDS:
    feedo sync                             Sync with configured server
    feedo sync login <server> <user> <pw>  Configure sync server
    feedo sync status                      Show sync configuration

    Supported providers: FreshRSS, Miniflux, Inoreader, The Old Reader

    Example:
      feedo sync login https://rss.example.com/api/greader.php user pass

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
      t               Change theme
      q / Esc         Quit

CONFIG:
    {config_path}

HOMEPAGE:
    https://github.com/ricardodantas/feedo
",
        feedo::ui::LOGO,
    );
}

fn print_version() {
    let version = env!("CARGO_PKG_VERSION");
    println!(
        r"
      ██████╗██████╗██████╗██████╗  ██████╗
      ██╔═══╝██╔═══╝██╔═══╝██╔══██╗██╔═══██╗
      █████╗ █████╗ █████╗ ██║  ██║██║   ██║
      ██╔══╝ ██╔══╝ ██╔══╝ ██║  ██║██║   ██║
      ██║    ██████╗██████╗██████╔╝╚██████╔╝
      ╚═╝    ╚═════╝╚═════╝╚═════╝  ╚═════╝ 
                        
      v{version}  (◕ᴥ◕)
"
    );
}

async fn run_tui() -> Result<()> {
    let mut app = App::new().await?;
    app.run().await
}

fn import_opml(path: &Path) -> Result<()> {
    let mut config = Config::load()?;
    let count = feedo::opml::import(path, &mut config)?;
    config.save()?;
    println!("(◕ᴥ◕) Imported {count} feeds from {}", path.display());
    Ok(())
}

fn export_opml(path: &Path) -> Result<()> {
    let config = Config::load()?;
    feedo::opml::export(&config, path)?;
    println!("(◕ᴥ◕) Exported feeds to {}", path.display());
    Ok(())
}

async fn sync_login(
    server: &str,
    username: &str,
    password: &str,
    provider: SyncProvider,
) -> Result<()> {
    println!("(◕ᴥ◕) Connecting to {server}...");

    // Test the connection
    let client = GReaderClient::new(server);
    let auth = client.login(username, password).await?;

    // Verify by fetching user info
    let user_info = client.user_info(&auth).await?;
    println!("✓ Logged in as: {}", user_info.user_name);

    // Fetch subscription count
    let subs = client.subscriptions(&auth).await?;
    println!("✓ Found {} subscriptions", subs.len());

    // Store password securely in system keychain
    let keychain_key = format!("{}@{}", username, server);
    match feedo::credentials::store_password(&keychain_key, password) {
        Ok(()) => println!("✓ Password stored securely in system keychain"),
        Err(e) => {
            println!("⚠ Could not store in keychain: {e}");
            println!("  Password will be stored in config file (less secure)");
        }
    }

    // Save to config (password only stored if keychain failed)
    let mut config = Config::load()?;
    let keychain_ok = feedo::credentials::get_password(&keychain_key).is_some();
    config.sync = Some(SyncConfig {
        provider,
        server: server.to_string(),
        username: username.to_string(),
        password: if keychain_ok { None } else { Some(password.to_string()) },
    });
    config.save()?;

    println!("\n(◕ᴥ◕) Sync configured! Run 'feedo sync' to sync your feeds.");
    Ok(())
}

/// Get sync password from keychain or config fallback.
fn get_sync_password(sync: &SyncConfig) -> Option<String> {
    // Try keychain first
    let keychain_key = format!("{}@{}", sync.username, sync.server);
    if let Some(password) = feedo::credentials::get_password(&keychain_key) {
        return Some(password);
    }
    // Fall back to config file
    sync.password.clone()
}

async fn sync_status() -> Result<()> {
    let config = Config::load()?;

    if let Some(sync) = &config.sync {
        println!("(◕ᴥ◕) Sync Configuration\n");
        println!("  Provider: {:?}", sync.provider);
        println!("  Server:   {}", sync.server);
        println!("  Username: {}", sync.username);
        
        let password = get_sync_password(sync);
        let keychain_key = format!("{}@{}", sync.username, sync.server);
        let in_keychain = feedo::credentials::get_password(&keychain_key).is_some();
        println!(
            "  Password: {}",
            if password.is_some() {
                if in_keychain { "**** (keychain)" } else { "**** (config file)" }
            } else {
                "(not set)"
            }
        );

        // Try to connect and show stats
        if let Some(password) = password {
            println!("\nTesting connection...");
            let client = GReaderClient::new(&sync.server);
            match client.login(&sync.username, &password).await {
                Ok(auth) => {
                    println!("✓ Connection successful");
                    if let Ok(subs) = client.subscriptions(&auth).await {
                        println!("✓ {} subscriptions on server", subs.len());
                    }
                    if let Ok(unread) = client.unread_count(&auth).await {
                        let total: i64 = unread.unreadcounts.iter().map(|u| u.count).sum();
                        println!("✓ {total} unread items");
                    }
                }
                Err(e) => println!("✗ Connection failed: {e}"),
            }
        }
    } else {
        println!("(◕ᴥ◕) No sync configured\n");
        println!("To configure sync, run:");
        println!("  feedo sync login <server> <username> <password>");
        println!("\nSupported services:");
        println!("  • FreshRSS: https://your-server/api/greader.php");
        println!("  • Miniflux: https://your-server/v1/");
        println!("  • Inoreader: https://www.inoreader.com");
        println!("  • The Old Reader: https://theoldreader.com");
    }

    Ok(())
}

async fn sync_feeds() -> Result<()> {
    let mut config = Config::load()?;

    let sync = config.sync.clone().ok_or_else(|| {
        color_eyre::eyre::eyre!("No sync configured. Run 'feedo sync login' first.")
    })?;

    let password = get_sync_password(&sync).ok_or_else(|| {
        color_eyre::eyre::eyre!("No password stored. Run 'feedo sync login' again.")
    })?;

    println!("(◕ᴥ◕) Syncing with {}...\n", sync.server);

    // Load cache
    let mut cache = feedo::FeedCache::load()?;

    // Connect and run full sync
    let manager = feedo::SyncManager::connect(&sync.server, &sync.username, &password).await?;
    let result = manager.full_sync(&mut config, &mut cache).await?;

    // Save changes
    config.save()?;
    cache.save()?;

    // Print results
    println!(
        "✓ Imported {} new feeds ({} already existed)",
        result.feeds_imported, result.feeds_existing
    );
    println!(
        "✓ Marked {} items as read (from server)",
        result.items_marked_read
    );
    println!(
        "✓ Synced {} items to server (from local)",
        result.items_synced_to_server
    );

    if !result.errors.is_empty() {
        println!("\n⚠ {} warnings:", result.errors.len());
        for err in &result.errors[..result.errors.len().min(5)] {
            println!("  • {err}");
        }
        if result.errors.len() > 5 {
            println!("  ... and {} more", result.errors.len() - 5);
        }
    }

    println!("\n(◕ᴥ◕) Sync complete!");
    Ok(())
}
