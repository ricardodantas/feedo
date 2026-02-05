//! UI state management.

use crate::feed::DiscoveredFeed;
use ratatui::widgets::ListState;

/// Active panel in the UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Panel {
    /// Feed list panel (left).
    #[default]
    Feeds,
    /// Article list panel (middle).
    Items,
    /// Content preview panel (right).
    Content,
}

/// Input mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Mode {
    /// Normal navigation mode.
    #[default]
    Normal,
    /// Search input mode.
    Search,
    /// Theme picker mode.
    ThemePicker,
    /// Add feed mode - entering URL.
    AddFeedUrl,
    /// Add feed mode - selecting discovered feed.
    AddFeedSelect,
    /// Add feed mode - entering custom name.
    AddFeedName,
    /// Confirm delete mode.
    ConfirmDelete,
    /// Error dialog mode.
    ErrorDialog,
    /// About dialog mode.
    About,
    /// Add feed mode - selecting folder.
    AddFeedFolder,
    /// Share article mode.
    Share,
    /// Syncing in progress.
    Syncing,
    /// Help/hotkeys dialog mode.
    Help,
    /// Update confirmation dialog.
    UpdateConfirm,
    /// Update in progress.
    Updating,
}

/// Item in the feed list (can be folder or feed).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeedListItem {
    /// A folder (expandable).
    Folder(usize),
    /// A feed.
    Feed(usize),
}

/// Complete UI state.
#[derive(Debug)]
#[allow(clippy::struct_excessive_bools)]
pub struct UiState {
    /// Currently active panel.
    pub panel: Panel,

    /// Current input mode.
    pub mode: Mode,

    /// Flattened list of visible feed items.
    pub feed_list: Vec<FeedListItem>,

    /// Selected index in feed list.
    pub feed_list_index: usize,

    /// List state for feed list (handles scrolling).
    pub feed_list_state: ListState,

    /// Currently selected feed index (if any).
    pub selected_feed: Option<usize>,

    /// Selected item index within the feed.
    pub selected_item: usize,

    /// List state for items list (handles scrolling).
    pub items_list_state: ListState,

    /// Whether content panel is visible.
    pub show_content: bool,

    /// Content scroll offset.
    pub scroll_offset: u16,

    /// Search query.
    pub search_query: String,

    /// Search results: (`feed_index`, `item_index`).
    pub search_results: Vec<(usize, usize)>,

    /// Selected search result index.
    pub search_selected: usize,

    /// Selected theme index in theme picker.
    pub theme_picker_index: usize,

    /// Error message to display.
    pub error: Option<String>,

    /// Status message to display.
    pub status: Option<String>,

    // --- Add Feed state ---
    /// URL input for adding feed.
    pub add_feed_url: String,

    /// Discovered feeds from URL.
    pub discovered_feeds: Vec<DiscoveredFeed>,

    /// Selected discovered feed index.
    pub discovered_feed_index: usize,

    /// Custom name for the feed (empty = use discovered title).
    pub add_feed_name: String,

    /// Whether currently discovering feeds (loading state).
    pub discovering: bool,

    /// Selected folder index for new feed (None = root, Some = folder index).
    pub add_feed_folder_index: Option<usize>,

    /// New folder name being created.
    pub add_feed_new_folder: String,

    /// Whether creating a new folder.
    pub creating_new_folder: bool,

    // --- Share state ---
    /// Selected share platform index.
    pub share_platform_index: usize,

    // --- Sync state ---
    /// Whether sync is configured.
    pub sync_enabled: bool,
    /// Last sync status message.
    pub sync_status: Option<String>,
    /// Whether sync is in progress.
    pub syncing: bool,

    // --- Delete confirmation state ---
    /// Feed index pending deletion (for confirmation).
    pub pending_delete_feed: Option<usize>,
    /// Folder index pending deletion (for confirmation).
    pub pending_delete_folder: Option<usize>,

    // --- Error dialog state ---
    /// Error details for the error dialog (error message, context).
    pub error_dialog: Option<(String, Option<String>)>,

    // --- Update state ---
    /// Update available (latest version string).
    pub update_available: Option<String>,
    /// Detected package manager.
    pub package_manager: crate::update::PackageManager,
    /// Update status message (after completion).
    pub update_status: Option<String>,
    /// Flag to trigger update on next tick.
    pub pending_update: bool,
}

impl UiState {
    /// Set an error message (status bar, transient).
    pub fn set_error(&mut self, msg: impl Into<String>) {
        self.error = Some(msg.into());
    }

    /// Clear error message.
    pub fn clear_error(&mut self) {
        self.error = None;
    }

    /// Set a status message.
    pub fn set_status(&mut self, msg: impl Into<String>) {
        self.status = Some(msg.into());
    }

    /// Clear status message.
    pub fn clear_status(&mut self) {
        self.status = None;
    }

    /// Show the error dialog with details.
    pub fn show_error_dialog(&mut self, error: impl Into<String>, context: Option<String>) {
        self.error_dialog = Some((error.into(), context));
        self.mode = Mode::ErrorDialog;
    }

    /// Close the error dialog.
    pub fn close_error_dialog(&mut self) {
        self.error_dialog = None;
        self.mode = Mode::Normal;
    }

    /// Reset add feed state.
    pub fn reset_add_feed(&mut self) {
        self.add_feed_url.clear();
        self.discovered_feeds.clear();
        self.discovered_feed_index = 0;
        self.add_feed_name.clear();
        self.discovering = false;
        self.add_feed_folder_index = None;
        self.add_feed_new_folder.clear();
        self.creating_new_folder = false;
    }

    /// Reset delete confirmation state.
    pub const fn reset_delete(&mut self) {
        self.pending_delete_feed = None;
        self.pending_delete_folder = None;
    }
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            panel: Panel::default(),
            mode: Mode::default(),
            feed_list: Vec::new(),
            feed_list_index: 0,
            feed_list_state: ListState::default(),
            selected_feed: None,
            selected_item: 0,
            items_list_state: ListState::default(),
            show_content: false,
            scroll_offset: 0,
            search_query: String::new(),
            search_results: Vec::new(),
            search_selected: 0,
            theme_picker_index: 0,
            error: None,
            status: None,
            add_feed_url: String::new(),
            discovered_feeds: Vec::new(),
            discovered_feed_index: 0,
            add_feed_name: String::new(),
            discovering: false,
            add_feed_folder_index: None,
            add_feed_new_folder: String::new(),
            creating_new_folder: false,
            share_platform_index: 0,
            sync_enabled: false,
            sync_status: None,
            syncing: false,
            pending_delete_feed: None,
            pending_delete_folder: None,
            error_dialog: None,
            update_available: None,
            package_manager: crate::update::detect_package_manager(),
            update_status: None,
            pending_update: false,
        }
    }
}
