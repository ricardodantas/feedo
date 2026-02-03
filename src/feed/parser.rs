//! Feed parsing utilities.

use color_eyre::Result;
use feed_rs::parser;

use super::FeedItem;

/// Parse raw feed bytes into a list of feed items.
///
/// # Errors
///
/// Returns an error if the feed cannot be parsed.
pub fn parse_feed(bytes: &[u8]) -> Result<Vec<FeedItem>> {
    let feed = parser::parse(bytes)?;

    let items = feed
        .entries
        .into_iter()
        .map(|entry| {
            let title = entry
                .title.map_or_else(|| "Untitled".to_string(), |t| t.content);

            let link = entry.links.first().map(|l| l.href.clone());
            let published = entry.published.or(entry.updated);
            let summary = entry
                .summary
                .map(|s| s.content)
                .or_else(|| entry.content.and_then(|c| c.body));

            FeedItem {
                title,
                link,
                published,
                summary,
                read: false,
            }
        })
        .collect();

    Ok(items)
}
