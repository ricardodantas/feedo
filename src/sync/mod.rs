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
//! use feedo::sync::GReaderClient;
//!
//! let client = GReaderClient::new("https://freshrss.example.com/api/greader.php");
//! let auth = client.login("username", "api_password").await?;
//!
//! // Fetch subscriptions
//! let subs = client.subscriptions(&auth).await?;
//!
//! // Fetch unread items
//! let items = client.stream_contents(&auth, "user/-/state/com.google/reading-list", None).await?;
//!
//! // Mark item as read
//! client.edit_tag(&auth, &item_id, Some("user/-/state/com.google/read"), None).await?;
//! ```

mod client;
mod types;

pub use client::GReaderClient;
pub use types::*;
