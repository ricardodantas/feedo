//! UI rendering.

use ratatui::{
    prelude::*,
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, Paragraph, Wrap},
};

use super::state::FeedListItem;
use super::{Mode, Panel};
use crate::app::App;

/// Modern ASCII art logo for Feedo - a cute RSS-eating dog.
pub const LOGO: &str = r"
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
";

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
                Constraint::Length(1), // Title bar
                Constraint::Min(0),    // Content
                Constraint::Length(1), // Status bar
            ])
            .split(area);

        self.render_title_bar(frame, layout[0]);
        self.render_content(frame, layout[1]);
        self.render_status_bar(frame, layout[2]);

        // Overlays
        if self.ui.mode == Mode::Search {
            self.render_search_overlay(frame, area);
        }

        if self.ui.mode == Mode::ThemePicker {
            self.render_theme_picker(frame, area);
        }

        if matches!(
            self.ui.mode,
            Mode::AddFeedUrl | Mode::AddFeedSelect | Mode::AddFeedName | Mode::AddFeedFolder
        ) {
            self.render_add_feed_overlay(frame, area);
        }

        if self.ui.mode == Mode::ConfirmDelete {
            self.render_delete_confirmation(frame, area);
        }

        if self.ui.mode == Mode::ErrorDialog {
            self.render_error_dialog(frame, area);
        }

        if self.ui.mode == Mode::About {
            self.render_about_dialog(frame, area);
        }

        if self.ui.mode == Mode::Share {
            self.render_share_dialog(frame, area);
        }

        if self.ui.mode == Mode::Help {
            self.render_help_dialog(frame, area);
        }

        if let Some(error) = &self.ui.error {
            self.render_error_overlay(frame, area, error);
        }
    }

    fn render_title_bar(&self, frame: &mut Frame, area: Rect) {
        let unread = self.feeds.total_unread_count();
        let title = if unread > 0 {
            format!(" {LOGO_COMPACT} │ {unread} unread")
        } else {
            format!(" {LOGO_COMPACT}")
        };

        let bar = Paragraph::new(title).style(
            Style::default()
                .fg(self.theme.accent())
                .add_modifier(Modifier::BOLD),
        );

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
                        let in_folder = self
                            .feeds
                            .folders
                            .iter()
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
        use std::fmt::Write;

        let is_active = self.ui.panel == Panel::Content;
        let accent = self.theme.accent();
        let muted = self.theme.muted();

        let content = self.selected_item().map_or_else(
            || format!("\n\n    {DOG_ICON}\n\n    Select an article to read"),
            |item| {
                let mut text = format!("  {}\n\n", item.title);

                if let Some(date) = item.published {
                    let _ = write!(text, "  📅 {}\n\n", date.format("%Y-%m-%d %H:%M"));
                }

                if let Some(summary) = &item.summary {
                    // Strip HTML tags
                    let clean = strip_html(summary);
                    text.push_str("  ");
                    text.push_str(&clean.replace('\n', "\n  "));
                }

                if let Some(link) = &item.link {
                    let _ = write!(text, "\n\n  🔗 {link}");
                }

                text
            },
        );

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

    #[allow(clippy::option_if_let_else)]
    fn render_status_bar(&self, frame: &mut Frame, area: Rect) {
        let muted = self.theme.muted();
        let accent = self.theme.accent();

        // Build sync indicator
        let sync_indicator = if self.ui.syncing {
            " ⟳ syncing │"
        } else if self.ui.sync_enabled {
            " ☁ │"
        } else {
            ""
        };

        let status = if let Some(msg) = &self.ui.status {
            Span::styled(format!(" {DOG_ICON} {msg}"), Style::default().fg(accent))
        } else if let Some(msg) = &self.ui.sync_status {
            Span::styled(format!(" ☁ {msg}"), Style::default().fg(accent))
        } else {
            Span::styled(
                format!(
                    "{sync_indicator} n add │ d delete │ r refresh │ / search │ s share │ a mark read │ t theme │ F1 help │ q quit"
                ),
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

    fn render_theme_picker(&self, frame: &mut Frame, area: Rect) {
        use crate::theme::ThemeName;

        let popup_area = centered_rect(50, 70, area);
        frame.render_widget(Clear, popup_area);

        let themes = ThemeName::all();
        let items: Vec<ListItem> = themes
            .iter()
            .enumerate()
            .map(|(i, theme)| {
                let palette = theme.palette();
                let selected = i == self.ui.theme_picker_index;

                // Create color preview squares
                let preview = format!(
                    "  {} {} ",
                    if selected { "▸" } else { " " },
                    theme.display_name()
                );

                let style = if selected {
                    Style::default()
                        .fg(palette.accent)
                        .bg(palette.selection)
                        .bold()
                } else {
                    Style::default().fg(palette.fg)
                };

                ListItem::new(Line::from(vec![
                    Span::styled(preview, style),
                    Span::styled("█", Style::default().fg(palette.accent)),
                    Span::styled("█", Style::default().fg(palette.secondary)),
                    Span::styled("█", Style::default().fg(palette.success)),
                    Span::styled("█", Style::default().fg(palette.warning)),
                ]))
            })
            .collect();

        let accent = self.theme.accent();
        let theme_list = List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(accent))
                .border_type(BorderType::Rounded)
                .title(format!(
                    " 🎨 Select Theme ({}/{}) ",
                    self.ui.theme_picker_index + 1,
                    themes.len()
                ))
                .title_bottom(Line::from(" ↑↓ navigate │ ↵ apply │ Esc cancel ").centered()),
        );

        frame.render_widget(theme_list, popup_area);
    }

    #[allow(clippy::too_many_lines)]
    fn render_add_feed_overlay(&self, frame: &mut Frame, area: Rect) {
        let accent = self.theme.accent();
        let muted = self.theme.muted();
        let popup_area = centered_rect(60, 50, area);

        frame.render_widget(Clear, popup_area);

        match self.ui.mode {
            Mode::AddFeedUrl => {
                // URL input mode
                let layout = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(3), // Input field
                        Constraint::Min(0),    // Instructions
                    ])
                    .split(popup_area);

                let cursor = if self.ui.discovering { "⏳" } else { "│" };
                let input = Paragraph::new(format!(" 🔗 {}{cursor}", self.ui.add_feed_url))
                    .style(Style::default().fg(accent))
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .border_style(Style::default().fg(accent))
                            .border_type(BorderType::Rounded)
                            .title(" ➕ Add Feed "),
                    );
                frame.render_widget(input, layout[0]);

                let help_text = vec![
                    "",
                    "  Enter a URL and press Enter to discover feeds.",
                    "",
                    "  Examples:",
                    "    • https://blog.rust-lang.org",
                    "    • lobste.rs",
                    "    • https://hnrss.org/frontpage",
                    "",
                    "  Feedo will auto-detect RSS/Atom feeds from any URL.",
                ];
                let help = Paragraph::new(help_text.join("\n"))
                    .style(Style::default().fg(muted))
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .border_style(Style::default().fg(muted))
                            .border_type(BorderType::Rounded)
                            .title_bottom(Line::from(" ↵ discover │ Esc cancel ").centered()),
                    );
                frame.render_widget(help, layout[1]);
            }

            Mode::AddFeedSelect => {
                // Feed selection mode (multiple feeds discovered)
                let items: Vec<ListItem> = self
                    .ui
                    .discovered_feeds
                    .iter()
                    .enumerate()
                    .map(|(i, feed)| {
                        let selected = i == self.ui.discovered_feed_index;
                        let title = feed.title.as_deref().unwrap_or("Untitled");
                        let prefix = if selected { "▸" } else { " " };

                        let style = if selected {
                            Style::default().fg(accent).bold()
                        } else {
                            Style::default()
                        };

                        ListItem::new(format!(
                            "  {prefix} {title} ({feed_type})\n      {url}",
                            feed_type = feed.feed_type,
                            url = feed.url
                        ))
                        .style(style)
                    })
                    .collect();

                let list = List::new(items).block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(accent))
                        .border_type(BorderType::Rounded)
                        .title(format!(
                            " 📡 Found {} Feeds ",
                            self.ui.discovered_feeds.len()
                        ))
                        .title_bottom(
                            Line::from(" ↑↓ select │ ↵ confirm │ Esc cancel ").centered(),
                        ),
                );
                frame.render_widget(list, popup_area);
            }

            Mode::AddFeedName => {
                // Name input mode
                let layout = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(5), // Feed info
                        Constraint::Length(3), // Name input
                        Constraint::Min(0),    // Padding
                    ])
                    .split(popup_area);

                // Show selected feed info
                if let Some(feed) = self.ui.discovered_feeds.get(self.ui.discovered_feed_index) {
                    let info = format!("\n  URL: {}\n  Type: {}", feed.url, feed.feed_type);
                    let info_widget = Paragraph::new(info)
                        .style(Style::default().fg(muted))
                        .block(
                            Block::default()
                                .borders(Borders::ALL)
                                .border_style(Style::default().fg(muted))
                                .border_type(BorderType::Rounded)
                                .title(" Feed Info "),
                        );
                    frame.render_widget(info_widget, layout[0]);
                }

                // Name input
                let input = Paragraph::new(format!(" 📝 {}│", self.ui.add_feed_name))
                    .style(Style::default().fg(accent))
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .border_style(Style::default().fg(accent))
                            .border_type(BorderType::Rounded)
                            .title(" Name (optional) ")
                            .title_bottom(Line::from(" ↵ next │ Esc back ").centered()),
                    );
                frame.render_widget(input, layout[1]);
            }

            Mode::AddFeedFolder => {
                // Folder selection mode
                self.render_folder_selection(frame, popup_area);
            }

            _ => {}
        }
    }

    fn render_folder_selection(&self, frame: &mut Frame, area: Rect) {
        let accent = self.theme.accent();
        let muted = self.theme.muted();
        let fg = self.theme.fg();

        if self.ui.creating_new_folder {
            // New folder name input
            let layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3), // Title
                    Constraint::Length(3), // Input
                    Constraint::Min(0),    // Padding
                ])
                .split(area);

            let title = Paragraph::new("\n  Enter a name for the new folder:")
                .style(Style::default().fg(muted))
                .block(Block::default());
            frame.render_widget(title, layout[0]);

            let input = Paragraph::new(format!(" 📁 {}│", self.ui.add_feed_new_folder))
                .style(Style::default().fg(accent))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(accent))
                        .border_type(BorderType::Rounded)
                        .title(" New Folder Name ")
                        .title_bottom(Line::from(" ↵ create │ Esc cancel ").centered()),
                );
            frame.render_widget(input, layout[1]);
        } else {
            // Folder list
            let folder_count = self.config.folders.len();
            let current_index = match self.ui.add_feed_folder_index {
                None => 0,
                Some(usize::MAX) => folder_count + 1,
                Some(i) => i + 1,
            };

            let mut items: Vec<ListItem> = Vec::new();

            // Root option (no folder)
            let selected = current_index == 0;
            let prefix = if selected { "▸" } else { " " };
            let style = if selected {
                Style::default().fg(accent).bold()
            } else {
                Style::default().fg(fg)
            };
            items.push(ListItem::new(format!("  {prefix} 🏠 Root (no folder)")).style(style));

            // Existing folders
            for (i, folder) in self.config.folders.iter().enumerate() {
                let selected = current_index == i + 1;
                let prefix = if selected { "▸" } else { " " };
                let icon = folder.icon.as_deref().unwrap_or("📁");
                let style = if selected {
                    Style::default().fg(accent).bold()
                } else {
                    Style::default().fg(fg)
                };
                items
                    .push(ListItem::new(format!("  {prefix} {icon} {}", folder.name)).style(style));
            }

            // New folder option
            let selected = current_index == folder_count + 1;
            let prefix = if selected { "▸" } else { " " };
            let style = if selected {
                Style::default().fg(accent).bold()
            } else {
                Style::default().fg(muted).italic()
            };
            items.push(ListItem::new(format!("  {prefix} ➕ Create new folder...")).style(style));

            let list = List::new(items).block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(accent))
                    .border_type(BorderType::Rounded)
                    .title(" 📁 Select Folder ")
                    .title_bottom(Line::from(" ↑↓ select │ ↵ confirm │ Esc back ").centered()),
            );
            frame.render_widget(list, area);
        }
    }

    #[allow(clippy::option_if_let_else)]
    #[allow(clippy::or_fun_call)]
    fn render_delete_confirmation(&self, frame: &mut Frame, area: Rect) {
        let accent = self.theme.accent();
        let muted = self.theme.muted();
        let popup_area = centered_rect(50, 25, area);

        frame.render_widget(Clear, popup_area);

        // Determine what we're deleting (folder or feed)
        let (item_name, item_type, extra_info) =
            if let Some(folder_idx) = self.ui.pending_delete_folder {
                let folder = self.config.folders.get(folder_idx);
                let name = folder.map_or("this folder", |f| f.name.as_str());
                let feed_count = folder.map_or(0, |f| f.feeds.len());
                (
                    name.to_string(),
                    "folder",
                    format!("This will remove the folder and all {feed_count} feeds inside."),
                )
            } else {
                let feed_name = self
                    .ui
                    .pending_delete_feed
                    .and_then(|idx| self.feeds.feeds.get(idx))
                    .map_or("this feed".to_string(), |f| f.name.clone());
                (
                    feed_name,
                    "feed",
                    "This will remove the feed from your subscriptions.".to_string(),
                )
            };

        let text = vec![
            Line::from(""),
            Line::from(Span::styled(
                format!("Delete {item_type} \"{item_name}\"?"),
                Style::default().fg(accent).bold(),
            )),
            Line::from(""),
            Line::from(Span::styled(extra_info, Style::default().fg(muted))),
            Line::from(""),
            Line::from(vec![
                Span::styled(" [Y] ", Style::default().fg(accent).bold()),
                Span::raw("Yes, delete"),
                Span::raw("    "),
                Span::styled(" [N] ", Style::default().fg(muted)),
                Span::raw("Cancel"),
            ]),
        ];

        let paragraph = Paragraph::new(text)
            .alignment(ratatui::layout::Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(accent))
                    .title(" ⚠️  Confirm Delete ")
                    .title_style(Style::default().fg(accent).bold()),
            );

        frame.render_widget(paragraph, popup_area);
    }

    fn render_error_dialog(&self, frame: &mut Frame, area: Rect) {
        let accent = self.theme.accent();
        let muted = self.theme.muted();
        let error_color = Color::Red;
        let popup_area = centered_rect(70, 50, area);

        frame.render_widget(Clear, popup_area);

        let (error_msg, context) = self
            .ui
            .error_dialog
            .as_ref()
            .map_or(("Unknown error", None), |(e, c)| (e.as_str(), c.as_deref()));

        // Truncate error message if too long
        let max_error_len = (popup_area.width as usize).saturating_sub(6);
        let truncated_error: String = if error_msg.len() > max_error_len {
            format!("{}…", &error_msg[..max_error_len.saturating_sub(1)])
        } else {
            error_msg.to_string()
        };

        let mut lines = vec![
            Line::from(""),
            Line::from(Span::styled(
                "Oops! Something went wrong 😿",
                Style::default().fg(error_color).bold(),
            )),
            Line::from(""),
            Line::from(Span::styled(truncated_error, Style::default().fg(muted))),
        ];

        if let Some(ctx) = context {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                format!("Context: {ctx}"),
                Style::default().fg(muted).italic(),
            )));
        }

        lines.extend([
            Line::from(""),
            Line::from(Span::styled(
                "You can report this issue on GitHub to help us fix it.",
                Style::default().fg(muted),
            )),
            Line::from(""),
            Line::from(vec![
                Span::styled(" [R] ", Style::default().fg(accent).bold()),
                Span::raw("Report on GitHub"),
                Span::raw("    "),
                Span::styled(" [C/Esc] ", Style::default().fg(muted)),
                Span::raw("Close"),
            ]),
        ]);

        let paragraph = Paragraph::new(lines)
            .alignment(ratatui::layout::Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(error_color))
                    .title(" ❌ Error ")
                    .title_style(Style::default().fg(error_color).bold()),
            );

        frame.render_widget(paragraph, popup_area);
    }

    fn render_about_dialog(&self, frame: &mut Frame, area: Rect) {
        let accent = self.theme.accent();
        let muted = self.theme.muted();
        let fg = self.theme.fg();
        let popup_area = centered_rect(60, 60, area);

        frame.render_widget(Clear, popup_area);

        let version = crate::error_report::VERSION;
        let repo = crate::error_report::REPO_URL;

        let logo = [
            "    ██████╗██████╗██████╗██████╗  ██████╗",
            "    ██╔═══╝██╔═══╝██╔═══╝██╔══██╗██╔═══██╗",
            "    █████╗ █████╗ █████╗ ██║  ██║██║   ██║",
            "    ██╔══╝ ██╔══╝ ██╔══╝ ██║  ██║██║   ██║",
            "    ██║    ██████╗██████╗██████╔╝╚██████╔╝",
            "    ╚═╝    ╚═════╝╚═════╝╚═════╝  ╚═════╝",
        ];

        let mut lines: Vec<Line> = logo
            .iter()
            .map(|line| Line::from(Span::styled(*line, Style::default().fg(accent))))
            .collect();

        lines.extend([
            Line::from(""),
            Line::from(Span::styled(
                "(◕ᴥ◕) Your terminal RSS companion",
                Style::default().fg(fg).italic(),
            )),
            Line::from(""),
            Line::from(vec![
                Span::styled("Version: ", Style::default().fg(muted)),
                Span::styled(version, Style::default().fg(accent).bold()),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Author: ", Style::default().fg(muted)),
                Span::styled("Ricardo Dantas", Style::default().fg(fg)),
            ]),
            Line::from(vec![
                Span::styled("License: ", Style::default().fg(muted)),
                Span::styled("MIT", Style::default().fg(fg)),
            ]),
            Line::from(vec![
                Span::styled("Repo: ", Style::default().fg(muted)),
                Span::styled(repo, Style::default().fg(accent)),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                "Built with Rust 🦀 + Ratatui",
                Style::default().fg(muted).italic(),
            )),
            Line::from(""),
            Line::from(vec![
                Span::styled(" [G] ", Style::default().fg(accent).bold()),
                Span::raw("Open GitHub"),
                Span::raw("    "),
                Span::styled(" [Esc] ", Style::default().fg(muted)),
                Span::raw("Close"),
            ]),
        ]);

        let paragraph = Paragraph::new(lines)
            .alignment(ratatui::layout::Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(accent))
                    .title(" 🐕 About Feedo ")
                    .title_style(Style::default().fg(accent).bold()),
            );

        frame.render_widget(paragraph, popup_area);
    }

    /// Render help/hotkeys dialog overlay.
    #[allow(clippy::too_many_lines)]
    fn render_help_dialog(&self, frame: &mut Frame, area: Rect) {
        let accent = self.theme.accent();
        let muted = self.theme.muted();
        let fg = self.theme.fg();
        let popup_area = centered_rect(65, 80, area);

        frame.render_widget(Clear, popup_area);

        let key_style = Style::default().fg(accent).bold();
        let desc_style = Style::default().fg(fg);
        let section_style = Style::default().fg(muted).italic();

        let lines = vec![
            Line::from(Span::styled("── Navigation ──", section_style)),
            Line::from(vec![
                Span::styled("  j/↓    ", key_style),
                Span::styled("Move down", desc_style),
            ]),
            Line::from(vec![
                Span::styled("  k/↑    ", key_style),
                Span::styled("Move up", desc_style),
            ]),
            Line::from(vec![
                Span::styled("  Tab    ", key_style),
                Span::styled("Switch panel (Feeds → Items → Content)", desc_style),
            ]),
            Line::from(vec![
                Span::styled("  g      ", key_style),
                Span::styled("Go to top", desc_style),
            ]),
            Line::from(vec![
                Span::styled("  G      ", key_style),
                Span::styled("Go to bottom", desc_style),
            ]),
            Line::from(vec![
                Span::styled("  Enter  ", key_style),
                Span::styled("Open link / expand folder", desc_style),
            ]),
            Line::from(vec![
                Span::styled("  v      ", key_style),
                Span::styled("Toggle content panel", desc_style),
            ]),
            Line::from(""),
            Line::from(Span::styled("── Feeds ──", section_style)),
            Line::from(vec![
                Span::styled("  n      ", key_style),
                Span::styled("Add new feed", desc_style),
            ]),
            Line::from(vec![
                Span::styled("  d      ", key_style),
                Span::styled("Delete feed/folder", desc_style),
            ]),
            Line::from(vec![
                Span::styled("  r      ", key_style),
                Span::styled("Refresh feeds", desc_style),
            ]),
            Line::from(vec![
                Span::styled("  R      ", key_style),
                Span::styled("Refresh all feeds", desc_style),
            ]),
            Line::from(""),
            Line::from(Span::styled("── Reading ──", section_style)),
            Line::from(vec![
                Span::styled("  Space  ", key_style),
                Span::styled("Toggle read/unread", desc_style),
            ]),
            Line::from(vec![
                Span::styled("  a      ", key_style),
                Span::styled("Mark all read in current feed", desc_style),
            ]),
            Line::from(vec![
                Span::styled("  s      ", key_style),
                Span::styled("Share article", desc_style),
            ]),
            Line::from(""),
            Line::from(Span::styled("── Search & Sync ──", section_style)),
            Line::from(vec![
                Span::styled("  /      ", key_style),
                Span::styled("Search articles", desc_style),
            ]),
            Line::from(vec![
                Span::styled("  S      ", key_style),
                Span::styled("Sync with cloud (if configured)", desc_style),
            ]),
            Line::from(""),
            Line::from(Span::styled("── Other ──", section_style)),
            Line::from(vec![
                Span::styled("  t      ", key_style),
                Span::styled("Change theme", desc_style),
            ]),
            Line::from(vec![
                Span::styled("  F1     ", key_style),
                Span::styled("Show this help", desc_style),
            ]),
            Line::from(vec![
                Span::styled("  ?      ", key_style),
                Span::styled("About Feedo", desc_style),
            ]),
            Line::from(vec![
                Span::styled("  q      ", key_style),
                Span::styled("Quit", desc_style),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled(" [Esc] ", Style::default().fg(muted)),
                Span::raw("Close"),
            ]),
        ];

        let paragraph = Paragraph::new(lines).block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(accent))
                .title(" ⌨️  Keyboard Shortcuts ")
                .title_style(Style::default().fg(accent).bold()),
        );

        frame.render_widget(paragraph, popup_area);
    }

    /// Render share dialog overlay.
    fn render_share_dialog(&self, frame: &mut Frame, area: Rect) {
        let accent = self.theme.accent();
        let popup_area = centered_rect(40, 35, area);

        // Clear background
        frame.render_widget(Clear, popup_area);

        let platforms = ["  X (Twitter)", "  Mastodon", "  Bluesky"];
        let selected = self.ui.share_platform_index;

        let items: Vec<Line> = platforms
            .iter()
            .enumerate()
            .map(|(i, name)| {
                let style = if i == selected {
                    Style::default().fg(accent).bold()
                } else {
                    Style::default().fg(self.theme.fg())
                };
                let prefix = if i == selected { "▸ " } else { "  " };
                Line::from(format!("{prefix}{name}")).style(style)
            })
            .collect();

        let help = Line::from(vec![
            Span::styled("↑↓", Style::default().fg(accent)),
            Span::raw(" nav  "),
            Span::styled("Enter", Style::default().fg(accent)),
            Span::raw(" share  "),
            Span::styled("x/m/b", Style::default().fg(accent)),
            Span::raw(" quick  "),
            Span::styled("Esc", Style::default().fg(accent)),
            Span::raw(" cancel"),
        ])
        .style(Style::default().fg(self.theme.muted()));

        let mut lines = vec![Line::from(""), Line::from("Select platform to share:")];
        lines.push(Line::from(""));
        lines.extend(items);
        lines.push(Line::from(""));
        lines.push(help);

        let paragraph = Paragraph::new(lines)
            .alignment(ratatui::layout::Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(accent))
                    .title(" 📤 Share Article ")
                    .title_style(Style::default().fg(accent).bold()),
            );

        frame.render_widget(paragraph, popup_area);
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
