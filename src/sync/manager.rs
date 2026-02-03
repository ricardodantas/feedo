//! Sync manager for bidirectional sync with Google Reader API servers.

#![allow(clippy::missing_errors_doc)]
#![allow(clippy::uninlined_format_args)]
#![allow(clippy::redundant_closure_for_method_calls)]

use std::collections::{HashMap, HashSet};

use color_eyre::Result;
use tracing::{debug, info};

use crate::config::{Config, FeedConfig, FolderConfig};
use crate::feed::FeedCache;
use crate::sync::{AuthToken, GReaderClient, StreamOptions, streams};

/// Result of a sync operation.
#[derive(Debug, Default)]
pub struct SyncResult {
    /// Number of feeds imported from server.
    pub feeds_imported: usize,
    /// Number of feeds already present locally.
    pub feeds_existing: usize,
    /// Number of items marked as read locally (from server).
    pub items_marked_read: usize,
    /// Number of items marked as read on server (from local).
    pub items_synced_to_server: usize,
    /// Errors encountered (non-fatal).
    pub errors: Vec<String>,
}

/// Sync manager for bidirectional sync.
pub struct SyncManager {
    client: GReaderClient,
    auth: AuthToken,
}

impl SyncManager {
    /// Create a new sync manager.
    pub async fn connect(server: &str, username: &str, password: &str) -> Result<Self> {
        let client = GReaderClient::new(server);
        let auth = client.login(username, password).await?;
        Ok(Self { client, auth })
    }

    /// Import subscriptions from server to local config.
    pub async fn import_subscriptions(&self, config: &mut Config) -> Result<SyncResult> {
        let mut result = SyncResult::default();

        let subs = self.client.subscriptions(&self.auth).await?;
        info!("Fetched {} subscriptions from server", subs.len());

        // Get existing feed URLs
        let existing_urls: HashSet<String> = config
            .folders
            .iter()
            .flat_map(|f| f.feeds.iter().map(|feed| feed.url.clone()))
            .chain(config.feeds.iter().map(|f| f.url.clone()))
            .collect();

        // Group subscriptions by category
        let mut by_category: HashMap<String, Vec<(String, String)>> = HashMap::new();
        let mut root_feeds: Vec<(String, String)> = Vec::new();

        for sub in &subs {
            let feed_url = sub.url.clone();
            let feed_name = sub.title.clone();

            if existing_urls.contains(&feed_url) {
                result.feeds_existing += 1;
                continue;
            }

            if let Some(cat) = sub.categories.first() {
                by_category
                    .entry(cat.label.clone())
                    .or_default()
                    .push((feed_url, feed_name));
            } else {
                root_feeds.push((feed_url, feed_name));
            }
        }

        // Add to folders
        for (category, feeds) in by_category {
            // Find or create folder
            let folder = config
                .folders
                .iter_mut()
                .find(|f| f.name.eq_ignore_ascii_case(&category));

            if let Some(folder) = folder {
                for (url, name) in feeds {
                    folder.feeds.push(FeedConfig { name, url });
                    result.feeds_imported += 1;
                }
            } else {
                let new_feeds: Vec<FeedConfig> = feeds
                    .into_iter()
                    .map(|(url, name)| {
                        result.feeds_imported += 1;
                        FeedConfig { name, url }
                    })
                    .collect();

                config.folders.push(FolderConfig {
                    name: category,
                    icon: None,
                    expanded: true,
                    feeds: new_feeds,
                });
            }
        }

        // Add root feeds
        for (url, name) in root_feeds {
            config.feeds.push(FeedConfig { name, url });
            result.feeds_imported += 1;
        }

        info!(
            "Imported {} feeds, {} already existed",
            result.feeds_imported, result.feeds_existing
        );
        Ok(result)
    }

    /// Sync read states from server to local cache.
    pub async fn sync_read_states_from_server(&self, cache: &mut FeedCache) -> Result<SyncResult> {
        let mut result = SyncResult::default();

        // Fetch all read item IDs from server
        let read_items = self
            .client
            .stream_item_ids(
                &self.auth,
                streams::READ,
                Some(StreamOptions::with_count(10000)),
            )
            .await?;

        info!("Server has {} read items", read_items.item_refs.len());

        // Build a set of read item IDs (in decimal form)
        let read_ids: HashSet<String> = read_items.item_refs.iter().map(|r| r.id.clone()).collect();

        // Get subscriptions to map feed IDs to URLs
        let subs = self.client.subscriptions(&self.auth).await?;
        let _feed_id_to_url: HashMap<String, String> =
            subs.iter().map(|s| (s.id.clone(), s.url.clone())).collect();

        // For each subscription, fetch items and update local read state
        for sub in &subs {
            let items = match self
                .client
                .stream_contents(&self.auth, &sub.id, Some(StreamOptions::with_count(100)))
                .await
            {
                Ok(items) => items,
                Err(e) => {
                    result
                        .errors
                        .push(format!("Failed to fetch {}: {}", sub.title, e));
                    continue;
                }
            };

            for item in &items.items {
                // Check if this item is read on server
                if let Some(decimal_id) = item.id_decimal() {
                    let id_str = decimal_id.to_string();
                    if read_ids.contains(&id_str) {
                        // Mark as read locally
                        // We need to find the local item ID
                        if let Some(link) = item.link() {
                            let local_id = crate::feed::CachedItem::generate_id(
                                Some(link),
                                item.title.as_deref().unwrap_or(""),
                            );
                            cache.set_item_read(&sub.url, &local_id, true);
                            result.items_marked_read += 1;
                        }
                    }
                }
            }
        }

        info!(
            "Marked {} items as read from server",
            result.items_marked_read
        );
        Ok(result)
    }

    /// Sync local read states to server.
    pub async fn sync_read_states_to_server(
        &self,
        cache: &FeedCache,
        config: &Config,
    ) -> Result<SyncResult> {
        let mut result = SyncResult::default();

        // Get all feed URLs from config
        let feed_urls: Vec<String> = config
            .folders
            .iter()
            .flat_map(|f| f.feeds.iter().map(|feed| feed.url.clone()))
            .chain(config.feeds.iter().map(|f| f.url.clone()))
            .collect();

        // Get subscriptions to map URLs to feed IDs
        let subs = self.client.subscriptions(&self.auth).await?;
        let url_to_feed_id: HashMap<String, String> =
            subs.iter().map(|s| (s.url.clone(), s.id.clone())).collect();

        // For each local feed, sync read items
        for feed_url in &feed_urls {
            let Some(cached_feed) = cache.get(feed_url) else {
                continue;
            };

            let Some(feed_id) = url_to_feed_id.get(feed_url) else {
                debug!("Feed {} not found on server, skipping", feed_url);
                continue;
            };

            // Get items from server for this feed
            let server_items = match self
                .client
                .stream_contents(&self.auth, feed_id, Some(StreamOptions::with_count(100)))
                .await
            {
                Ok(items) => items,
                Err(e) => {
                    result
                        .errors
                        .push(format!("Failed to fetch {}: {}", feed_url, e));
                    continue;
                }
            };

            // Find items that are read locally but not on server
            let mut to_mark_read: Vec<String> = Vec::new();

            for server_item in &server_items.items {
                if server_item.is_read() {
                    continue; // Already read on server
                }

                // Check if read locally
                if let Some(link) = server_item.link() {
                    let local_id = crate::feed::CachedItem::generate_id(
                        Some(link),
                        server_item.title.as_deref().unwrap_or(""),
                    );

                    if let Some(local_item) = cached_feed.items.iter().find(|i| i.id == local_id) {
                        if local_item.read {
                            to_mark_read.push(server_item.id.clone());
                        }
                    }
                }
            }

            // Mark items as read on server
            if !to_mark_read.is_empty() {
                let ids: Vec<&str> = to_mark_read.iter().map(|s| s.as_str()).collect();
                match self.client.mark_read(&self.auth, &ids).await {
                    Ok(()) => {
                        result.items_synced_to_server += to_mark_read.len();
                        info!(
                            "Marked {} items as read on server for {}",
                            to_mark_read.len(),
                            feed_url
                        );
                    }
                    Err(e) => {
                        result
                            .errors
                            .push(format!("Failed to mark read on server: {}", e));
                    }
                }
            }
        }

        info!("Synced {} items to server", result.items_synced_to_server);
        Ok(result)
    }

    /// Full bidirectional sync.
    pub async fn full_sync(
        &self,
        config: &mut Config,
        cache: &mut FeedCache,
    ) -> Result<SyncResult> {
        let mut result = SyncResult::default();

        // 1. Import subscriptions from server
        info!("Step 1: Importing subscriptions from server...");
        let import_result = self.import_subscriptions(config).await?;
        result.feeds_imported = import_result.feeds_imported;
        result.feeds_existing = import_result.feeds_existing;
        result.errors.extend(import_result.errors);

        // 2. Sync read states from server to local
        info!("Step 2: Syncing read states from server...");
        let from_server = self.sync_read_states_from_server(cache).await?;
        result.items_marked_read = from_server.items_marked_read;
        result.errors.extend(from_server.errors);

        // 3. Sync local read states to server
        info!("Step 3: Syncing read states to server...");
        let to_server = self.sync_read_states_to_server(cache, config).await?;
        result.items_synced_to_server = to_server.items_synced_to_server;
        result.errors.extend(to_server.errors);

        Ok(result)
    }
}
