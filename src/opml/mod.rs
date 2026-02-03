//! OPML import and export.
//!
//! OPML (Outline Processor Markup Language) is the standard format
//! for exchanging RSS subscription lists between applications.

use std::{fs, path::Path};

use color_eyre::Result;
use quick_xml::{
    events::{BytesStart, Event},
    Reader,
};

use crate::config::{Config, FeedConfig, FolderConfig};

/// Import feeds from an OPML file.
///
/// # Errors
///
/// Returns an error if the file cannot be read or parsed.
pub fn import(path: &Path, config: &mut Config) -> Result<usize> {
    let content = fs::read_to_string(path)?;
    let outlines = parse_opml(&content)?;

    let mut imported = 0;

    for outline in outlines {
        if let Some(url) = &outline.xml_url {
            // Root-level feed
            config.feeds.push(FeedConfig {
                name: outline.title.clone(),
                url: url.clone(),
            });
            imported += 1;
        } else if !outline.children.is_empty() {
            // Folder with feeds
            let folder_feeds: Vec<FeedConfig> = outline
                .children
                .iter()
                .filter_map(|child| {
                    child.xml_url.as_ref().map(|url| FeedConfig {
                        name: child.title.clone(),
                        url: url.clone(),
                    })
                })
                .collect();

            imported += folder_feeds.len();

            if !folder_feeds.is_empty() {
                config.folders.push(FolderConfig {
                    name: outline.title,
                    icon: None,
                    expanded: true,
                    feeds: folder_feeds,
                });
            }
        }
    }

    Ok(imported)
}

/// Export feeds to an OPML file.
///
/// # Errors
///
/// Returns an error if the file cannot be written.
pub fn export(config: &Config, path: &Path) -> Result<()> {
    let mut xml = String::from(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<opml version="2.0">
  <head>
    <title>Feedo Subscriptions</title>
  </head>
  <body>
"#,
    );

    // Export folders
    for folder in &config.folders {
        xml.push_str(&format!(
            r#"    <outline text="{}" title="{}">"#,
            escape_xml(&folder.name),
            escape_xml(&folder.name)
        ));
        xml.push('\n');

        for feed in &folder.feeds {
            xml.push_str(&format!(
                r#"      <outline type="rss" text="{}" title="{}" xmlUrl="{}"/>"#,
                escape_xml(&feed.name),
                escape_xml(&feed.name),
                escape_xml(&feed.url)
            ));
            xml.push('\n');
        }

        xml.push_str("    </outline>\n");
    }

    // Export root-level feeds
    for feed in &config.feeds {
        xml.push_str(&format!(
            r#"    <outline type="rss" text="{}" title="{}" xmlUrl="{}"/>"#,
            escape_xml(&feed.name),
            escape_xml(&feed.name),
            escape_xml(&feed.url)
        ));
        xml.push('\n');
    }

    xml.push_str("  </body>\n</opml>\n");

    fs::write(path, xml)?;
    Ok(())
}

/// Internal OPML outline structure.
#[derive(Debug, Clone)]
struct OpmlOutline {
    title: String,
    xml_url: Option<String>,
    children: Vec<OpmlOutline>,
}

/// Parse OPML XML into outline structures.
fn parse_opml(content: &str) -> Result<Vec<OpmlOutline>> {
    let mut reader = Reader::from_str(content);
    reader.config_mut().trim_text(true);

    let mut outlines = Vec::new();
    let mut stack: Vec<OpmlOutline> = Vec::new();
    let mut in_body = false;

    loop {
        match reader.read_event() {
            Ok(Event::Start(ref e)) => {
                let name_bytes = e.name();
                let name = std::str::from_utf8(name_bytes.as_ref()).unwrap_or("");

                if name.eq_ignore_ascii_case("body") {
                    in_body = true;
                } else if in_body && name.eq_ignore_ascii_case("outline") {
                    stack.push(parse_outline_attrs(e));
                }
            }
            Ok(Event::Empty(ref e)) => {
                let name_bytes = e.name();
                let name = std::str::from_utf8(name_bytes.as_ref()).unwrap_or("");

                if in_body && name.eq_ignore_ascii_case("outline") {
                    let outline = parse_outline_attrs(e);
                    if let Some(parent) = stack.last_mut() {
                        parent.children.push(outline);
                    } else {
                        outlines.push(outline);
                    }
                }
            }
            Ok(Event::End(ref e)) => {
                let name_bytes = e.name();
                let name = std::str::from_utf8(name_bytes.as_ref()).unwrap_or("");

                if name.eq_ignore_ascii_case("outline") {
                    if let Some(outline) = stack.pop() {
                        if let Some(parent) = stack.last_mut() {
                            parent.children.push(outline);
                        } else {
                            outlines.push(outline);
                        }
                    }
                } else if name.eq_ignore_ascii_case("body") {
                    in_body = false;
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(color_eyre::eyre::eyre!("XML parse error: {e}")),
            _ => {}
        }
    }

    Ok(outlines)
}

/// Extract outline attributes from XML element.
fn parse_outline_attrs(e: &BytesStart) -> OpmlOutline {
    let mut title = String::new();
    let mut xml_url = None;

    for attr in e.attributes().flatten() {
        let key = std::str::from_utf8(attr.key.as_ref()).unwrap_or("");
        let value = attr.unescape_value().unwrap_or_default().to_string();

        match key.to_lowercase().as_str() {
            "title" | "text" => {
                if title.is_empty() {
                    title = value;
                }
            }
            "xmlurl" => xml_url = Some(value),
            _ => {}
        }
    }

    OpmlOutline {
        title,
        xml_url,
        children: Vec::new(),
    }
}

/// Escape special XML characters.
fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
