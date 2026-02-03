//! Feed management and fetching.
//!
//! This module handles:
//! - Fetching RSS/Atom feeds from the network
//! - Parsing feed content
//! - Managing feed state (read/unread)

mod item;
mod manager;
mod parser;

pub use item::FeedItem;
pub use manager::{Feed, FeedManager, Folder};
