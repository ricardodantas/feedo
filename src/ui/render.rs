//! UI rendering.

use ratatui::{
    prelude::*,
    widgets::*,
};

use crate::app::App;
use super::{Mode, Panel};
use super::state::FeedListItem;

/// Modern ASCII art logo for Feedo - a cute RSS-eating dog.
pub const LOGO: &str = r#"
                    ╭──────────────────────────────────────────╮
                    │                                          │
                    │      ██████╗██████╗██████╗██████╗  ██████╗│
                    │      ██╔═══╝██╔═══╝██╔═══╝██╔══██╗██╔═══██╗
                    │      █████╗ █████╗ █████╗ ██║  ██║██║   ██║
                    │      ██╔══╝ ██╔══╝ ██╔══╝ ██║  ██║██║   ██║
                    │      ██║    ██████╗██████╗██████╔╝╚██████╔╝
                    │      ╚═╝    ╚═════╝╚═════╝╚═════╝  ╚═════╝ 
                    │                                          │
                    │           ∩＿∩                            │
                    │          (◕ᴥ◕)  ♪ nom nom RSS ♪          │
                    │         ⊂(　 )つ                          │
                    │          /　　\                           │
                    │         (_/￣＼_)                          │
                    │                                          │
                    │      Your terminal RSS companion 🦴       │
                    │                                          │
                    ╰──────────────────────────────────────────╯
"#;

/// Compact logo for the title bar.
pub const LOGO_COMPACT: &str = "◉ feedo";

/// Minimal dog icon.
pub const DOG_ICON: &str = "(◕ᴥ◕)";

impl App {
    /// Render the entire UI.
    pub fn render(&self, frame: &mut Frame) {
        let area = frame.area();

        // Main layout: title bar, content, status bar
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),  // Title bar
                Constraint::Min(0),     // Content
                Constraint::Length(1),  // Status bar
            ])
            .split(area);

        self.render_title_bar(frame, layout[0]);
        self.render_content(frame, layout[1]);
        self.render_status_bar(frame, layout[2]);

        // Overlays
        if self.ui.mode == Mode::Search {
            self.render_search_overlay(frame, area);
        }

        if let Some(error) = &self.ui.error {
            self.render_error_overlay(frame, area, error);
        }
    }

    fn render_title_bar(&self, frame: &mut Frame, area: Rect) {
        let unread = self.feeds.total_unread_count();
        let title = if unread > 0 {
            format!(" {} │ {} unread", LOGO_COMPACT, unread)
        } else {
            format!(" {}", LOGO_COMPACT)
        };

        let bar = Paragraph::new(title)
            .style(Style::default()
                .fg(self.theme.accent())
                .add_modifier(Modifier::BOLD));
        
        frame.render_widget(bar, area);
    }

    fn render_content(&self, frame: &mut Frame, area: Rect) {
        let constraints = if self.ui.show_content {
            [
                Constraint::Percentage(20),
                Constraint::Percentage(30),
                Constraint::Percentage(50),
            ]
            .as_ref()
        } else {
            [
                Constraint::Percentage(30),
                Constraint::Percentage(70),
                Constraint::Percentage(0),
            ]
            .as_ref()
        };

        let layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(constraints)
            .split(area);

        self.render_feeds_panel(frame, layout[0]);
        self.render_items_panel(frame, layout[1]);

        if self.ui.show_content {
            self.render_content_panel(frame, layout[2]);
        }
    }

    fn render_feeds_panel(&self, frame: &mut Frame, area: Rect) {
        let is_active = self.ui.panel == Panel::Feeds;
        let accent = self.theme.accent();
        let muted = self.theme.muted();

        let items: Vec<ListItem> = self
            .ui
            .feed_list
            .iter()
            .enumerate()
            .map(|(i, list_item)| {
                let is_selected = i == self.ui.feed_list_index;

                match list_item {
                    FeedListItem::Folder(idx) => {
                        let folder = &self.feeds.folders[*idx];
                        let icon = folder.icon.as_deref().unwrap_or("📁");
                        let arrow = if folder.expanded { "▼" } else { "▶" };
                        let unread = self.feeds.folder_unread_count(*idx);

                        let text = if unread > 0 {
                            format!("{arrow} {icon} {} ({unread})", folder.name)
                        } else {
                            format!("{arrow} {icon} {}", folder.name)
                        };

                        let style = if is_selected {
                            Style::default().fg(self.theme.highlight()).bold()
                        } else {
                            Style::default().fg(Color::White).bold()
                        };

                        ListItem::new(text).style(style)
                    }
                    FeedListItem::Feed(idx) => {
                        let feed = &self.feeds.feeds[*idx];
                        let unread = feed.unread_count();
                        
                        // Check if feed is in a folder (indented)
                        let in_folder = self.feeds.folders.iter()
                            .any(|f| f.feed_indices.contains(idx));
                        let indent = if in_folder { "    " } else { "" };

                        let text = if unread > 0 {
                            format!("{indent}● {} ({unread})", feed.name)
                        } else {
                            format!("{indent}○ {}", feed.name)
                        };

                        let style = if is_selected {
                            Style::default().fg(accent).bold()
                        } else if unread > 0 {
                            Style::default().fg(Color::White)
                        } else {
                            Style::default().fg(muted)
                        };

                        ListItem::new(text).style(style)
                    }
                }
            })
            .collect();

        let border_style = if is_active {
            Style::default().fg(accent)
        } else {
            Style::default().fg(muted)
        };

        let list = List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(border_style)
                .border_type(BorderType::Rounded)
                .title(" 📡 Feeds "),
        );

        frame.render_widget(list, area);
    }

    fn render_items_panel(&self, frame: &mut Frame, area: Rect) {
        let is_active = self.ui.panel == Panel::Items;
        let accent = self.theme.accent();
        let muted = self.theme.muted();

        let items: Vec<ListItem> = self
            .current_feed_items()
            .iter()
            .enumerate()
            .map(|(i, item)| {
                let is_selected = i == self.ui.selected_item;
                let prefix = if item.read { "○" } else { "●" };

                let style = if is_selected {
                    Style::default().fg(accent).bold()
                } else if item.read {
                    Style::default().fg(muted)
                } else {
                    Style::default()
                };

                // Truncate title to fit
                let max_width = area.width.saturating_sub(6) as usize;
                let title = if item.title.len() > max_width {
                    format!("{}…", &item.title[..max_width.saturating_sub(1)])
                } else {
                    item.title.clone()
                };

                ListItem::new(format!(" {prefix} {title}")).style(style)
            })
            .collect();

        let border_style = if is_active {
            Style::default().fg(accent)
        } else {
            Style::default().fg(muted)
        };

        let title = self
            .ui
            .selected_feed
            .and_then(|idx| self.feeds.feeds.get(idx))
            .map_or(" Articles ", |f| &f.name);

        let list = List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(border_style)
                .border_type(BorderType::Rounded)
                .title(format!(" 📰 {title} ")),
        );

        frame.render_widget(list, area);
    }

    fn render_content_panel(&self, frame: &mut Frame, area: Rect) {
        let is_active = self.ui.panel == Panel::Content;
        let accent = self.theme.accent();
        let muted = self.theme.muted();

        let content = if let Some(item) = self.selected_item() {
            let mut text = format!("  {}\n\n", item.title);

            if let Some(date) = item.published {
                text.push_str(&format!("  📅 {}\n\n", date.format("%Y-%m-%d %H:%M")));
            }

            if let Some(summary) = &item.summary {
                // Strip HTML tags
                let clean = strip_html(summary);
                text.push_str("  ");
                text.push_str(&clean.replace('\n', "\n  "));
            }

            if let Some(link) = &item.link {
                text.push_str(&format!("\n\n  🔗 {link}"));
            }

            text
        } else {
            format!("\n\n    {DOG_ICON}\n\n    Select an article to read")
        };

        let border_style = if is_active {
            Style::default().fg(accent)
        } else {
            Style::default().fg(muted)
        };

        let paragraph = Paragraph::new(content)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(border_style)
                    .border_type(BorderType::Rounded)
                    .title(" 📖 Content "),
            )
            .wrap(Wrap { trim: false })
            .scroll((self.ui.scroll_offset, 0));

        frame.render_widget(paragraph, area);
    }

    fn render_status_bar(&self, frame: &mut Frame, area: Rect) {
        let muted = self.theme.muted();
        let accent = self.theme.accent();

        let status = if let Some(msg) = &self.ui.status {
            Span::styled(format!(" {DOG_ICON} {msg}"), Style::default().fg(accent))
        } else {
            Span::styled(
                " ↑↓ navigate │ ↵ select │ / search │ r refresh │ o open │ q quit",
                Style::default().fg(muted),
            )
        };

        let bar = Paragraph::new(Line::from(status));
        frame.render_widget(bar, area);
    }

    fn render_search_overlay(&self, frame: &mut Frame, area: Rect) {
        let accent = self.theme.accent();
        let popup_area = centered_rect(60, 50, area);

        frame.render_widget(Clear, popup_area);

        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(0)])
            .split(popup_area);

        // Search input
        let input = Paragraph::new(format!(" 🔍 {}", self.ui.search_query))
            .style(Style::default().fg(accent))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(accent))
                    .border_type(BorderType::Rounded)
                    .title(" Search "),
            );
        frame.render_widget(input, layout[0]);

        // Results
        let results: Vec<ListItem> = self
            .ui
            .search_results
            .iter()
            .enumerate()
            .take(20)
            .map(|(i, (feed_idx, item_idx))| {
                let feed = &self.feeds.feeds[*feed_idx];
                let item = &feed.items[*item_idx];
                let text = format!("  [{feed}] {title}", feed = feed.name, title = item.title);

                let style = if i == self.ui.search_selected {
                    Style::default().fg(accent).bold()
                } else {
                    Style::default()
                };

                ListItem::new(text).style(style)
            })
            .collect();

        let results_title = format!(" Results ({}) ", self.ui.search_results.len());
        let results_list = List::new(results).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(accent))
                .border_type(BorderType::Rounded)
                .title(results_title),
        );
        frame.render_widget(results_list, layout[1]);
    }

    fn render_error_overlay(&self, frame: &mut Frame, area: Rect, error: &str) {
        let popup_area = centered_rect(60, 20, area);
        frame.render_widget(Clear, popup_area);

        let error_block = Paragraph::new(format!("\n  ⚠️  {error}"))
            .style(Style::default().fg(self.theme.error()))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(self.theme.error()))
                    .border_type(BorderType::Rounded)
                    .title(" Error "),
            )
            .wrap(Wrap { trim: true });

        frame.render_widget(error_block, popup_area);
    }
}

/// Create a centered rectangle.
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

/// Strip HTML tags from a string.
fn strip_html(s: &str) -> String {
    let clean = s
        .replace("<p>", "\n")
        .replace("</p>", "\n")
        .replace("<br>", "\n")
        .replace("<br/>", "\n")
        .replace("<br />", "\n")
        .replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"");

    regex_lite::Regex::new(r"<[^>]+>")
        .map(|re| re.replace_all(&clean, "").to_string())
        .unwrap_or(clean)
}
