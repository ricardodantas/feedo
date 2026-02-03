//! Input handling.

use crossterm::event::KeyCode;

use crate::app::App;
use crate::config::FeedConfig;
use crate::feed::FeedDiscovery;

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
            super::Mode::ThemePicker => self.handle_theme_picker_key(key),
            super::Mode::AddFeedUrl => self.handle_add_feed_url_key(key).await,
            super::Mode::AddFeedSelect => self.handle_add_feed_select_key(key),
            super::Mode::AddFeedName => self.handle_add_feed_name_key(key),
            super::Mode::AddFeedFolder => self.handle_add_feed_folder_key(key).await,
            super::Mode::ConfirmDelete => self.handle_confirm_delete_key(key),
            super::Mode::ErrorDialog => self.handle_error_dialog_key(key),
            super::Mode::About => self.handle_about_key(key),
            super::Mode::Share => self.handle_share_key(key),
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

            // Theme picker
            KeyCode::Char('t') => {
                self.ui.mode = super::Mode::ThemePicker;
                // Set picker index to current theme
                let current = self.theme.name;
                self.ui.theme_picker_index = crate::theme::ThemeName::all()
                    .iter()
                    .position(|&t| t == current)
                    .unwrap_or(0);
            }

            // Add feed
            KeyCode::Char('n') => {
                self.ui.reset_add_feed();
                self.ui.mode = super::Mode::AddFeedUrl;
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
            KeyCode::Char('s') => self.open_share_dialog(),
            KeyCode::Char(' ') => self.toggle_read(),
            KeyCode::Char('a') => self.mark_all_read(),

            // Delete feed
            KeyCode::Char('d') | KeyCode::Delete => self.delete_selected_feed(),

            // About dialog
            KeyCode::Char('?') => {
                self.ui.mode = super::Mode::About;
            }

            _ => {}
        }
        KeyResult::Continue
    }

    fn handle_theme_picker_key(&mut self, key: KeyCode) -> KeyResult {
        let themes = crate::theme::ThemeName::all();

        match key {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.ui.mode = super::Mode::Normal;
            }
            KeyCode::Enter => {
                // Apply selected theme
                let selected_theme = themes[self.ui.theme_picker_index];
                self.theme = crate::Theme::new(selected_theme);
                self.config.theme = self.theme.clone();

                // Save config
                if let Err(e) = self.config.save() {
                    self.ui.set_error(format!("Failed to save config: {e}"));
                } else {
                    self.ui
                        .set_status(format!("Theme set to {}", selected_theme.display_name()));
                }

                self.ui.mode = super::Mode::Normal;
            }
            KeyCode::Char('j') | KeyCode::Down => {
                self.ui.theme_picker_index = (self.ui.theme_picker_index + 1) % themes.len();
                // Live preview
                self.theme = crate::Theme::new(themes[self.ui.theme_picker_index]);
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.ui.theme_picker_index = self
                    .ui
                    .theme_picker_index
                    .checked_sub(1)
                    .unwrap_or(themes.len() - 1);
                // Live preview
                self.theme = crate::Theme::new(themes[self.ui.theme_picker_index]);
            }
            _ => {}
        }
        KeyResult::Continue
    }

    async fn handle_add_feed_url_key(&mut self, key: KeyCode) -> KeyResult {
        match key {
            KeyCode::Esc => {
                self.ui.reset_add_feed();
                self.ui.mode = super::Mode::Normal;
            }
            KeyCode::Enter => {
                if !self.ui.add_feed_url.is_empty() {
                    self.discover_feeds().await;
                }
            }
            KeyCode::Backspace => {
                self.ui.add_feed_url.pop();
            }
            KeyCode::Char(c) => {
                self.ui.add_feed_url.push(c);
            }
            _ => {}
        }
        KeyResult::Continue
    }

    fn handle_add_feed_select_key(&mut self, key: KeyCode) -> KeyResult {
        match key {
            KeyCode::Esc => {
                self.ui.reset_add_feed();
                self.ui.mode = super::Mode::Normal;
            }
            KeyCode::Enter => {
                // Move to name input with suggested name
                if let Some(feed) = self.ui.discovered_feeds.get(self.ui.discovered_feed_index) {
                    self.ui.add_feed_name = feed.title.clone().unwrap_or_default();
                }
                self.ui.mode = super::Mode::AddFeedName;
            }
            KeyCode::Char('j') | KeyCode::Down => {
                if !self.ui.discovered_feeds.is_empty() {
                    self.ui.discovered_feed_index =
                        (self.ui.discovered_feed_index + 1) % self.ui.discovered_feeds.len();
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if !self.ui.discovered_feeds.is_empty() {
                    self.ui.discovered_feed_index = self
                        .ui
                        .discovered_feed_index
                        .checked_sub(1)
                        .unwrap_or(self.ui.discovered_feeds.len() - 1);
                }
            }
            _ => {}
        }
        KeyResult::Continue
    }

    fn handle_add_feed_name_key(&mut self, key: KeyCode) -> KeyResult {
        match key {
            KeyCode::Esc => {
                // Go back to feed selection
                self.ui.mode = super::Mode::AddFeedSelect;
            }
            KeyCode::Enter => {
                // Go to folder selection
                self.ui.mode = super::Mode::AddFeedFolder;
            }
            KeyCode::Backspace => {
                self.ui.add_feed_name.pop();
            }
            KeyCode::Char(c) => {
                self.ui.add_feed_name.push(c);
            }
            _ => {}
        }
        KeyResult::Continue
    }

    /// Handle keys in folder selection mode.
    async fn handle_add_feed_folder_key(&mut self, key: KeyCode) -> KeyResult {
        let folder_count = self.config.folders.len();
        // Options: None (root), Some(0..folder_count-1) for existing folders, or "new folder"
        // We represent this as: 0 = root, 1..=folder_count = existing folders, folder_count+1 = new folder
        let total_options = folder_count + 2; // root + folders + "new folder"

        if self.ui.creating_new_folder {
            // Creating a new folder - text input mode
            match key {
                KeyCode::Esc => {
                    self.ui.creating_new_folder = false;
                    self.ui.add_feed_new_folder.clear();
                }
                KeyCode::Enter => {
                    if !self.ui.add_feed_new_folder.is_empty() {
                        // Create the folder and select it
                        let new_folder = crate::config::FolderConfig {
                            name: self.ui.add_feed_new_folder.clone(),
                            icon: Some("📁".to_string()),
                            expanded: true,
                            feeds: vec![],
                        };
                        self.config.folders.push(new_folder);
                        self.ui.add_feed_folder_index = Some(self.config.folders.len() - 1);
                        self.ui.creating_new_folder = false;
                        self.ui.add_feed_new_folder.clear();
                        // Now add the feed
                        self.add_discovered_feed().await;
                    }
                }
                KeyCode::Backspace => {
                    self.ui.add_feed_new_folder.pop();
                }
                KeyCode::Char(c) => {
                    self.ui.add_feed_new_folder.push(c);
                }
                _ => {}
            }
        } else {
            // Folder selection mode
            let current_index = self.ui.add_feed_folder_index.map_or(0, |i| i + 1);

            match key {
                KeyCode::Esc => {
                    // Go back to name input
                    self.ui.mode = super::Mode::AddFeedName;
                }
                KeyCode::Char('j') | KeyCode::Down => {
                    let new_index = (current_index + 1) % total_options;
                    self.ui.add_feed_folder_index = if new_index == 0 {
                        None
                    } else if new_index <= folder_count {
                        Some(new_index - 1)
                    } else {
                        // "New folder" option - keep as last folder + 1 marker
                        Some(usize::MAX)
                    };
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    let new_index = if current_index == 0 {
                        total_options - 1
                    } else {
                        current_index - 1
                    };
                    self.ui.add_feed_folder_index = if new_index == 0 {
                        None
                    } else if new_index <= folder_count {
                        Some(new_index - 1)
                    } else {
                        Some(usize::MAX)
                    };
                }
                KeyCode::Enter => {
                    if self.ui.add_feed_folder_index == Some(usize::MAX) {
                        // "New folder" selected - start creating
                        self.ui.creating_new_folder = true;
                        self.ui.add_feed_new_folder.clear();
                    } else {
                        // Add the feed to selected folder (or root)
                        self.add_discovered_feed().await;
                    }
                }
                _ => {}
            }
        }
        KeyResult::Continue
    }

    /// Discover feeds from the entered URL.
    async fn discover_feeds(&mut self) {
        self.ui.discovering = true;

        match FeedDiscovery::new() {
            Ok(discovery) => {
                match discovery.discover(&self.ui.add_feed_url).await {
                    Ok(feeds) => {
                        self.ui.discovered_feeds = feeds;
                        self.ui.discovered_feed_index = 0;

                        if self.ui.discovered_feeds.len() == 1 {
                            // Only one feed found, go straight to name input
                            if let Some(feed) = self.ui.discovered_feeds.first() {
                                self.ui.add_feed_name = feed.title.clone().unwrap_or_default();
                            }
                            self.ui.mode = super::Mode::AddFeedName;
                        } else {
                            // Multiple feeds, let user choose
                            self.ui.mode = super::Mode::AddFeedSelect;
                        }
                    }
                    Err(e) => {
                        self.ui.show_error_dialog(
                            format!("No feeds found at this URL: {e}"),
                            Some(format!("URL: {}", self.ui.add_feed_url)),
                        );
                    }
                }
            }
            Err(e) => {
                self.ui.show_error_dialog(
                    format!("Failed to discover feeds: {e}"),
                    Some(format!("URL: {}", self.ui.add_feed_url)),
                );
            }
        }

        self.ui.discovering = false;
    }

    /// Add the selected discovered feed.
    async fn add_discovered_feed(&mut self) {
        let Some(discovered) = self.ui.discovered_feeds.get(self.ui.discovered_feed_index) else {
            self.ui.set_error("No feed selected");
            return;
        };

        let name = if self.ui.add_feed_name.is_empty() {
            discovered
                .title
                .clone()
                .unwrap_or_else(|| "Untitled Feed".to_string())
        } else {
            self.ui.add_feed_name.clone()
        };

        let url = discovered.url.clone();

        let feed_config = FeedConfig {
            name: name.clone(),
            url: url.clone(),
        };

        // Add to folder if one is selected, otherwise add to root feeds
        match self.ui.add_feed_folder_index {
            Some(folder_idx) if folder_idx != usize::MAX => {
                // Add to existing folder
                if let Some(folder) = self.config.folders.get_mut(folder_idx) {
                    folder.feeds.push(feed_config);
                } else {
                    self.config.feeds.push(feed_config);
                }
            }
            _ => {
                // Add to root (no folder) or usize::MAX case
                self.config.feeds.push(feed_config);
            }
        }

        // Save config
        if let Err(e) = self.config.save() {
            self.ui.set_error(format!("Failed to save: {e}"));
            return;
        }

        // Add to feed manager
        let feed_idx = self.feeds.feeds.len();
        self.feeds
            .feeds
            .push(crate::feed::Feed::new(name.clone(), url));

        // Refresh the new feed
        self.feeds.refresh_feed(feed_idx).await;

        // Update UI
        self.rebuild_feed_list();
        self.ui.set_status(format!("Added: {name}"));
        self.ui.reset_add_feed();
        self.ui.mode = super::Mode::Normal;
    }

    /// Prompt for delete confirmation.
    fn delete_selected_feed(&mut self) {
        // Only delete if we're in the Feeds panel and have a feed selected
        if !matches!(self.ui.panel, super::Panel::Feeds) {
            return;
        }

        let Some(super::state::FeedListItem::Feed(feed_idx)) =
            self.ui.feed_list.get(self.ui.feed_list_index).copied()
        else {
            return;
        };

        // Store the feed index and switch to confirmation mode
        self.ui.pending_delete_feed = Some(feed_idx);
        self.ui.mode = super::Mode::ConfirmDelete;
    }

    /// Handle keys in delete confirmation mode.
    fn handle_confirm_delete_key(&mut self, key: KeyCode) -> KeyResult {
        match key {
            KeyCode::Char('y' | 'Y') => {
                self.perform_delete();
            }
            KeyCode::Char('n' | 'N') | KeyCode::Esc => {
                self.ui.reset_delete();
                self.ui.mode = super::Mode::Normal;
            }
            _ => {}
        }
        KeyResult::Continue
    }

    /// Handle keys in error dialog mode.
    fn handle_error_dialog_key(&mut self, key: KeyCode) -> KeyResult {
        match key {
            KeyCode::Char('r' | 'R') => {
                // Report bug on GitHub
                if let Some((error, context)) = &self.ui.error_dialog {
                    let _ = crate::error_report::open_issue(error, context.as_deref());
                }
                self.ui.close_error_dialog();
            }
            KeyCode::Esc | KeyCode::Enter | KeyCode::Char('c' | 'C') => {
                self.ui.close_error_dialog();
            }
            _ => {}
        }
        KeyResult::Continue
    }

    /// Handle keys in about dialog mode.
    fn handle_about_key(&mut self, key: KeyCode) -> KeyResult {
        match key {
            KeyCode::Esc | KeyCode::Enter | KeyCode::Char('q') => {
                self.ui.mode = super::Mode::Normal;
            }
            KeyCode::Char('g' | 'G') => {
                // Open GitHub repo
                let _ = open::that(crate::error_report::REPO_URL);
            }
            _ => {}
        }
        KeyResult::Continue
    }

    /// Actually delete the feed after confirmation.
    fn perform_delete(&mut self) {
        let Some(feed_idx) = self.ui.pending_delete_feed else {
            self.ui.mode = super::Mode::Normal;
            return;
        };

        // Get the feed name for status message
        let feed_name = self
            .feeds
            .feeds
            .get(feed_idx)
            .map(|f| f.name.clone())
            .unwrap_or_default();

        // Remove from config - check folders first, then root feeds
        let mut found = false;
        for folder in &mut self.config.folders {
            if let Some(pos) = folder.feeds.iter().position(|f| {
                self.feeds
                    .feeds
                    .get(feed_idx)
                    .is_some_and(|feed| feed.url == f.url)
            }) {
                folder.feeds.remove(pos);
                found = true;
                break;
            }
        }

        if !found {
            if let Some(pos) = self.config.feeds.iter().position(|f| {
                self.feeds
                    .feeds
                    .get(feed_idx)
                    .is_some_and(|feed| feed.url == f.url)
            }) {
                self.config.feeds.remove(pos);
            }
        }

        // Save config
        if let Err(e) = self.config.save() {
            self.ui.set_error(format!("Failed to save: {e}"));
            self.ui.reset_delete();
            self.ui.mode = super::Mode::Normal;
            return;
        }

        // Reload feed manager from config (simplest way to keep indices consistent)
        if let Ok(new_feeds) = crate::feed::FeedManager::new(&self.config) {
            self.feeds = new_feeds;
        }

        self.rebuild_feed_list();
        self.select_first_feed();
        self.ui.set_status(format!("Deleted: {feed_name}"));
        self.ui.reset_delete();
        self.ui.mode = super::Mode::Normal;
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
                // Mark item as read when opening
                self.mark_current_read();
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

    fn open_link(&mut self) {
        if let Some(item) = self.selected_item() {
            if let Some(link) = &item.link {
                if let Err(e) = open::that(link) {
                    self.ui.show_error_dialog(
                        "Failed to open browser",
                        Some(format!("Error: {e}\n\nURL: {link}")),
                    );
                    return;
                }
            } else {
                self.ui.show_error_dialog(
                    "No link available",
                    Some("This article doesn't have a URL to open.".to_string()),
                );
                return;
            }
        }
        // Mark as read when opening in browser
        self.mark_current_read();
    }

    fn toggle_read(&mut self) {
        if matches!(self.ui.panel, super::Panel::Items | super::Panel::Content) {
            if let Some(feed_idx) = self.ui.selected_feed {
                if let Some(feed) = self.feeds.feeds.get_mut(feed_idx) {
                    if let Some(item) = feed.items.get_mut(self.ui.selected_item) {
                        item.toggle_read();
                        // Persist to cache
                        let feed_url = feed.url.clone();
                        let item_id = item.id.clone();
                        let is_read = item.read;
                        self.feeds.cache.set_item_read(&feed_url, &item_id, is_read);
                        let _ = self.feeds.cache.save();
                    }
                }
            }
        }
    }

    fn mark_current_read(&mut self) {
        if let Some(feed_idx) = self.ui.selected_feed {
            if let Some(feed) = self.feeds.feeds.get_mut(feed_idx) {
                if let Some(item) = feed.items.get_mut(self.ui.selected_item) {
                    item.mark_read();
                    // Persist to cache
                    let feed_url = feed.url.clone();
                    let item_id = item.id.clone();
                    self.feeds.cache.set_item_read(&feed_url, &item_id, true);
                    let _ = self.feeds.cache.save();
                }
            }
        }
    }

    fn mark_all_read(&mut self) {
        if let Some(feed_idx) = self.ui.selected_feed {
            if let Some(feed) = self.feeds.feeds.get_mut(feed_idx) {
                feed.mark_all_read();
                // Persist to cache
                let feed_url = feed.url.clone();
                self.feeds.cache.mark_feed_read(&feed_url);
                let _ = self.feeds.cache.save();
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

    /// Open the share dialog for the current item.
    fn open_share_dialog(&mut self) {
        // Only allow sharing when an item is selected
        if matches!(self.ui.panel, super::Panel::Items | super::Panel::Content)
            && self.selected_item().is_some()
        {
            self.ui.share_platform_index = 0;
            self.ui.mode = super::Mode::Share;
        }
    }

    /// Handle keys in share mode.
    fn handle_share_key(&mut self, key: KeyCode) -> KeyResult {
        const PLATFORM_COUNT: usize = 3;

        match key {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.ui.mode = super::Mode::Normal;
            }
            KeyCode::Char('j') | KeyCode::Down => {
                self.ui.share_platform_index =
                    (self.ui.share_platform_index + 1) % PLATFORM_COUNT;
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.ui.share_platform_index = self
                    .ui
                    .share_platform_index
                    .checked_sub(1)
                    .unwrap_or(PLATFORM_COUNT - 1);
            }
            KeyCode::Enter => {
                self.share_to_platform();
                self.ui.mode = super::Mode::Normal;
            }
            // Quick keys for direct sharing
            KeyCode::Char('x' | 'X') => {
                self.ui.share_platform_index = 0;
                self.share_to_platform();
                self.ui.mode = super::Mode::Normal;
            }
            KeyCode::Char('m' | 'M') => {
                self.ui.share_platform_index = 1;
                self.share_to_platform();
                self.ui.mode = super::Mode::Normal;
            }
            KeyCode::Char('b' | 'B') => {
                self.ui.share_platform_index = 2;
                self.share_to_platform();
                self.ui.mode = super::Mode::Normal;
            }
            _ => {}
        }
        KeyResult::Continue
    }

    /// Share the current item to the selected platform.
    fn share_to_platform(&mut self) {
        let Some(item) = self.selected_item() else {
            return;
        };

        let Some(link) = item.link.clone() else {
            self.ui.show_error_dialog(
                "No link available",
                Some("This article doesn't have a URL to share.".to_string()),
            );
            return;
        };

        let title = item.title.clone();
        let text = format!("{title} {link}");
        let encoded_text = urlencoding::encode(&text);

        let share_url = match self.ui.share_platform_index {
            0 => {
                // X (Twitter)
                format!("https://twitter.com/intent/tweet?text={encoded_text}")
            }
            1 => {
                // Mastodon (uses share page that works with any instance)
                format!("https://mastodonshare.com/?text={encoded_text}")
            }
            2 => {
                // Bluesky
                format!("https://bsky.app/intent/compose?text={encoded_text}")
            }
            _ => return,
        };

        if let Err(e) = open::that(&share_url) {
            self.ui.show_error_dialog(
                "Failed to open browser",
                Some(format!("Error: {e}\n\nShare URL: {share_url}")),
            );
        } else {
            let platform = match self.ui.share_platform_index {
                0 => "X",
                1 => "Mastodon",
                2 => "Bluesky",
                _ => "Unknown",
            };
            self.ui.set_status(format!("Sharing to {platform}..."));
        }
    }
}
