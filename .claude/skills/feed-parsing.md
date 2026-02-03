# RSS/Atom Feed Parsing

## Overview

Feedo uses the `feed-rs` crate to parse RSS 1.0, RSS 2.0, Atom, and JSON Feed formats.

## Basic Usage

```rust
use feed_rs::parser;

fn parse_feed(bytes: &[u8]) -> Result<Vec<FeedItem>> {
    let feed = parser::parse(bytes)?;
    
    let items = feed.entries
        .into_iter()
        .map(|entry| {
            FeedItem {
                title: entry.title
                    .map(|t| t.content)
                    .unwrap_or_else(|| "Untitled".to_string()),
                link: entry.links.first().map(|l| l.href.clone()),
                published: entry.published.or(entry.updated),
                summary: entry.summary.map(|s| s.content),
            }
        })
        .collect();
    
    Ok(items)
}
```

## Feed Entry Fields

| Field | Type | Notes |
|-------|------|-------|
| `id` | `String` | Unique identifier |
| `title` | `Option<Text>` | Entry title |
| `links` | `Vec<Link>` | URLs (first is usually main) |
| `summary` | `Option<Text>` | Short description |
| `content` | `Option<Content>` | Full content |
| `published` | `Option<DateTime>` | Publication date |
| `updated` | `Option<DateTime>` | Last modified |
| `authors` | `Vec<Person>` | Author info |
| `categories` | `Vec<Category>` | Tags/categories |

## Common Feed Formats

### RSS 2.0 (Most Common)
```xml
<?xml version="1.0"?>
<rss version="2.0">
  <channel>
    <title>Example Feed</title>
    <link>https://example.com</link>
    <item>
      <title>Article Title</title>
      <link>https://example.com/article</link>
      <description>Summary text...</description>
      <pubDate>Mon, 03 Feb 2025 12:00:00 GMT</pubDate>
    </item>
  </channel>
</rss>
```

### Atom
```xml
<?xml version="1.0"?>
<feed xmlns="http://www.w3.org/2005/Atom">
  <title>Example Feed</title>
  <entry>
    <title>Article Title</title>
    <link href="https://example.com/article"/>
    <summary>Summary text...</summary>
    <published>2025-02-03T12:00:00Z</published>
  </entry>
</feed>
```

### JSON Feed
```json
{
  "version": "https://jsonfeed.org/version/1",
  "title": "Example Feed",
  "items": [
    {
      "id": "1",
      "title": "Article Title",
      "url": "https://example.com/article",
      "content_text": "Full content...",
      "date_published": "2025-02-03T12:00:00Z"
    }
  ]
}
```

## Handling HTML in Content

Feed content often contains HTML. Strip it for display:

```rust
fn strip_html(s: &str) -> String {
    // Convert common tags to text equivalents
    let clean = s
        .replace("<p>", "\n")
        .replace("</p>", "\n")
        .replace("<br>", "\n")
        .replace("<br/>", "\n")
        .replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">");
    
    // Remove remaining tags
    regex_lite::Regex::new(r"<[^>]+>")
        .map(|re| re.replace_all(&clean, "").to_string())
        .unwrap_or(clean)
}
```

## Date Handling

Dates come in various formats. `feed-rs` normalizes to `chrono::DateTime<Utc>`:

```rust
use chrono::{DateTime, Utc};

// Prefer published, fall back to updated
let date: Option<DateTime<Utc>> = entry.published.or(entry.updated);

// Format for display
if let Some(d) = date {
    println!("{}", d.format("%Y-%m-%d %H:%M"));
}
```

## Error Handling

```rust
match parser::parse(bytes) {
    Ok(feed) => {
        // Process feed
    }
    Err(e) => {
        // Common errors:
        // - Invalid XML/JSON
        // - Not a feed (HTML page)
        // - Encoding issues
        tracing::warn!("Failed to parse feed: {e}");
    }
}
```

## Feed Discovery

Many sites don't advertise their feed URL. Common patterns:

```rust
fn guess_feed_urls(site_url: &str) -> Vec<String> {
    let base = site_url.trim_end_matches('/');
    vec![
        format!("{base}/feed"),
        format!("{base}/rss"),
        format!("{base}/feed.xml"),
        format!("{base}/rss.xml"),
        format!("{base}/atom.xml"),
        format!("{base}/feed.json"),
        format!("{base}/index.xml"),
    ]
}
```

Or parse HTML for `<link rel="alternate" type="application/rss+xml">` tags.

## Performance Tips

1. **Stream large feeds** — Don't load entire feed into memory if huge
2. **Cache ETags** — Use conditional requests to avoid re-downloading
3. **Respect TTL** — RSS has `<ttl>` element suggesting refresh interval
4. **Parallel fetching** — Use `join_all` to fetch multiple feeds concurrently

## Resources

- [feed-rs docs](https://docs.rs/feed-rs)
- [RSS 2.0 Spec](https://www.rssboard.org/rss-specification)
- [Atom Spec (RFC 4287)](https://tools.ietf.org/html/rfc4287)
- [JSON Feed Spec](https://jsonfeed.org/version/1.1)
