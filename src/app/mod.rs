//! Main application module.
//!
//! Orchestrates all components and runs the main event loop.

use std::io::{self, stdout};

use color_eyre::Result;
use crossterm::{
    event::{self, Event, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::prelude::*;
use tracing::info;

use crate::config::Config;
use crate::feed::{FeedItem, FeedManager};
use crate::theme::Theme;
use crate::ui::{FeedListItem, UiState};

/// Main application state.
pub struct App {
    /// Application configuration.
    pub config: Config,

    /// Feed manager.
    pub feeds: FeedManager,

    /// UI state.
    pub ui: UiState,

    /// Theme configuration.
    pub theme: Theme,
}

impl App {
    /// Create a new application instance.
    ///
    /// # Errors
    ///
    /// Returns an error if configuration cannot be loaded or feeds cannot be initialized.
    pub async fn new() -> Result<Self> {
        let config = Config::load()?;
        let theme = config.theme.clone();
        let sync_enabled = config.sync.is_some();
        let feeds = FeedManager::new(&config)?;

        // Check if we have cached data (offline mode)
        let has_cached = feeds.feeds.iter().any(|f| !f.items.is_empty());

        if has_cached {
            info!("Loaded cached articles for offline reading");
        }

        // Don't refresh on startup - let the UI show first, then refresh in background
        // feeds.refresh_all().await;

        let ui = UiState {
            sync_enabled,
            // Mark that we need to refresh feeds
            refreshing: !has_cached,
            ..Default::default()
        };

        let mut app = Self {
            config,
            feeds,
            ui,
            theme,
        };

        // Build initial feed list
        app.rebuild_feed_list();
        app.select_first_feed();

        Ok(app)
    }

    /// Run the main application loop.
    ///
    /// # Errors
    ///
    /// Returns an error if terminal operations fail.
    pub async fn run(&mut self) -> Result<()> {
        // Setup terminal
        enable_raw_mode()?;
        let mut stdout = stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        // Main loop
        let result = self.main_loop(&mut terminal).await;

        // Save cache before exit
        self.feeds.save_cache();

        // Restore terminal
        disable_raw_mode()?;
        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;

        result
    }

    async fn main_loop(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    ) -> Result<()> {
        use std::time::Duration;
        use crossterm::event::poll;

        // Track if we need initial refresh
        let mut needs_initial_refresh = self.ui.refreshing;
        let mut update_check_done = false;

        loop {
            // Render
            terminal.draw(|frame| self.render(frame))?;

            // Process pending update after draw (so "Updating..." is visible)
            if self.ui.pending_update {
                self.process_pending_update();
                // Redraw immediately after update completes
                terminal.draw(|frame| self.render(frame))?;
            }

            // Process pending sync after draw (so "Syncing..." is visible)
            if self.ui.pending_sync {
                self.ui.pending_sync = false;
                match self.run_sync().await {
                    Ok(()) => {}
                    Err(e) => self.ui.set_error(format!("Sync failed: {e}")),
                }
                self.ui.mode = crate::ui::Mode::Normal;
                // Redraw immediately after sync completes
                terminal.draw(|frame| self.render(frame))?;
            }

            // Use poll with timeout to allow background work
            if poll(Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()? {
                    if key.kind == KeyEventKind::Press {
                        match self.handle_key(key.code).await {
                            crate::ui::input::KeyResult::Quit => break,
                            crate::ui::input::KeyResult::Continue => {}
                        }
                    }
                }
            } else {
                // No input - do background work

                // Initial refresh (one feed at a time to stay responsive)
                if needs_initial_refresh {
                    if let Some(idx) = self.feeds.feeds.iter().position(|f| f.last_updated.is_none()) {
                        self.feeds.refresh_feed(idx).await;
                        self.rebuild_feed_list();
                    } else {
                        needs_initial_refresh = false;
                        self.ui.refreshing = false;
                        self.feeds.save_cache();
                    }
                }

                // Check for updates in background (once)
                if !update_check_done && !needs_initial_refresh {
                    update_check_done = true;
                    if let crate::VersionCheck::UpdateAvailable { latest, .. } =
                        crate::check_for_updates_timeout(Duration::from_secs(2)).await
                    {
                        self.ui.update_available = Some(latest);
                    }
                }
            }
        }

        Ok(())
    }

    /// Process a pending update.
    fn process_pending_update(&mut self) {
        self.ui.pending_update = false;

        match crate::run_update(&self.ui.package_manager) {
            Ok(()) => {
                self.ui.update_status = Some("Update complete! Please restart feedo.".to_string());
                self.ui.update_available = None;
            }
            Err(e) => {
                self.ui.update_status = Some(format!("Update failed: {e}"));
            }
        }
        self.ui.mode = crate::ui::Mode::Normal;
    }

    /// Rebuild the flattened feed list for the UI.
    pub fn rebuild_feed_list(&mut self) {
        self.ui.feed_list.clear();

        // Add folders and their feeds
        for (folder_idx, folder) in self.feeds.folders.iter().enumerate() {
            self.ui.feed_list.push(FeedListItem::Folder(folder_idx));

            if folder.expanded {
                for &feed_idx in &folder.feed_indices {
                    self.ui.feed_list.push(FeedListItem::Feed(feed_idx));
                }
            }
        }

        // Add root-level feeds
        for feed_idx in self.feeds.root_feed_indices() {
            self.ui.feed_list.push(FeedListItem::Feed(feed_idx));
        }

        // Sync list state for scrolling
        self.sync_feed_list_state();
    }

    /// Sync `feed_list_state` selection with `feed_list_index`.
    pub fn sync_feed_list_state(&mut self) {
        self.ui.feed_list_state.select(Some(self.ui.feed_list_index));
    }

    /// Sync `items_list_state` selection with `selected_item`.
    pub fn sync_items_list_state(&mut self) {
        self.ui.items_list_state.select(Some(self.ui.selected_item));
    }

    /// Select the first feed in the list.
    pub fn select_first_feed(&mut self) {
        for (idx, item) in self.ui.feed_list.iter().enumerate() {
            if let FeedListItem::Feed(feed_idx) = item {
                self.ui.feed_list_index = idx;
                self.ui.selected_feed = Some(*feed_idx);
                break;
            }
        }
    }

    /// Get items from the currently selected feed.
    #[must_use]
    pub fn current_feed_items(&self) -> &[FeedItem] {
        self.ui
            .selected_feed
            .and_then(|idx| self.feeds.feeds.get(idx))
            .map_or(&[], |f| f.items.as_slice())
    }

    /// Get the currently selected item.
    #[must_use]
    pub fn selected_item(&self) -> Option<&FeedItem> {
        self.ui
            .selected_feed
            .and_then(|idx| self.feeds.feeds.get(idx))
            .and_then(|f| f.items.get(self.ui.selected_item))
    }

    /// Run sync with configured server.
    ///
    /// # Errors
    ///
    /// Returns an error if sync is not configured or the sync operation fails.
    pub async fn run_sync(&mut self) -> Result<()> {
        use crate::sync::SyncManager;

        let sync = self
            .config
            .sync
            .clone()
            .ok_or_else(|| color_eyre::eyre::eyre!("No sync configured"))?;

        let password = sync
            .password
            .as_deref()
            .ok_or_else(|| color_eyre::eyre::eyre!("No password stored"))?;

        self.ui.syncing = true;
        self.ui.sync_status = Some("Connecting...".to_string());

        let manager = SyncManager::connect(&sync.server, &sync.username, password).await?;

        self.ui.sync_status = Some("Syncing subscriptions...".to_string());
        let result = manager
            .full_sync(&mut self.config, &mut self.feeds.cache)
            .await?;

        // Save changes
        self.config.save()?;
        self.feeds.save_cache();

        // Reload feeds if new subscriptions were imported
        if result.feeds_imported > 0 {
            self.feeds = crate::feed::FeedManager::new(&self.config)?;
            self.feeds.refresh_all().await;
            self.rebuild_feed_list();
        }

        self.ui.syncing = false;
        self.ui.sync_status = Some(format!(
            "Synced: +{} feeds, {} read",
            result.feeds_imported,
            result.items_marked_read + result.items_synced_to_server
        ));

        Ok(())
    }
}
