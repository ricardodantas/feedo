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
//! - [`theme`] — UI theming with 15 popular color schemes
//! - [`ui`] — Terminal UI rendering and input handling
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

pub mod app;
pub mod config;
pub mod error_report;
pub mod feed;
pub mod opml;
pub mod sync;
pub mod theme;
pub mod ui;

// Re-export main types for convenience
pub use app::App;
pub use config::Config;
pub use error_report::{REPO_URL, VERSION, create_issue_url, open_issue};
pub use feed::{
    CacheStats, CachedFeed, CachedItem, DiscoveredFeed, Feed, FeedCache, FeedDiscovery, FeedItem,
    FeedManager, FeedType,
};
pub use sync::{GReaderClient, SyncConfig, SyncManager, SyncProvider, SyncResult};
pub use theme::{Theme, ThemeName, ThemePalette};
