//! Theme configuration and colors.

use ratatui::style::Color;
use serde::{Deserialize, Serialize};

/// Available accent colors for the UI.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AccentColor {
    #[default]
    Cyan,
    Blue,
    Green,
    Yellow,
    Magenta,
    Red,
    Orange,
    Pink,
}

impl AccentColor {
    /// Convert to ratatui Color.
    #[must_use]
    pub const fn to_color(self) -> Color {
        match self {
            Self::Cyan => Color::Cyan,
            Self::Blue => Color::Blue,
            Self::Green => Color::Green,
            Self::Yellow => Color::Yellow,
            Self::Magenta => Color::Magenta,
            Self::Red => Color::Red,
            Self::Orange => Color::Rgb(255, 165, 0),
            Self::Pink => Color::Rgb(255, 105, 180),
        }
    }
}

/// Theme configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Theme {
    /// Primary accent color.
    #[serde(default)]
    pub accent: AccentColor,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            accent: AccentColor::Cyan,
        }
    }
}

impl Theme {
    /// Get the accent color.
    #[must_use]
    pub const fn accent(&self) -> Color {
        self.accent.to_color()
    }

    /// Get the muted/secondary color.
    #[must_use]
    pub const fn muted(&self) -> Color {
        Color::DarkGray
    }

    /// Get the highlight color for selected items.
    #[must_use]
    pub const fn highlight(&self) -> Color {
        Color::Yellow
    }

    /// Get the unread indicator color.
    #[must_use]
    pub const fn unread(&self) -> Color {
        self.accent.to_color()
    }

    /// Get the error color.
    #[must_use]
    pub const fn error(&self) -> Color {
        Color::Red
    }
}
