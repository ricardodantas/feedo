//! Input handling.

use crossterm::event::KeyCode;

use crate::app::App;

/// Result of handling a key press.
pub enum KeyResult {
    /// Continue running.
    Continue,
    /// Exit the application.
    Quit,
}

impl App {
    /// Handle a key press event.
    pub async fn handle_key(&mut self, key: KeyCode) -> KeyResult {
        // Clear transient messages
        self.ui.clear_error();
        self.ui.clear_status();

        match self.ui.mode {
            super::Mode::Search => self.handle_search_key(key),
            super::Mode::Normal => self.handle_normal_key(key).await,
        }
    }

    fn handle_search_key(&mut self, key: KeyCode) -> KeyResult {
        match key {
            KeyCode::Esc => {
                self.ui.mode = super::Mode::Normal;
                self.ui.search_query.clear();
                self.ui.search_results.clear();
            }
            KeyCode::Enter => {
                if let Some(&(feed_idx, item_idx)) =
                    self.ui.search_results.get(self.ui.search_selected)
                {
                    self.ui.selected_feed = Some(feed_idx);
                    self.ui.selected_item = item_idx;
                    self.ui.mode = super::Mode::Normal;
                    self.ui.panel = super::Panel::Items;
                    self.ui.search_query.clear();
                    self.ui.search_results.clear();
                }
            }
            KeyCode::Backspace => {
                self.ui.search_query.pop();
                self.perform_search();
            }
            KeyCode::Char(c) => {
                self.ui.search_query.push(c);
                self.perform_search();
            }
            KeyCode::Down | KeyCode::Tab => {
                if !self.ui.search_results.is_empty() {
                    self.ui.search_selected =
                        (self.ui.search_selected + 1) % self.ui.search_results.len();
                }
            }
            KeyCode::Up | KeyCode::BackTab => {
                if !self.ui.search_results.is_empty() {
                    self.ui.search_selected = self
                        .ui
                        .search_selected
                        .checked_sub(1)
                        .unwrap_or(self.ui.search_results.len() - 1);
                }
            }
            _ => {}
        }
        KeyResult::Continue
    }

    async fn handle_normal_key(&mut self, key: KeyCode) -> KeyResult {
        match key {
            // Quit
            KeyCode::Char('q') | KeyCode::Esc => return KeyResult::Quit,

            // Search
            KeyCode::Char('/') => {
                self.ui.mode = super::Mode::Search;
                self.ui.search_query.clear();
                self.ui.search_results.clear();
            }

            // Navigation
            KeyCode::Tab => self.next_panel(),
            KeyCode::Char('j') | KeyCode::Down => self.move_down(),
            KeyCode::Char('k') | KeyCode::Up => self.move_up(),
            KeyCode::Char('l') | KeyCode::Right | KeyCode::Enter => self.select(),
            KeyCode::Char('h') | KeyCode::Left => self.go_back(),
            KeyCode::Char('g') => self.go_to_top(),
            KeyCode::Char('G') => self.go_to_bottom(),

            // Actions
            KeyCode::Char('r') => {
                self.ui.set_status("Refreshing feeds...");
                self.feeds.refresh_all().await;
                self.ui.set_status("Feeds refreshed!");
            }
            KeyCode::Char('o') => self.open_link(),
            KeyCode::Char(' ') => self.toggle_read(),
            KeyCode::Char('a') => self.mark_all_read(),

            _ => {}
        }
        KeyResult::Continue
    }

    const fn next_panel(&mut self) {
        self.ui.panel = match self.ui.panel {
            super::Panel::Feeds => super::Panel::Items,
            super::Panel::Items => {
                if self.ui.show_content {
                    super::Panel::Content
                } else {
                    super::Panel::Feeds
                }
            }
            super::Panel::Content => super::Panel::Feeds,
        };
    }

    fn move_down(&mut self) {
        match self.ui.panel {
            super::Panel::Feeds => {
                if self.ui.feed_list_index < self.ui.feed_list.len().saturating_sub(1) {
                    self.ui.feed_list_index += 1;
                    self.update_selected_feed();
                }
            }
            super::Panel::Items => {
                let item_count = self.current_feed_items().len();
                if self.ui.selected_item < item_count.saturating_sub(1) {
                    self.ui.selected_item += 1;
                }
            }
            super::Panel::Content => {
                self.ui.scroll_offset = self.ui.scroll_offset.saturating_add(1);
            }
        }
    }

    fn move_up(&mut self) {
        match self.ui.panel {
            super::Panel::Feeds => {
                if self.ui.feed_list_index > 0 {
                    self.ui.feed_list_index -= 1;
                    self.update_selected_feed();
                }
            }
            super::Panel::Items => {
                self.ui.selected_item = self.ui.selected_item.saturating_sub(1);
            }
            super::Panel::Content => {
                self.ui.scroll_offset = self.ui.scroll_offset.saturating_sub(1);
            }
        }
    }

    fn go_to_top(&mut self) {
        match self.ui.panel {
            super::Panel::Feeds => {
                self.ui.feed_list_index = 0;
                self.update_selected_feed();
            }
            super::Panel::Items => {
                self.ui.selected_item = 0;
            }
            super::Panel::Content => {
                self.ui.scroll_offset = 0;
            }
        }
    }

    fn go_to_bottom(&mut self) {
        match self.ui.panel {
            super::Panel::Feeds => {
                self.ui.feed_list_index = self.ui.feed_list.len().saturating_sub(1);
                self.update_selected_feed();
            }
            super::Panel::Items => {
                let len = self.current_feed_items().len();
                self.ui.selected_item = len.saturating_sub(1);
            }
            super::Panel::Content => {
                self.ui.scroll_offset = u16::MAX;
            }
        }
    }

    fn select(&mut self) {
        match self.ui.panel {
            super::Panel::Feeds => {
                if let Some(item) = self.ui.feed_list.get(self.ui.feed_list_index).copied() {
                    match item {
                        super::state::FeedListItem::Folder(idx) => {
                            self.feeds.toggle_folder(idx);
                            self.rebuild_feed_list();
                        }
                        super::state::FeedListItem::Feed(idx) => {
                            self.ui.selected_feed = Some(idx);
                            self.ui.selected_item = 0;
                            self.ui.panel = super::Panel::Items;
                        }
                    }
                }
            }
            super::Panel::Items => {
                self.ui.show_content = true;
                self.ui.panel = super::Panel::Content;
                self.ui.scroll_offset = 0;
            }
            super::Panel::Content => {}
        }
    }

    const fn go_back(&mut self) {
        match self.ui.panel {
            super::Panel::Content => {
                self.ui.panel = super::Panel::Items;
            }
            super::Panel::Items => {
                self.ui.panel = super::Panel::Feeds;
            }
            super::Panel::Feeds => {}
        }
    }

    fn open_link(&self) {
        if let Some(item) = self.selected_item() {
            if let Some(link) = &item.link {
                let _ = open::that(link);
            }
        }
    }

    fn toggle_read(&mut self) {
        if matches!(self.ui.panel, super::Panel::Items | super::Panel::Content) {
            if let Some(feed_idx) = self.ui.selected_feed {
                if let Some(feed) = self.feeds.feeds.get_mut(feed_idx) {
                    if let Some(item) = feed.items.get_mut(self.ui.selected_item) {
                        item.toggle_read();
                    }
                }
            }
        }
    }

    fn mark_all_read(&mut self) {
        if let Some(feed_idx) = self.ui.selected_feed {
            if let Some(feed) = self.feeds.feeds.get_mut(feed_idx) {
                feed.mark_all_read();
            }
        }
    }

    fn update_selected_feed(&mut self) {
        if let Some(super::state::FeedListItem::Feed(idx)) =
            self.ui.feed_list.get(self.ui.feed_list_index)
        {
            self.ui.selected_feed = Some(*idx);
            self.ui.selected_item = 0;
        }
    }

    fn perform_search(&mut self) {
        self.ui.search_results.clear();

        if self.ui.search_query.is_empty() {
            return;
        }

        let query = self.ui.search_query.to_lowercase();

        for (feed_idx, feed) in self.feeds.feeds.iter().enumerate() {
            for (item_idx, item) in feed.items.iter().enumerate() {
                let matches = item.title.to_lowercase().contains(&query)
                    || item
                        .summary
                        .as_ref()
                        .is_some_and(|s| s.to_lowercase().contains(&query));

                if matches {
                    self.ui.search_results.push((feed_idx, item_idx));
                }
            }
        }

        self.ui.search_selected = 0;
    }
}
