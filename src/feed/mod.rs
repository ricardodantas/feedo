//! Feed management and fetching.
//!
//! This module handles:
//! - Fetching RSS/Atom feeds from the network
//! - Parsing feed content
//! - Managing feed state (read/unread)
//! - Auto-discovering feeds from URLs
//! - Offline caching of feed data

mod cache;
mod discovery;
mod item;
mod manager;
mod parser;

pub use cache::{CachedFeed, CachedItem, CacheStats, FeedCache};
pub use discovery::{DiscoveredFeed, FeedDiscovery, FeedType};
pub use item::FeedItem;
pub use manager::{Feed, FeedManager, Folder};
