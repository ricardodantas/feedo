//! # Feedo 🐕
//!
//! A stunning terminal RSS reader built with Rust.
//!
//! ## Overview
//!
//! Feedo is a modern, fast, and beautiful terminal-based RSS/Atom feed reader.
//! It provides a three-panel interface inspired by desktop readers like Reeder,
//! but designed for the command line.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                          App                                │
//! │  Orchestrates all components and runs the main event loop   │
//! └─────────────────────────────────────────────────────────────┘
//!                              │
//!          ┌───────────────────┼───────────────────┐
//!          ▼                   ▼                   ▼
//! ┌─────────────────┐ ┌─────────────────┐ ┌─────────────────┐
//! │     Config      │ │  FeedManager    │ │       UI        │
//! │                 │ │                 │ │                 │
//! │ • Load/Save     │ │ • Fetch feeds   │ │ • Render panels │
//! │ • Folders       │ │ • Parse RSS     │ │ • Handle input  │
//! │ • Theme         │ │ • Track state   │ │ • Search        │
//! └─────────────────┘ └─────────────────┘ └─────────────────┘
//!          │                   │                   │
//!          └───────────────────┴───────────────────┘
//!                              │
//!                    ┌─────────────────┐
//!                    │      OPML       │
//!                    │                 │
//!                    │ • Import feeds  │
//!                    │ • Export feeds  │
//!                    └─────────────────┘
//! ```
//!
//! ## Modules
//!
//! - [`app`] — Main application state and event loop
//! - [`config`] — Configuration management and persistence
//! - [`feed`] — Feed fetching, parsing, and state management
//! - [`opml`] — OPML import/export for feed migration
//! - [`sync`] — Sync with `FreshRSS`, `Miniflux` via Google Reader API
//! - [`ui`] — Terminal UI rendering and input handling
//!
//! Theme support is provided by the [`ratatui_themes`](https://crates.io/crates/ratatui-themes) crate.
//!
//! ## Example
//!
//! ```no_run
//! use feedo::App;
//!
//! #[tokio::main]
//! async fn main() -> color_eyre::Result<()> {
//!     let mut app = App::new().await?;
//!     app.run().await
//! }
//! ```
//!
//! ## Features
//!
//! - **Beautiful TUI** — Clean three-panel interface with ratatui
//! - **Folder Organization** — Group feeds with custom emoji icons
//! - **Instant Search** — Find articles across all feeds
//! - **15 Themes** — Dracula, Nord, Catppuccin, Gruvbox, Tokyo Night, and more
//! - **OPML Support** — Import/export for easy migration
//! - **Social Sharing** — Share articles to X, Mastodon, and Bluesky
//! - **Async** — Non-blocking feed fetching with Tokio
//! - **Cross-Platform** — Works on Linux, macOS, and Windows

#![doc(html_root_url = "https://docs.rs/feedo/0.1.0")]
#![warn(missing_docs)]
#![warn(rustdoc::missing_crate_level_docs)]
// Allow some clippy lints for now - to be addressed in future cleanup
#![allow(clippy::unused_async)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::field_reassign_with_default)]
#![allow(clippy::single_match_else)]
#![allow(clippy::uninlined_format_args)]
#![allow(clippy::match_wildcard_for_single_variants)]
#![allow(clippy::redundant_closure_for_method_calls)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::redundant_clone)]
#![allow(clippy::manual_let_else)]
#![allow(clippy::option_if_let_else)]
#![allow(clippy::if_not_else)]
#![allow(clippy::struct_excessive_bools)]
#![allow(clippy::unnecessary_map_or)]
#![allow(clippy::too_many_lines)]
#![allow(clippy::missing_const_for_fn)]

pub mod app;
pub mod config;
pub mod credentials;
pub mod error_report;
pub mod feed;
pub mod opml;
pub mod sync;
pub mod ui;
pub mod update;

// Re-export main types for convenience
pub use app::App;
pub use config::Config;
pub use error_report::{REPO_URL, VERSION, create_issue_url, open_issue};
pub use feed::{
    CacheStats, CachedFeed, CachedItem, DiscoveredFeed, Feed, FeedCache, FeedDiscovery, FeedItem,
    FeedManager, FeedType,
};
pub use sync::{GReaderClient, SyncConfig, SyncManager, SyncProvider, SyncResult};
pub use update::{
    PackageManager, VersionCheck, check_for_updates, check_for_updates_crates_io,
    check_for_updates_timeout, detect_package_manager, run_update,
};

// Re-export theme types from ratatui-themes crate
pub use ratatui_themes::{Theme, ThemeName, ThemePalette};
