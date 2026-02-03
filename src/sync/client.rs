//! Google Reader API client implementation.

#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::doc_markdown)]
#![allow(clippy::format_push_string)]
#![allow(clippy::uninlined_format_args)]
#![allow(clippy::must_use_candidate)]

use color_eyre::{Result, eyre::eyre};
use reqwest::{Client, header};
use tracing::debug;

use super::types::{
    AuthToken, StreamContents, StreamItemIds, Subscription, SubscriptionList, Tag, TagList,
    UnreadCount, UserInfo, streams,
};

/// Google Reader API client.
#[derive(Debug, Clone)]
pub struct GReaderClient {
    /// Base URL of the API (e.g., "https://freshrss.example.com/api/greader.php").
    base_url: String,
    /// HTTP client.
    client: Client,
}

impl GReaderClient {
    /// Create a new client.
    ///
    /// # Arguments
    ///
    /// * `base_url` - The API base URL (e.g., "https://freshrss.example.com/api/greader.php")
    pub fn new(base_url: impl Into<String>) -> Self {
        let base_url = base_url.into().trim_end_matches('/').to_string();
        let client = Client::builder()
            .user_agent(concat!("feedo/", env!("CARGO_PKG_VERSION")))
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        Self { base_url, client }
    }

    /// Login and get an auth token.
    ///
    /// # Arguments
    ///
    /// * `username` - Username or email
    /// * `password` - Password or API key
    pub async fn login(&self, username: &str, password: &str) -> Result<AuthToken> {
        let url = format!("{}/accounts/ClientLogin", self.base_url);

        let response = self
            .client
            .post(&url)
            .form(&[("Email", username), ("Passwd", password)])
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(eyre!(
                "Login failed: {} {}",
                response.status(),
                response.text().await.unwrap_or_default()
            ));
        }

        let text = response.text().await?;
        debug!("Login response: {}", text);

        // Parse response: SID=...\nLSID=...\nAuth=...
        for line in text.lines() {
            if let Some(token) = line.strip_prefix("Auth=") {
                return Ok(AuthToken {
                    token: token.to_string(),
                });
            }
        }

        Err(eyre!("No Auth token in login response"))
    }

    /// Get a CSRF token for write operations.
    pub async fn token(&self, auth: &AuthToken) -> Result<String> {
        let url = format!("{}/reader/api/0/token", self.base_url);

        let response = self
            .client
            .get(&url)
            .header(
                header::AUTHORIZATION,
                format!("GoogleLogin auth={}", auth.token),
            )
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(eyre!("Failed to get token: {}", response.status()));
        }

        Ok(response.text().await?)
    }

    /// Get user information.
    pub async fn user_info(&self, auth: &AuthToken) -> Result<UserInfo> {
        let url = format!("{}/reader/api/0/user-info?output=json", self.base_url);

        let response = self
            .client
            .get(&url)
            .header(
                header::AUTHORIZATION,
                format!("GoogleLogin auth={}", auth.token),
            )
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(eyre!("Failed to get user info: {}", response.status()));
        }

        Ok(response.json().await?)
    }

    /// List subscriptions (feeds).
    pub async fn subscriptions(&self, auth: &AuthToken) -> Result<Vec<Subscription>> {
        let url = format!(
            "{}/reader/api/0/subscription/list?output=json",
            self.base_url
        );

        let response = self
            .client
            .get(&url)
            .header(
                header::AUTHORIZATION,
                format!("GoogleLogin auth={}", auth.token),
            )
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(eyre!("Failed to list subscriptions: {}", response.status()));
        }

        let list: SubscriptionList = response.json().await?;
        Ok(list.subscriptions)
    }

    /// List tags (categories/folders).
    pub async fn tags(&self, auth: &AuthToken) -> Result<Vec<Tag>> {
        let url = format!("{}/reader/api/0/tag/list?output=json", self.base_url);

        let response = self
            .client
            .get(&url)
            .header(
                header::AUTHORIZATION,
                format!("GoogleLogin auth={}", auth.token),
            )
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(eyre!("Failed to list tags: {}", response.status()));
        }

        let list: TagList = response.json().await?;
        Ok(list.tags)
    }

    /// Get unread counts.
    pub async fn unread_count(&self, auth: &AuthToken) -> Result<UnreadCount> {
        let url = format!("{}/reader/api/0/unread-count?output=json", self.base_url);

        let response = self
            .client
            .get(&url)
            .header(
                header::AUTHORIZATION,
                format!("GoogleLogin auth={}", auth.token),
            )
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(eyre!("Failed to get unread count: {}", response.status()));
        }

        Ok(response.json().await?)
    }

    /// Get stream contents (items).
    ///
    /// # Arguments
    ///
    /// * `auth` - Auth token
    /// * `stream_id` - Stream ID (e.g., "user/-/state/com.google/reading-list" or "feed/123")
    /// * `options` - Optional query parameters
    pub async fn stream_contents(
        &self,
        auth: &AuthToken,
        stream_id: &str,
        options: Option<StreamOptions>,
    ) -> Result<StreamContents> {
        let encoded_stream = urlencoding::encode(stream_id);
        let mut url = format!(
            "{}/reader/api/0/stream/contents/{}?output=json",
            self.base_url, encoded_stream
        );

        if let Some(opts) = options {
            if let Some(n) = opts.count {
                url.push_str(&format!("&n={}", n));
            }
            if let Some(c) = opts.continuation {
                url.push_str(&format!("&c={}", c));
            }
            if let Some(ot) = opts.older_than {
                url.push_str(&format!("&ot={}", ot));
            }
            if let Some(nt) = opts.newer_than {
                url.push_str(&format!("&nt={}", nt));
            }
            if let Some(xt) = opts.exclude_target {
                url.push_str(&format!("&xt={}", urlencoding::encode(&xt)));
            }
            if opts.unread_only {
                url.push_str(&format!("&xt={}", urlencoding::encode(streams::READ)));
            }
        }

        debug!("Fetching stream: {}", url);

        let response = self
            .client
            .get(&url)
            .header(
                header::AUTHORIZATION,
                format!("GoogleLogin auth={}", auth.token),
            )
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(eyre!(
                "Failed to get stream contents: {}",
                response.status()
            ));
        }

        Ok(response.json().await?)
    }

    /// Get item IDs from a stream (more efficient for sync).
    pub async fn stream_item_ids(
        &self,
        auth: &AuthToken,
        stream_id: &str,
        options: Option<StreamOptions>,
    ) -> Result<StreamItemIds> {
        let encoded_stream = urlencoding::encode(stream_id);
        let mut url = format!(
            "{}/reader/api/0/stream/items/ids?output=json&s={}",
            self.base_url, encoded_stream
        );

        if let Some(opts) = options {
            if let Some(n) = opts.count {
                url.push_str(&format!("&n={}", n));
            }
            if let Some(c) = opts.continuation {
                url.push_str(&format!("&c={}", c));
            }
            if let Some(ot) = opts.older_than {
                url.push_str(&format!("&ot={}", ot));
            }
            if let Some(nt) = opts.newer_than {
                url.push_str(&format!("&nt={}", nt));
            }
            if opts.unread_only {
                url.push_str(&format!("&xt={}", urlencoding::encode(streams::READ)));
            }
        }

        let response = self
            .client
            .get(&url)
            .header(
                header::AUTHORIZATION,
                format!("GoogleLogin auth={}", auth.token),
            )
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(eyre!(
                "Failed to get stream item IDs: {}",
                response.status()
            ));
        }

        Ok(response.json().await?)
    }

    /// Get contents for specific item IDs.
    pub async fn items_contents(
        &self,
        auth: &AuthToken,
        item_ids: &[&str],
    ) -> Result<StreamContents> {
        let url = format!(
            "{}/reader/api/0/stream/items/contents?output=json",
            self.base_url
        );

        // Build form data with multiple 'i' parameters
        let mut form_data = Vec::new();
        for id in item_ids {
            form_data.push(("i", *id));
        }

        let response = self
            .client
            .post(&url)
            .header(
                header::AUTHORIZATION,
                format!("GoogleLogin auth={}", auth.token),
            )
            .form(&form_data)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(eyre!("Failed to get item contents: {}", response.status()));
        }

        Ok(response.json().await?)
    }

    /// Edit tags on an item (mark read, star, etc.).
    ///
    /// # Arguments
    ///
    /// * `auth` - Auth token
    /// * `item_ids` - Item IDs to modify
    /// * `add_tag` - Tag to add (e.g., "user/-/state/com.google/read")
    /// * `remove_tag` - Tag to remove
    pub async fn edit_tag(
        &self,
        auth: &AuthToken,
        item_ids: &[&str],
        add_tag: Option<&str>,
        remove_tag: Option<&str>,
    ) -> Result<()> {
        let token = self.token(auth).await?;
        let url = format!("{}/reader/api/0/edit-tag", self.base_url);

        let mut form_data: Vec<(&str, &str)> = vec![("T", &token)];

        for id in item_ids {
            form_data.push(("i", *id));
        }

        if let Some(tag) = add_tag {
            form_data.push(("a", tag));
        }

        if let Some(tag) = remove_tag {
            form_data.push(("r", tag));
        }

        let response = self
            .client
            .post(&url)
            .header(
                header::AUTHORIZATION,
                format!("GoogleLogin auth={}", auth.token),
            )
            .form(&form_data)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(eyre!("Failed to edit tag: {}", response.status()));
        }

        Ok(())
    }

    /// Mark items as read.
    pub async fn mark_read(&self, auth: &AuthToken, item_ids: &[&str]) -> Result<()> {
        self.edit_tag(auth, item_ids, Some(streams::READ), None)
            .await
    }

    /// Mark items as unread.
    pub async fn mark_unread(&self, auth: &AuthToken, item_ids: &[&str]) -> Result<()> {
        self.edit_tag(auth, item_ids, None, Some(streams::READ))
            .await
    }

    /// Star items.
    pub async fn star(&self, auth: &AuthToken, item_ids: &[&str]) -> Result<()> {
        self.edit_tag(auth, item_ids, Some(streams::STARRED), None)
            .await
    }

    /// Unstar items.
    pub async fn unstar(&self, auth: &AuthToken, item_ids: &[&str]) -> Result<()> {
        self.edit_tag(auth, item_ids, None, Some(streams::STARRED))
            .await
    }

    /// Mark all items in a stream as read.
    ///
    /// # Arguments
    ///
    /// * `auth` - Auth token
    /// * `stream_id` - Stream to mark read (feed ID or category)
    /// * `timestamp` - Mark items older than this timestamp (seconds since epoch)
    pub async fn mark_all_as_read(
        &self,
        auth: &AuthToken,
        stream_id: &str,
        timestamp: Option<i64>,
    ) -> Result<()> {
        let token = self.token(auth).await?;
        let url = format!("{}/reader/api/0/mark-all-as-read", self.base_url);

        let mut form_data = vec![("T", token.as_str()), ("s", stream_id)];

        let ts_string;
        if let Some(ts) = timestamp {
            ts_string = ts.to_string();
            form_data.push(("ts", &ts_string));
        }

        let response = self
            .client
            .post(&url)
            .header(
                header::AUTHORIZATION,
                format!("GoogleLogin auth={}", auth.token),
            )
            .form(&form_data)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(eyre!("Failed to mark all as read: {}", response.status()));
        }

        Ok(())
    }

    /// Add a subscription.
    pub async fn add_subscription(
        &self,
        auth: &AuthToken,
        feed_url: &str,
        title: Option<&str>,
        category: Option<&str>,
    ) -> Result<()> {
        let token = self.token(auth).await?;
        let url = format!("{}/reader/api/0/subscription/edit", self.base_url);
        let feed_id = format!("feed/{}", feed_url);

        let mut form_data: Vec<(&str, &str)> =
            vec![("T", token.as_str()), ("ac", "subscribe"), ("s", &feed_id)];

        if let Some(t) = title {
            form_data.push(("t", t));
        }

        if let Some(cat) = category {
            form_data.push(("a", cat));
        }

        let response = self
            .client
            .post(&url)
            .header(
                header::AUTHORIZATION,
                format!("GoogleLogin auth={}", auth.token),
            )
            .form(&form_data)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(eyre!("Failed to add subscription: {}", response.status()));
        }

        Ok(())
    }

    /// Remove a subscription.
    pub async fn remove_subscription(&self, auth: &AuthToken, feed_id: &str) -> Result<()> {
        let token = self.token(auth).await?;
        let url = format!("{}/reader/api/0/subscription/edit", self.base_url);

        let form_data = vec![("T", token.as_str()), ("ac", "unsubscribe"), ("s", feed_id)];

        let response = self
            .client
            .post(&url)
            .header(
                header::AUTHORIZATION,
                format!("GoogleLogin auth={}", auth.token),
            )
            .form(&form_data)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(eyre!(
                "Failed to remove subscription: {}",
                response.status()
            ));
        }

        Ok(())
    }

    /// Rename a subscription.
    pub async fn rename_subscription(
        &self,
        auth: &AuthToken,
        feed_id: &str,
        new_title: &str,
    ) -> Result<()> {
        let token = self.token(auth).await?;
        let url = format!("{}/reader/api/0/subscription/edit", self.base_url);

        let form_data = vec![
            ("T", token.as_str()),
            ("ac", "edit"),
            ("s", feed_id),
            ("t", new_title),
        ];

        let response = self
            .client
            .post(&url)
            .header(
                header::AUTHORIZATION,
                format!("GoogleLogin auth={}", auth.token),
            )
            .form(&form_data)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(eyre!(
                "Failed to rename subscription: {}",
                response.status()
            ));
        }

        Ok(())
    }
}

/// Options for stream queries.
#[derive(Debug, Clone, Default)]
pub struct StreamOptions {
    /// Number of items to fetch.
    pub count: Option<u32>,
    /// Continuation token for pagination.
    pub continuation: Option<String>,
    /// Only items older than this timestamp (seconds).
    pub older_than: Option<i64>,
    /// Only items newer than this timestamp (seconds).
    pub newer_than: Option<i64>,
    /// Exclude items with this tag.
    pub exclude_target: Option<String>,
    /// Only unread items (convenience for exclude_target=read).
    pub unread_only: bool,
}

impl StreamOptions {
    /// Create options for fetching unread items.
    pub fn unread() -> Self {
        Self {
            unread_only: true,
            ..Default::default()
        }
    }

    /// Create options with a count limit.
    pub fn with_count(count: u32) -> Self {
        Self {
            count: Some(count),
            ..Default::default()
        }
    }
}
