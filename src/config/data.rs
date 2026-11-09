//! Configuration data structures.

use std::{
    env, fs,
    path::{Path, PathBuf},
};

use color_eyre::{Result, eyre::WrapErr};
use serde::{Deserialize, Serialize};

use crate::sync::SyncConfig;
use ratatui_themes::Theme;

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

    /// File this configuration was loaded from, and is saved back to.
    ///
    /// Not stored in the file itself. `None` means the default location
    /// ([`Config::config_path`]). Public so that
    /// `Config { .., ..Config::default() }` keeps working for library users.
    #[serde(skip)]
    pub path: Option<PathBuf>,
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

/// File name of the config file inside [`Config::config_dir`].
const CONFIG_FILE_NAME: &str = "config.json";

/// Warn about a `config.toml` next to the real config file.
///
/// Feedo reads JSON from `config.json`. Older website docs said
/// `config.toml`, so people created that file and wondered why nothing
/// changed (issue #8). Returns the warning to show, if any.
fn ignored_toml_warning(config_dir: &Path) -> Option<String> {
    let toml = config_dir.join("config.toml");
    toml.is_file().then(|| {
        format!(
            "found {}, but feedo reads its settings from {} (JSON); the TOML file is ignored",
            toml.display(),
            config_dir.join(CONFIG_FILE_NAME).display()
        )
    })
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
            path: None,
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
        Self::config_dir().map(|dir| dir.join(CONFIG_FILE_NAME))
    }

    /// Get the data directory path (for caches, read states, etc.).
    #[must_use]
    pub fn data_dir() -> Option<PathBuf> {
        Self::config_dir().map(|dir| dir.join("data"))
    }

    /// Load configuration from the default location, creating it if missing.
    ///
    /// The default location is [`Config::config_path`]. Use
    /// [`Config::load_from`] to read a different file.
    ///
    /// # Errors
    ///
    /// Returns an error if the configuration file cannot be read or parsed.
    pub fn load() -> Result<Self> {
        let dir = Self::config_dir()
            .ok_or_else(|| color_eyre::eyre::eyre!("could not determine config directory"))?;

        if let Some(warning) = ignored_toml_warning(&dir) {
            tracing::warn!("{warning}");
        }

        Self::load_from(dir.join(CONFIG_FILE_NAME))
    }

    /// Load configuration from `path`, creating a default config there if the
    /// file does not exist yet.
    ///
    /// The returned config remembers `path`, so [`Config::save`] writes back
    /// to the same file.
    ///
    /// # Errors
    ///
    /// Returns an error if the file exists but cannot be read or is not valid
    /// JSON, or if a missing file cannot be created.
    pub fn load_from(path: impl Into<PathBuf>) -> Result<Self> {
        let path = path.into();

        if path.exists() {
            let content = fs::read_to_string(&path)
                .wrap_err_with(|| format!("failed to read config file {}", path.display()))?;
            let mut config: Self = serde_json::from_str(&content).wrap_err_with(|| {
                format!(
                    "failed to parse config file {} (feedo config files are JSON)",
                    path.display()
                )
            })?;
            config.path = Some(path);
            Ok(config)
        } else {
            let config = Self {
                path: Some(path),
                ..Self::default()
            };
            config.save()?;
            Ok(config)
        }
    }

    /// The file this configuration is saved to.
    ///
    /// This is the file it was loaded from, or the default location for a
    /// config that was not loaded from disk.
    #[must_use]
    pub fn file_path(&self) -> Option<PathBuf> {
        self.path.clone().or_else(Self::config_path)
    }

    /// Save configuration to disk.
    ///
    /// Writes to [`Config::file_path`], creating parent directories as needed.
    ///
    /// # Errors
    ///
    /// Returns an error if the configuration file cannot be written.
    pub fn save(&self) -> Result<()> {
        let path = self
            .file_path()
            .ok_or_else(|| color_eyre::eyre::eyre!("could not determine config directory"))?;

        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            fs::create_dir_all(parent)
                .wrap_err_with(|| format!("failed to create {}", parent.display()))?;
        }

        let content = serde_json::to_string_pretty(self)?;
        fs::write(&path, content)
            .wrap_err_with(|| format!("failed to write config file {}", path.display()))?;
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
    use ratatui_themes::ThemeName;

    /// A fresh, empty directory under the system temp dir, unique per test.
    fn temp_dir(name: &str) -> PathBuf {
        let dir = env::temp_dir().join(format!("feedo-test-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }

    #[test]
    fn load_from_should_read_the_given_file() {
        let dir = temp_dir("load-from");
        let path = dir.join("custom.json");
        fs::write(
            &path,
            r#"{"feeds": [{"name": "xkcd", "url": "https://xkcd.com/rss.xml"}], "refresh_interval": 5}"#,
        )
        .unwrap();

        let config = Config::load_from(&path).expect("config loads");

        assert_eq!(config.feeds[0].name, "xkcd");
        assert_eq!(config.refresh_interval, 5);
    }

    #[test]
    fn save_should_write_back_to_the_file_it_was_loaded_from() {
        let dir = temp_dir("save-back");
        let path = dir.join("custom.json");
        fs::write(&path, r#"{"refresh_interval": 5}"#).unwrap();

        let mut config = Config::load_from(&path).unwrap();
        config.refresh_interval = 42;
        config.save().unwrap();

        let reloaded = Config::load_from(&path).unwrap();
        assert_eq!(reloaded.refresh_interval, 42);
    }

    #[test]
    fn load_from_should_create_a_default_config_when_the_file_is_missing() {
        let dir = temp_dir("create-missing");
        let path = dir.join("nested").join("new.json");

        let config = Config::load_from(&path).unwrap();

        assert!(
            path.exists(),
            "default config written to {}",
            path.display()
        );
        assert_eq!(config.total_feeds(), Config::default().total_feeds());
    }

    #[test]
    fn load_from_should_name_the_file_when_it_is_not_json() {
        let dir = temp_dir("not-json");
        let path = dir.join("config.toml");
        fs::write(&path, "theme = \"dracula\"\n").unwrap();

        let err = Config::load_from(&path).expect_err("TOML is rejected");

        let message = format!("{err:#}");
        assert!(message.contains("config.toml"), "{message}");
        assert!(message.contains("JSON"), "{message}");
    }

    #[test]
    fn ignored_toml_warning_should_fire_when_config_toml_sits_next_to_config_json() {
        let dir = temp_dir("stray-toml");
        fs::write(dir.join("config.toml"), "theme = \"dracula\"\n").unwrap();

        let warning = ignored_toml_warning(&dir).expect("warning for stray config.toml");

        assert!(warning.contains("config.json"), "{warning}");
    }

    #[test]
    fn ignored_toml_warning_should_stay_quiet_without_config_toml() {
        let dir = temp_dir("no-toml");

        assert_eq!(ignored_toml_warning(&dir), None);
    }

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

    #[test]
    fn test_theme_serialization_roundtrip() {
        // Create config with specific theme
        let mut config = Config::default();
        config.theme = Theme::new(ThemeName::Dracula);

        // Serialize
        let json = serde_json::to_string_pretty(&config).unwrap();
        println!("Serialized config:\n{}", json);

        // Verify theme is in JSON
        assert!(
            json.contains(r#""name": "dracula""#),
            "Theme name not found in JSON"
        );

        // Deserialize
        let parsed: Config = serde_json::from_str(&json).unwrap();
        assert_eq!(
            parsed.theme.name,
            ThemeName::Dracula,
            "Theme not preserved after roundtrip"
        );
    }

    #[test]
    fn test_theme_deserialization_from_file_format() {
        // Test exact format from config file
        let json = r#"{
            "folders": [],
            "feeds": [],
            "theme": {
                "name": "dracula"
            },
            "refresh_interval": 30
        }"#;

        let config: Config = serde_json::from_str(json).unwrap();
        assert_eq!(config.theme.name, ThemeName::Dracula);
    }

    #[test]
    fn config_should_load_themes_added_in_ratatui_themes_0_4() {
        for (slug, expected) in [
            ("midnight-commander", ThemeName::MidnightCommander),
            ("msx", ThemeName::Msx),
        ] {
            let json = format!(
                r#"{{"folders": [], "feeds": [], "theme": {{"name": "{slug}"}}, "refresh_interval": 30}}"#
            );

            let config: Config = serde_json::from_str(&json).expect("config with new theme parses");

            assert_eq!(config.theme.name, expected, "theme slug {slug}");
        }
    }
}
