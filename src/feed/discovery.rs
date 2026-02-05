//! Feed discovery - auto-detect RSS/Atom feeds from URLs.
//!
//! This module provides functionality to:
//! - Detect feed URLs from website HTML (via `<link>` tags)
//! - Validate that a URL is a valid RSS/Atom feed
//! - Extract feed metadata (title, description)

use color_eyre::{Result, eyre::eyre};
use regex_lite::Regex;
use tracing::debug;

/// Discovered feed information.
#[derive(Debug, Clone)]
pub struct DiscoveredFeed {
    /// Feed URL.
    pub url: String,
    /// Feed title (if discovered).
    pub title: Option<String>,
    /// Feed type (RSS, Atom, etc.).
    pub feed_type: FeedType,
}

/// Type of feed discovered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeedType {
    /// RSS 1.0 or 2.0
    Rss,
    /// Atom feed
    Atom,
    /// JSON Feed
    Json,
    /// Unknown but valid feed
    Unknown,
}

impl std::fmt::Display for FeedType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Rss => write!(f, "RSS"),
            Self::Atom => write!(f, "Atom"),
            Self::Json => write!(f, "JSON Feed"),
            Self::Unknown => write!(f, "Feed"),
        }
    }
}

/// Feed discovery client.
pub struct FeedDiscovery {
    client: reqwest::Client,
}

impl FeedDiscovery {
    /// Create a new feed discovery client.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP client cannot be created.
    pub fn new() -> Result<Self> {
        let client = reqwest::Client::builder()
            .user_agent(concat!("feedo/", env!("CARGO_PKG_VERSION")))
            .timeout(std::time::Duration::from_secs(15))
            .redirect(reqwest::redirect::Policy::limited(5))
            .build()?;

        Ok(Self { client })
    }

    /// Discover feeds from a URL.
    ///
    /// This will:
    /// 1. Try the URL directly as a feed
    /// 2. If it's HTML, look for `<link>` tags pointing to feeds
    /// 3. Try common feed URL patterns (/feed, /rss, etc.)
    ///
    /// # Errors
    ///
    /// Returns an error if no feeds can be discovered.
    pub async fn discover(&self, url: &str) -> Result<Vec<DiscoveredFeed>> {
        let url = normalize_url(url)?;
        debug!("Discovering feeds from: {url}");

        let mut feeds = Vec::new();

        // Try direct URL first
        if let Ok(feed) = self.try_as_feed(&url).await {
            debug!("URL is a direct feed");
            feeds.push(feed);
            return Ok(feeds);
        }

        // Fetch the page and look for feed links
        let response = self.client.get(&url).send().await?;
        let content_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");

        if content_type.contains("text/html") {
            let html = response.text().await?;
            let discovered = Self::extract_feed_links(&html, &url);

            // Validate each discovered URL
            for feed_url in discovered {
                if let Ok(feed) = self.try_as_feed(&feed_url).await {
                    feeds.push(feed);
                }
            }
        }

        // Try common feed paths if nothing found
        if feeds.is_empty() {
            let base_url = extract_base_url(&url);
            let common_paths = [
                "/feed",
                "/feed/",
                "/rss",
                "/rss/",
                "/rss.xml",
                "/feed.xml",
                "/atom.xml",
                "/index.xml",
                "/feed.json",
                "/.rss",
                "/blog/feed",
                "/blog/rss",
            ];

            for path in common_paths {
                let test_url = format!("{base_url}{path}");
                if let Ok(feed) = self.try_as_feed(&test_url).await {
                    feeds.push(feed);
                    break; // Found one, good enough
                }
            }
        }

        if feeds.is_empty() {
            Err(eyre!("No feeds found at {url}"))
        } else {
            Ok(feeds)
        }
    }

    /// Try to parse a URL as a feed directly.
    async fn try_as_feed(&self, url: &str) -> Result<DiscoveredFeed> {
        let response = self.client.get(url).send().await?;

        if !response.status().is_success() {
            return Err(eyre!("HTTP {}", response.status()));
        }

        let content_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();

        let bytes = response.bytes().await?;

        // Try to parse as feed
        let feed = feed_rs::parser::parse(&bytes[..])?;

        let feed_type = if content_type.contains("json") {
            FeedType::Json
        } else if content_type.contains("atom") || feed.feed_type == feed_rs::model::FeedType::Atom
        {
            FeedType::Atom
        } else if content_type.contains("rss")
            || matches!(
                feed.feed_type,
                feed_rs::model::FeedType::RSS0
                    | feed_rs::model::FeedType::RSS1
                    | feed_rs::model::FeedType::RSS2
            )
        {
            FeedType::Rss
        } else {
            FeedType::Unknown
        };

        Ok(DiscoveredFeed {
            url: url.to_string(),
            title: feed.title.map(|t| t.content),
            feed_type,
        })
    }

    /// Extract feed links from HTML.
    fn extract_feed_links(html: &str, base_url: &str) -> Vec<String> {
        let mut feeds = Vec::new();
        let base = extract_base_url(base_url);

        // Pattern to match <link> tags with feed types
        // This is simplified for regex-lite compatibility
        let link_pattern = Regex::new(r"<link[^>]*>").unwrap();

        for cap in link_pattern.find_iter(html) {
            let tag = cap.as_str();

            // Check if it's a feed link
            let is_feed = tag.contains("application/rss+xml")
                || tag.contains("application/atom+xml")
                || tag.contains("application/feed+json");

            if is_feed {
                // Extract href
                if let Some(href) = extract_href(tag) {
                    let full_url = resolve_url(&href, &base);
                    if !feeds.contains(&full_url) {
                        feeds.push(full_url);
                    }
                }
            }
        }

        // Also look for obvious feed links in <a> tags
        let a_pattern = Regex::new(r#"<a[^>]*href="([^"]*)"[^>]*>"#).unwrap();
        for cap in a_pattern.captures_iter(html) {
            if let Some(href) = cap.get(1) {
                let href_str = href.as_str().to_lowercase();
                let looks_like_feed = href_str.contains("rss")
                    || href_str.contains("feed")
                    || href_str.contains("atom");
                let has_feed_extension = href_str.ends_with("/rss")
                    || href_str.ends_with("/feed")
                    || href_str.to_lowercase().ends_with(".xml");

                if looks_like_feed && has_feed_extension {
                    let full_url = resolve_url(href.as_str(), &base);
                    if !feeds.contains(&full_url) {
                        feeds.push(full_url);
                    }
                }
            }
        }

        feeds
    }
}

/// Extract href attribute from a tag string.
fn extract_href(tag: &str) -> Option<String> {
    // Try href="..."
    if let Some(start) = tag.find("href=\"") {
        let start = start + 6;
        if let Some(end) = tag[start..].find('"') {
            return Some(tag[start..start + end].to_string());
        }
    }
    // Try href='...'
    if let Some(start) = tag.find("href='") {
        let start = start + 6;
        if let Some(end) = tag[start..].find('\'') {
            return Some(tag[start..start + end].to_string());
        }
    }
    None
}

/// Normalize a URL (add https:// if missing, etc.)
fn normalize_url(url: &str) -> Result<String> {
    let url = url.trim();

    if url.is_empty() {
        return Err(eyre!("URL is empty"));
    }

    // Add https:// if no protocol
    let url = if url.contains("://") {
        url.to_string()
    } else {
        format!("https://{url}")
    };

    // Validate URL
    let _ = reqwest::Url::parse(&url)?;

    Ok(url)
}

/// Extract the base URL (scheme + host) from a full URL.
fn extract_base_url(url: &str) -> String {
    reqwest::Url::parse(url).map_or_else(
        |_| url.to_string(),
        |parsed| format!("{}://{}", parsed.scheme(), parsed.host_str().unwrap_or("")),
    )
}

/// Resolve a potentially relative URL against a base.
fn resolve_url(href: &str, base: &str) -> String {
    if href.starts_with("http://") || href.starts_with("https://") {
        href.to_string()
    } else if href.starts_with("//") {
        format!("https:{href}")
    } else if href.starts_with('/') {
        format!("{base}{href}")
    } else {
        format!("{base}/{href}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_url() {
        assert_eq!(normalize_url("example.com").unwrap(), "https://example.com");
        assert_eq!(
            normalize_url("https://example.com").unwrap(),
            "https://example.com"
        );
        assert_eq!(
            normalize_url("http://example.com").unwrap(),
            "http://example.com"
        );
    }

    #[test]
    fn test_extract_base_url() {
        assert_eq!(
            extract_base_url("https://example.com/path/to/page"),
            "https://example.com"
        );
        assert_eq!(
            extract_base_url("http://blog.example.com/feed"),
            "http://blog.example.com"
        );
    }

    #[test]
    fn test_resolve_url() {
        let base = "https://example.com";
        assert_eq!(resolve_url("/feed", base), "https://example.com/feed");
        assert_eq!(
            resolve_url("//cdn.example.com/feed", base),
            "https://cdn.example.com/feed"
        );
        assert_eq!(
            resolve_url("https://other.com/feed", base),
            "https://other.com/feed"
        );
    }

    #[test]
    fn test_extract_href() {
        assert_eq!(
            extract_href(r"<link href='https://example.com/feed' rel='alternate'>"),
            Some("https://example.com/feed".to_string())
        );
        assert_eq!(
            extract_href(r"<link href='/rss.xml' type='application/rss+xml'>"),
            Some("/rss.xml".to_string())
        );
    }
}
