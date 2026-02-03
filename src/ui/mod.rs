//! Terminal UI components.
//!
//! This module contains all UI-related code including:
//! - Screen rendering
//! - Input handling
//! - Widget components

pub mod input;
mod render;
pub mod state;
pub mod widgets;

pub use render::LOGO;
pub use state::{FeedListItem, Mode, Panel, UiState};
