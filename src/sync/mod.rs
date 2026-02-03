//! Sync support for external RSS services.
//!
//! This module provides integration with RSS sync services that implement
//! the Google Reader API, including:
//!
//! - [FreshRSS](https://freshrss.org/)
//! - [Miniflux](https://miniflux.app/)
//! - [Inoreader](https://www.inoreader.com/)
//! - [The Old Reader](https://theoldreader.com/)
//! - [BazQux](https://bazqux.com/)
//!
//! # Example
//!
//! ```ignore
//! use feedo::sync::{GReaderClient, SyncManager};
//!
//! // Low-level client
//! let client = GReaderClient::new("https://freshrss.example.com/api/greader.php");
//! let auth = client.login("username", "api_password").await?;
//! let subs = client.subscriptions(&auth).await?;
//!
//! // High-level sync manager
//! let manager = SyncManager::connect(server, user, pass).await?;
//! let result = manager.full_sync(&mut config, &mut cache).await?;
//! ```

mod client;
mod manager;
mod types;

pub use client::{GReaderClient, StreamOptions};
pub use manager::{SyncManager, SyncResult};
pub use types::*;
