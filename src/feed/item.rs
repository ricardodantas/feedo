//! Feed item data structure.

use chrono::{DateTime, Utc};

/// A single item/article from a feed.
#[derive(Debug, Clone)]
pub struct FeedItem {
    /// Article title.
    pub title: String,

    /// Article URL (if available).
    pub link: Option<String>,

    /// Publication date (if available).
    pub published: Option<DateTime<Utc>>,

    /// Summary or content (if available).
    pub summary: Option<String>,

    /// Whether the item has been read.
    pub read: bool,
}

impl FeedItem {
    /// Create a new feed item.
    #[must_use]
    pub fn new(title: String) -> Self {
        Self {
            title,
            link: None,
            published: None,
            summary: None,
            read: false,
        }
    }

    /// Set the item as read.
    pub fn mark_read(&mut self) {
        self.read = true;
    }

    /// Set the item as unread.
    pub fn mark_unread(&mut self) {
        self.read = false;
    }

    /// Toggle read state.
    pub fn toggle_read(&mut self) {
        self.read = !self.read;
    }
}
