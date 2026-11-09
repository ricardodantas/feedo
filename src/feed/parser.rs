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
                .title
                .map_or_else(|| "Untitled".to_string(), |t| t.content);

            let link = entry.links.first().map(|l| l.href.clone());
            let published = entry.published.or(entry.updated);
            // Prefer the full article over the teaser. feed-rs maps RSS
            // <content:encoded>, inline Atom <content> and JSON Feed
            // content_html/content_text to `content`, and RSS <description>
            // and Atom <summary> to `summary`. Atom content given by `src`
            // has no body, so it falls back to the summary.
            let summary = entry
                .content
                .and_then(|c| c.body)
                .filter(|body| !body.trim().is_empty())
                .or_else(|| entry.summary.map(|s| s.content));

            let id = FeedItem::generate_id(link.as_deref(), &title);

            FeedItem {
                id,
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

#[cfg(test)]
mod tests {
    use super::*;

    const RSS2: &[u8] = include_bytes!("../../tests/fixtures/feeds/rss2_content_encoded.xml");
    const ATOM: &[u8] = include_bytes!("../../tests/fixtures/feeds/atom_content.xml");
    const JSON_FEED: &[u8] = include_bytes!("../../tests/fixtures/feeds/json_feed.json");

    fn body_of(bytes: &[u8], title: &str) -> Option<String> {
        parse_feed(bytes)
            .expect("fixture parses")
            .into_iter()
            .find(|item| item.title == title)
            .unwrap_or_else(|| panic!("fixture has an item titled {title:?}"))
            .summary
    }

    #[test]
    fn rss_item_should_use_content_encoded_over_description() {
        let body = body_of(RSS2, "Full article").expect("item has a body");

        assert!(
            body.contains("The second paragraph, only in the full content."),
            "{body}"
        );
    }

    #[test]
    fn rss_item_should_fall_back_to_description_without_content_encoded() {
        let body = body_of(RSS2, "Summary only");

        assert_eq!(
            body.as_deref(),
            Some("This feed item only has a description.")
        );
    }

    #[test]
    fn rss_item_should_fall_back_to_description_when_content_encoded_is_blank() {
        let body = body_of(RSS2, "Blank full content");

        assert_eq!(
            body.as_deref(),
            Some("The description is the only real text here.")
        );
    }

    #[test]
    fn atom_entry_should_use_inline_content_over_summary() {
        let body = body_of(ATOM, "Atom full article").expect("entry has a body");

        assert!(body.contains("Atom full body, second paragraph."), "{body}");
    }

    #[test]
    fn atom_entry_should_fall_back_to_summary_when_content_is_out_of_line() {
        let body = body_of(ATOM, "Atom out-of-line content");

        assert_eq!(body.as_deref(), Some("Only the summary is inline."));
    }

    #[test]
    fn json_feed_item_should_use_content_html_over_summary() {
        let body = body_of(JSON_FEED, "JSON full article");

        assert_eq!(body.as_deref(), Some("<p>JSON full body.</p>"));
    }
}
