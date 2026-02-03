//! Offline cache for feed articles.
//!
//! This module provides persistent storage for feed data,
//! allowing the app to work offline and preserve read states.

use std::{collections::HashMap, fs, path::PathBuf};

use chrono::{DateTime, Utc};
use color_eyre::Result;
use serde::{Deserialize, Serialize};
use tracing::{debug, warn};

use crate::config::Config;

/// Cached feed data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedFeed {
    /// Feed URL (used as key).
    pub url: String,

    /// Feed name.
    pub name: String,

    /// Cached items.
    pub items: Vec<CachedItem>,

    /// Last successful fetch time.
    pub last_fetched: Option<DateTime<Utc>>,

    /// Last fetch error (if any).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
}

/// Cached item data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedItem {
    /// Unique ID (generated from link or title hash).
    pub id: String,

    /// Article title.
    pub title: String,

    /// Article URL.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub link: Option<String>,

    /// Publication date.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub published: Option<DateTime<Utc>>,

    /// Summary or content.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,

    /// Whether the item has been read.
    #[serde(default)]
    pub read: bool,

    /// When this item was first cached.
    pub cached_at: DateTime<Utc>,
}

impl CachedItem {
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
}

/// Feed cache manager.
#[derive(Debug, Default)]
pub struct FeedCache {
    /// Cached feeds by URL.
    feeds: HashMap<String, CachedFeed>,

    /// Whether cache has been modified.
    dirty: bool,
}

impl FeedCache {
    /// Load cache from disk.
    ///
    /// # Errors
    ///
    /// Returns an error if the cache file exists but cannot be read or parsed.
    pub fn load() -> Result<Self> {
        let path = Self::cache_path()?;

        if !path.exists() {
            debug!("No cache file found, starting fresh");
            return Ok(Self::default());
        }

        let content = fs::read_to_string(&path)?;
        let feeds: HashMap<String, CachedFeed> = serde_json::from_str(&content)?;

        debug!("Loaded {} feeds from cache", feeds.len());

        Ok(Self {
            feeds,
            dirty: false,
        })
    }

    /// Save cache to disk.
    ///
    /// # Errors
    ///
    /// Returns an error if the cache file cannot be written.
    pub fn save(&mut self) -> Result<()> {
        if !self.dirty {
            return Ok(());
        }

        let path = Self::cache_path()?;

        // Ensure directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let content = serde_json::to_string_pretty(&self.feeds)?;
        fs::write(&path, content)?;

        self.dirty = false;
        debug!("Saved {} feeds to cache", self.feeds.len());

        Ok(())
    }

    /// Get the cache file path.
    fn cache_path() -> Result<PathBuf> {
        Config::data_dir()
            .map(|dir| dir.join("cache.json"))
            .ok_or_else(|| color_eyre::eyre::eyre!("Could not determine cache directory"))
    }

    /// Get cached feed by URL.
    #[must_use]
    pub fn get(&self, url: &str) -> Option<&CachedFeed> {
        self.feeds.get(url)
    }

    /// Update cache for a feed.
    pub fn update_feed(
        &mut self,
        url: &str,
        name: &str,
        items: Vec<CachedItem>,
        error: Option<String>,
    ) {
        let now = Utc::now();

        let cached = self
            .feeds
            .entry(url.to_string())
            .or_insert_with(|| CachedFeed {
                url: url.to_string(),
                name: name.to_string(),
                items: Vec::new(),
                last_fetched: None,
                last_error: None,
            });

        cached.name = name.to_string();
        cached.last_error = error;

        if cached.last_error.is_none() {
            cached.last_fetched = Some(now);

            // Merge items, preserving read state
            let old_states: HashMap<String, bool> = cached
                .items
                .iter()
                .map(|i| (i.id.clone(), i.read))
                .collect();

            cached.items = items
                .into_iter()
                .map(|mut item| {
                    // Restore read state from old cache
                    if let Some(&was_read) = old_states.get(&item.id) {
                        item.read = was_read;
                    }
                    item
                })
                .collect();
        }

        self.dirty = true;
    }

    /// Mark an item as read/unread.
    pub fn set_item_read(&mut self, feed_url: &str, item_id: &str, read: bool) {
        if let Some(feed) = self.feeds.get_mut(feed_url) {
            if let Some(item) = feed.items.iter_mut().find(|i| i.id == item_id) {
                if item.read != read {
                    item.read = read;
                    self.dirty = true;
                }
            }
        }
    }

    /// Mark all items in a feed as read.
    pub fn mark_feed_read(&mut self, feed_url: &str) {
        if let Some(feed) = self.feeds.get_mut(feed_url) {
            for item in &mut feed.items {
                if !item.read {
                    item.read = true;
                    self.dirty = true;
                }
            }
        }
    }

    /// Remove a feed from cache.
    pub fn remove_feed(&mut self, url: &str) {
        if self.feeds.remove(url).is_some() {
            self.dirty = true;
        }
    }

    /// Get cache statistics.
    #[must_use]
    pub fn stats(&self) -> CacheStats {
        let total_feeds = self.feeds.len();
        let total_items: usize = self.feeds.values().map(|f| f.items.len()).sum();
        let unread_items: usize = self
            .feeds
            .values()
            .flat_map(|f| f.items.iter())
            .filter(|i| !i.read)
            .count();

        let oldest_fetch = self.feeds.values().filter_map(|f| f.last_fetched).min();

        CacheStats {
            total_feeds,
            total_items,
            unread_items,
            oldest_fetch,
        }
    }

    /// Prune old items beyond a limit per feed.
    pub fn prune(&mut self, max_items_per_feed: usize) {
        for feed in self.feeds.values_mut() {
            if feed.items.len() > max_items_per_feed {
                // Keep newest items, but always keep unread items
                feed.items.sort_by(|a, b| {
                    // Unread items first, then by date descending
                    match (a.read, b.read) {
                        (false, true) => std::cmp::Ordering::Less,
                        (true, false) => std::cmp::Ordering::Greater,
                        _ => b.cached_at.cmp(&a.cached_at),
                    }
                });

                let old_len = feed.items.len();
                feed.items.truncate(max_items_per_feed);

                if feed.items.len() < old_len {
                    self.dirty = true;
                    debug!(
                        "Pruned {} items from {}",
                        old_len - feed.items.len(),
                        feed.name
                    );
                }
            }
        }
    }
}

/// Cache statistics.
#[derive(Debug, Clone)]
pub struct CacheStats {
    /// Total number of cached feeds.
    pub total_feeds: usize,
    /// Total number of cached items.
    pub total_items: usize,
    /// Number of unread items.
    pub unread_items: usize,
    /// Oldest fetch time.
    pub oldest_fetch: Option<DateTime<Utc>>,
}

impl Drop for FeedCache {
    fn drop(&mut self) {
        if self.dirty {
            if let Err(e) = self.save() {
                warn!("Failed to save cache on drop: {e}");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_id() {
        let id1 = CachedItem::generate_id(Some("https://example.com/1"), "Title");
        let id2 = CachedItem::generate_id(Some("https://example.com/2"), "Title");
        let id3 = CachedItem::generate_id(None, "Title");

        assert_ne!(id1, id2);
        assert_ne!(id1, id3);
    }

    #[test]
    fn test_cache_stats() {
        let cache = FeedCache::default();
        let stats = cache.stats();

        assert_eq!(stats.total_feeds, 0);
        assert_eq!(stats.total_items, 0);
    }
}
