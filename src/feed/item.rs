//! Feed item data structure.

use chrono::{DateTime, Utc};

/// A single item/article from a feed.
#[derive(Debug, Clone)]
pub struct FeedItem {
    /// Unique ID (for cache matching).
    pub id: String,

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
    /// Create a new feed item with auto-generated ID.
    #[must_use]
    pub fn new(title: String) -> Self {
        let id = Self::generate_id(None, &title);
        Self {
            id,
            title,
            link: None,
            published: None,
            summary: None,
            read: false,
        }
    }

    /// Create a new feed item with link (uses link for ID).
    #[must_use]
    pub fn with_link(title: String, link: Option<String>) -> Self {
        let id = Self::generate_id(link.as_deref(), &title);
        Self {
            id,
            title,
            link,
            published: None,
            summary: None,
            read: false,
        }
    }

    /// Generate a unique ID for an item.
    #[must_use]
    pub fn generate_id(link: Option<&str>, title: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        if let Some(link) = link {
            link.hash(&mut hasher);
        } else {
            title.hash(&mut hasher);
        }
        format!("{:x}", hasher.finish())
    }

    /// Set the item as read.
    pub const fn mark_read(&mut self) {
        self.read = true;
    }

    /// Set the item as unread.
    pub const fn mark_unread(&mut self) {
        self.read = false;
    }

    /// Toggle read state.
    pub const fn toggle_read(&mut self) {
        self.read = !self.read;
    }
}
