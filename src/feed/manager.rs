//! Feed manager - handles feed state and fetching.

use chrono::{DateTime, Utc};
use color_eyre::Result;
use tracing::{debug, warn};

use super::{parser, FeedItem};
use crate::config::Config;

/// A single feed with its items.
#[derive(Debug, Clone)]
pub struct Feed {
    /// Display name.
    pub name: String,

    /// Feed URL.
    pub url: String,

    /// Fetched items.
    pub items: Vec<FeedItem>,

    /// Last successful update time.
    pub last_updated: Option<DateTime<Utc>>,

    /// Last error message (if any).
    pub error: Option<String>,
}

impl Feed {
    /// Create a new feed.
    #[must_use]
    pub fn new(name: String, url: String) -> Self {
        Self {
            name,
            url,
            items: Vec::new(),
            last_updated: None,
            error: None,
        }
    }

    /// Count unread items.
    #[must_use]
    pub fn unread_count(&self) -> usize {
        self.items.iter().filter(|i| !i.read).count()
    }

    /// Mark all items as read.
    pub fn mark_all_read(&mut self) {
        for item in &mut self.items {
            item.read = true;
        }
    }
}

/// A folder containing feeds.
#[derive(Debug, Clone)]
pub struct Folder {
    /// Display name.
    pub name: String,

    /// Optional emoji icon.
    pub icon: Option<String>,

    /// Whether expanded in UI.
    pub expanded: bool,

    /// Indices of feeds in this folder (into `FeedManager.feeds`).
    pub feed_indices: Vec<usize>,
}

impl Folder {
    /// Create a new folder.
    #[must_use]
    pub fn new(name: String, icon: Option<String>, expanded: bool) -> Self {
        Self {
            name,
            icon,
            expanded,
            feed_indices: Vec::new(),
        }
    }
}

/// Manages all feeds and folders.
pub struct FeedManager {
    /// All feeds (flat list).
    pub feeds: Vec<Feed>,

    /// Folder structure.
    pub folders: Vec<Folder>,

    /// HTTP client for fetching.
    client: reqwest::Client,
}

impl FeedManager {
    /// Create a new feed manager from configuration.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP client cannot be created.
    pub fn new(config: &Config) -> Result<Self> {
        let client = reqwest::Client::builder()
            .user_agent(concat!("feedo/", env!("CARGO_PKG_VERSION")))
            .timeout(std::time::Duration::from_secs(30))
            .build()?;

        let mut feeds: Vec<Feed> = Vec::new();
        let mut folders: Vec<Folder> = Vec::new();

        // Process folders
        for folder_config in &config.folders {
            let mut folder = Folder::new(
                folder_config.name.clone(),
                folder_config.icon.clone(),
                folder_config.expanded,
            );

            for feed_config in &folder_config.feeds {
                let feed_idx = feeds.len();
                feeds.push(Feed::new(feed_config.name.clone(), feed_config.url.clone()));
                folder.feed_indices.push(feed_idx);
            }

            folders.push(folder);
        }

        // Process root-level feeds
        for feed_config in &config.feeds {
            feeds.push(Feed::new(feed_config.name.clone(), feed_config.url.clone()));
        }

        Ok(Self {
            feeds,
            folders,
            client,
        })
    }

    /// Refresh all feeds.
    pub async fn refresh_all(&mut self) {
        for i in 0..self.feeds.len() {
            self.refresh_feed(i).await;
        }
    }

    /// Refresh a single feed by index.
    pub async fn refresh_feed(&mut self, index: usize) {
        let Some(feed) = self.feeds.get(index) else {
            return;
        };

        let url = feed.url.clone();
        let name = feed.name.clone();

        debug!("Fetching feed: {name} ({url})");

        match self.fetch_feed(&url).await {
            Ok(items) => {
                if let Some(feed) = self.feeds.get_mut(index) {
                    feed.items = items;
                    feed.last_updated = Some(Utc::now());
                    feed.error = None;
                    debug!("Fetched {} items from {name}", feed.items.len());
                }
            }
            Err(e) => {
                warn!("Failed to fetch {name}: {e}");
                if let Some(feed) = self.feeds.get_mut(index) {
                    feed.error = Some(e.to_string());
                }
            }
        }
    }

    /// Fetch and parse a feed from URL.
    async fn fetch_feed(&self, url: &str) -> Result<Vec<FeedItem>> {
        let response = self.client.get(url).send().await?;
        let bytes = response.bytes().await?;
        parser::parse_feed(&bytes)
    }

    /// Toggle folder expansion.
    pub fn toggle_folder(&mut self, index: usize) {
        if let Some(folder) = self.folders.get_mut(index) {
            folder.expanded = !folder.expanded;
        }
    }

    /// Get unread count for a folder.
    #[must_use]
    pub fn folder_unread_count(&self, folder_index: usize) -> usize {
        self.folders
            .get(folder_index)
            .map(|folder| {
                folder
                    .feed_indices
                    .iter()
                    .filter_map(|&idx| self.feeds.get(idx))
                    .map(Feed::unread_count)
                    .sum()
            })
            .unwrap_or(0)
    }

    /// Get total unread count.
    #[must_use]
    pub fn total_unread_count(&self) -> usize {
        self.feeds.iter().map(Feed::unread_count).sum()
    }

    /// Get indices of root-level feeds (not in any folder).
    #[must_use]
    pub fn root_feed_indices(&self) -> Vec<usize> {
        let folder_feeds: std::collections::HashSet<usize> = self
            .folders
            .iter()
            .flat_map(|f| f.feed_indices.iter().copied())
            .collect();

        (0..self.feeds.len())
            .filter(|i| !folder_feeds.contains(i))
            .collect()
    }
}
