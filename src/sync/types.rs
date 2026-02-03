//! Google Reader API type definitions.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Authentication token from login.
#[derive(Debug, Clone)]
pub struct AuthToken {
    /// The auth token string.
    pub token: String,
}

/// User information.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserInfo {
    /// User ID.
    pub user_id: String,
    /// Username.
    pub user_name: String,
    /// User email.
    pub user_email: Option<String>,
    /// Profile ID.
    pub user_profile_id: Option<String>,
}

/// A subscription (feed).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Subscription {
    /// Feed ID (format: "feed/{id}").
    pub id: String,
    /// Feed title.
    pub title: String,
    /// Feed URL.
    pub url: String,
    /// Website URL.
    #[serde(default)]
    pub html_url: Option<String>,
    /// Favicon URL.
    #[serde(default)]
    pub icon_url: Option<String>,
    /// Categories/folders.
    #[serde(default)]
    pub categories: Vec<Category>,
}

/// A category/folder/tag.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Category {
    /// Category ID (format: "user/-/label/{name}").
    pub id: String,
    /// Display label.
    pub label: String,
    /// Type (e.g., "folder").
    #[serde(rename = "type", default)]
    pub category_type: Option<String>,
}

/// Response from subscription/list endpoint.
#[derive(Debug, Deserialize)]
pub struct SubscriptionList {
    /// List of subscriptions.
    pub subscriptions: Vec<Subscription>,
}

/// Response from tag/list endpoint.
#[derive(Debug, Deserialize)]
pub struct TagList {
    /// List of tags.
    pub tags: Vec<Tag>,
}

/// A tag (label, state, or folder).
#[derive(Debug, Clone, Deserialize)]
pub struct Tag {
    /// Tag ID.
    pub id: String,
    /// Sort ID (optional).
    #[serde(rename = "sortid", default)]
    pub sort_id: Option<String>,
}

/// Response from unread-count endpoint.
#[derive(Debug, Deserialize)]
pub struct UnreadCount {
    /// Maximum count.
    pub max: i64,
    /// Unread counts per feed/category.
    #[serde(default)]
    pub unreadcounts: Vec<UnreadCountItem>,
}

/// Unread count for a single feed or category.
#[derive(Debug, Deserialize)]
pub struct UnreadCountItem {
    /// Feed or category ID.
    pub id: String,
    /// Number of unread items.
    pub count: i64,
    /// Newest item timestamp (microseconds).
    #[serde(rename = "newestItemTimestampUsec", default)]
    pub newest_item_timestamp_usec: Option<String>,
}

/// Response from stream/contents endpoint.
#[derive(Debug, Deserialize)]
pub struct StreamContents {
    /// Stream ID.
    pub id: String,
    /// Stream title.
    #[serde(default)]
    pub title: Option<String>,
    /// Last updated timestamp.
    #[serde(default)]
    pub updated: Option<i64>,
    /// Continuation token for pagination.
    #[serde(default)]
    pub continuation: Option<String>,
    /// Items in the stream.
    #[serde(default)]
    pub items: Vec<StreamItem>,
}

/// A single item from a stream.
#[derive(Debug, Clone, Deserialize)]
pub struct StreamItem {
    /// Item ID (long form: "tag:google.com,2005:reader/item/{hex}").
    pub id: String,
    /// Feed ID.
    #[serde(default)]
    pub origin: Option<StreamItemOrigin>,
    /// Item title.
    #[serde(default)]
    pub title: Option<String>,
    /// Author name.
    #[serde(default)]
    pub author: Option<String>,
    /// Published timestamp (seconds).
    #[serde(default)]
    pub published: Option<i64>,
    /// Updated timestamp (seconds).
    #[serde(default)]
    pub updated: Option<i64>,
    /// Crawled timestamp (microseconds).
    #[serde(rename = "crawlTimeMsec", default)]
    pub crawl_time_msec: Option<String>,
    /// Timestamp in microseconds.
    #[serde(rename = "timestampUsec", default)]
    pub timestamp_usec: Option<String>,
    /// Categories/tags applied to this item.
    #[serde(default)]
    pub categories: Vec<String>,
    /// Canonical URL.
    #[serde(default)]
    pub canonical: Option<Vec<StreamItemLink>>,
    /// Alternate URLs.
    #[serde(default)]
    pub alternate: Option<Vec<StreamItemLink>>,
    /// Summary content.
    #[serde(default)]
    pub summary: Option<StreamItemContent>,
    /// Full content.
    #[serde(default)]
    pub content: Option<StreamItemContent>,
}

impl StreamItem {
    /// Check if item is read.
    pub fn is_read(&self) -> bool {
        self.categories
            .iter()
            .any(|c| c.ends_with("/state/com.google/read"))
    }

    /// Check if item is starred.
    pub fn is_starred(&self) -> bool {
        self.categories
            .iter()
            .any(|c| c.ends_with("/state/com.google/starred"))
    }

    /// Get the item's link URL.
    pub fn link(&self) -> Option<&str> {
        self.canonical
            .as_ref()
            .and_then(|links| links.first())
            .or_else(|| self.alternate.as_ref().and_then(|links| links.first()))
            .map(|l| l.href.as_str())
    }

    /// Get content (prefers full content over summary).
    pub fn get_content(&self) -> Option<&str> {
        self.content
            .as_ref()
            .or(self.summary.as_ref())
            .map(|c| c.content.as_str())
    }

    /// Get published datetime.
    pub fn published_at(&self) -> Option<DateTime<Utc>> {
        self.published
            .and_then(|ts| DateTime::from_timestamp(ts, 0))
    }

    /// Parse item ID to decimal.
    pub fn id_decimal(&self) -> Option<i64> {
        parse_item_id(&self.id)
    }
}

/// Origin (feed) of a stream item.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamItemOrigin {
    /// Feed ID.
    pub stream_id: String,
    /// Feed title.
    #[serde(default)]
    pub title: Option<String>,
    /// Feed HTML URL.
    #[serde(default)]
    pub html_url: Option<String>,
}

/// A link in a stream item.
#[derive(Debug, Clone, Deserialize)]
pub struct StreamItemLink {
    /// URL.
    pub href: String,
    /// MIME type.
    #[serde(rename = "type", default)]
    pub link_type: Option<String>,
}

/// Content of a stream item.
#[derive(Debug, Clone, Deserialize)]
pub struct StreamItemContent {
    /// Direction (ltr/rtl).
    #[serde(default)]
    pub direction: Option<String>,
    /// Content HTML.
    pub content: String,
}

/// Response from stream/items/ids endpoint.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamItemIds {
    /// Item references.
    pub item_refs: Vec<ItemRef>,
    /// Continuation token.
    #[serde(default)]
    pub continuation: Option<String>,
}

/// A reference to an item (ID only).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemRef {
    /// Item ID (decimal string).
    pub id: String,
    /// Timestamp in microseconds.
    #[serde(default)]
    pub timestamp_usec: Option<String>,
    /// Direct stream IDs.
    #[serde(default)]
    pub direct_stream_ids: Option<Vec<String>>,
}

/// Sync configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncConfig {
    /// Sync provider type.
    pub provider: SyncProvider,
    /// Server URL (e.g., "https://freshrss.example.com/api/greader.php").
    pub server: String,
    /// Username.
    pub username: String,
    /// Password or API key (stored securely).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
}

/// Supported sync providers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SyncProvider {
    /// FreshRSS (Google Reader API).
    FreshRSS,
    /// Miniflux (Google Reader API).
    Miniflux,
    /// Generic Google Reader API.
    GReader,
}

impl Default for SyncProvider {
    fn default() -> Self {
        Self::GReader
    }
}

// --- ID Parsing Utilities ---

/// Parse an item ID from any format to decimal.
///
/// Supports:
/// - Long form: `tag:google.com,2005:reader/item/000000000000001F`
/// - Short hex: `000000000000001F` (16 chars)
/// - Decimal: `31`
pub fn parse_item_id(id: &str) -> Option<i64> {
    const PREFIX: &str = "tag:google.com,2005:reader/item/";

    if let Some(hex) = id.strip_prefix(PREFIX) {
        // Long form hex
        i64::from_str_radix(hex, 16).ok()
    } else if id.len() == 16 && id.chars().all(|c| c.is_ascii_hexdigit()) {
        // Short form hex (16 chars, zero-padded)
        i64::from_str_radix(id, 16).ok()
    } else {
        // Decimal
        id.parse().ok()
    }
}

/// Format an item ID as long form hex.
pub fn format_item_id_long(id: i64) -> String {
    format!("tag:google.com,2005:reader/item/{:016x}", id as u64)
}

/// Format an item ID as short form hex.
pub fn format_item_id_short(id: i64) -> String {
    format!("{:016x}", id as u64)
}

// --- Stream IDs ---

/// Well-known stream IDs.
pub mod streams {
    /// All items (reading list).
    pub const READING_LIST: &str = "user/-/state/com.google/reading-list";
    /// Read items.
    pub const READ: &str = "user/-/state/com.google/read";
    /// Starred items.
    pub const STARRED: &str = "user/-/state/com.google/starred";
    /// Kept unread items.
    pub const KEPT_UNREAD: &str = "user/-/state/com.google/kept-unread";
    /// Broadcast items.
    pub const BROADCAST: &str = "user/-/state/com.google/broadcast";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_item_id_long() {
        let id = "tag:google.com,2005:reader/item/000000000000001f";
        assert_eq!(parse_item_id(id), Some(31));
    }

    #[test]
    fn test_parse_item_id_short_hex() {
        let id = "000000000000001f";
        assert_eq!(parse_item_id(id), Some(31));
    }

    #[test]
    fn test_parse_item_id_decimal() {
        let id = "31";
        assert_eq!(parse_item_id(id), Some(31));
    }

    #[test]
    fn test_format_item_id_long() {
        assert_eq!(
            format_item_id_long(31),
            "tag:google.com,2005:reader/item/000000000000001f"
        );
    }
}
