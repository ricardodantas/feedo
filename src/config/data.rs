//! Configuration data structures.

use std::{env, fs, path::PathBuf};

use color_eyre::Result;
use serde::{Deserialize, Serialize};

use crate::sync::SyncConfig;
use crate::theme::Theme;

/// Application configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Folders containing feeds.
    #[serde(default)]
    pub folders: Vec<FolderConfig>,

    /// Root-level feeds (not in any folder).
    #[serde(default)]
    pub feeds: Vec<FeedConfig>,

    /// UI theme settings.
    #[serde(default)]
    pub theme: Theme,

    /// Refresh interval in minutes (0 = manual only).
    #[serde(default = "default_refresh_interval")]
    pub refresh_interval: u32,

    /// Sync configuration (optional).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sync: Option<SyncConfig>,
}

/// A folder containing multiple feeds.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FolderConfig {
    /// Display name.
    pub name: String,

    /// Optional emoji icon.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,

    /// Whether the folder is expanded in the UI.
    #[serde(default = "default_true")]
    pub expanded: bool,

    /// Feeds in this folder.
    pub feeds: Vec<FeedConfig>,
}

/// A single feed configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedConfig {
    /// Display name.
    pub name: String,

    /// Feed URL (RSS/Atom).
    pub url: String,

    /// Sync ID from server (e.g., "feed/123" for Google Reader API).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sync_id: Option<String>,
}

const fn default_true() -> bool {
    true
}

const fn default_refresh_interval() -> u32 {
    30 // 30 minutes
}

impl Default for Config {
    fn default() -> Self {
        Self {
            folders: vec![
                FolderConfig {
                    name: "Tech".to_string(),
                    icon: Some("💻".to_string()),
                    expanded: true,
                    feeds: vec![
                        FeedConfig {
                            name: "Hacker News".to_string(),
                            url: "https://hnrss.org/frontpage".to_string(),
                            sync_id: None,
                        },
                        FeedConfig {
                            name: "Lobsters".to_string(),
                            url: "https://lobste.rs/rss".to_string(),
                            sync_id: None,
                        },
                    ],
                },
                FolderConfig {
                    name: "News".to_string(),
                    icon: Some("📰".to_string()),
                    expanded: false,
                    feeds: vec![FeedConfig {
                        name: "BBC World".to_string(),
                        url: "https://feeds.bbci.co.uk/news/world/rss.xml".to_string(),
                        sync_id: None,
                    }],
                },
            ],
            feeds: vec![],
            theme: Theme::default(),
            refresh_interval: default_refresh_interval(),
            sync: None,
        }
    }
}

impl Config {
    /// Get the configuration directory path.
    ///
    /// Uses `~/.config/feedo` on all platforms for consistency.
    /// Falls back to platform-specific directories if HOME is not set.
    #[must_use]
    pub fn config_dir() -> Option<PathBuf> {
        // Prefer ~/.config/feedo on all platforms (XDG-style)
        if let Ok(home) = env::var("HOME") {
            return Some(PathBuf::from(home).join(".config").join("feedo"));
        }

        // Fallback for Windows if HOME not set
        if let Ok(userprofile) = env::var("USERPROFILE") {
            return Some(PathBuf::from(userprofile).join(".config").join("feedo"));
        }

        None
    }

    /// Get the configuration file path.
    #[must_use]
    pub fn config_path() -> Option<PathBuf> {
        Self::config_dir().map(|dir| dir.join("config.json"))
    }

    /// Get the data directory path (for caches, read states, etc.).
    #[must_use]
    pub fn data_dir() -> Option<PathBuf> {
        Self::config_dir().map(|dir| dir.join("data"))
    }

    /// Load configuration from disk, creating default if not exists.
    ///
    /// # Errors
    ///
    /// Returns an error if the configuration file cannot be read or parsed.
    pub fn load() -> Result<Self> {
        let path = Self::config_path()
            .ok_or_else(|| color_eyre::eyre::eyre!("Could not determine config directory"))?;

        if path.exists() {
            let content = fs::read_to_string(&path)?;
            let config: Self = serde_json::from_str(&content)?;
            Ok(config)
        } else {
            let config = Self::default();
            config.save()?;
            Ok(config)
        }
    }

    /// Save configuration to disk.
    ///
    /// # Errors
    ///
    /// Returns an error if the configuration file cannot be written.
    pub fn save(&self) -> Result<()> {
        let dir = Self::config_dir()
            .ok_or_else(|| color_eyre::eyre::eyre!("Could not determine config directory"))?;
        fs::create_dir_all(&dir)?;

        let path = dir.join("config.json");
        let content = serde_json::to_string_pretty(self)?;
        fs::write(path, content)?;
        Ok(())
    }

    /// Count total number of feeds across all folders and root.
    #[must_use]
    pub fn total_feeds(&self) -> usize {
        self.folders.iter().map(|f| f.feeds.len()).sum::<usize>() + self.feeds.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert!(!config.folders.is_empty());
        assert_eq!(config.refresh_interval, 30);
    }

    #[test]
    fn test_config_serialization() {
        let config = Config::default();
        let json = serde_json::to_string(&config).unwrap();
        let parsed: Config = serde_json::from_str(&json).unwrap();
        assert_eq!(config.folders.len(), parsed.folders.len());
    }
}
