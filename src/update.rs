//! Update checking and self-update functionality.

use std::process::Stdio;

/// GitHub repository path for update checks.
pub const GITHUB_REPO: &str = "ricardodantas/feedo";

/// Current version from Cargo.toml.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Result of a version check.
#[derive(Debug, Clone)]
pub enum VersionCheck {
    /// An update is available.
    UpdateAvailable {
        /// Latest version available.
        latest: String,
        /// Current installed version.
        current: String,
    },
    /// Already on the latest version.
    UpToDate,
    /// Check failed with an error.
    CheckFailed(String),
}

/// Detected package manager for installation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PackageManager {
    /// Installed via cargo.
    Cargo,
    /// Installed via Homebrew (includes tap formula name).
    Homebrew {
        /// Full formula name (e.g., "ricardodantas/tap/feedo").
        formula: String,
    },
}

impl PackageManager {
    /// Get display name.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Cargo => "cargo",
            Self::Homebrew { .. } => "brew",
        }
    }

    /// Get the update command.
    #[must_use]
    pub fn update_command(&self) -> String {
        match self {
            Self::Cargo => "cargo install feedo".to_string(),
            Self::Homebrew { formula } => format!("brew upgrade {formula}"),
        }
    }
}

/// Detect how feedo was installed.
#[must_use]
pub fn detect_package_manager() -> PackageManager {
    // Check if the current executable is in Homebrew's Cellar
    if let Ok(exe_path) = std::env::current_exe() {
        let exe_str = exe_path.to_string_lossy();

        if exe_str.contains("/Cellar/") || exe_str.contains("/homebrew/") {
            // Try to get the full formula name from brew
            if let Ok(output) = std::process::Command::new("brew")
                .args(["info", "--json=v2", "feedo"])
                .output()
                && output.status.success()
                && let Ok(json) = serde_json::from_slice::<serde_json::Value>(&output.stdout)
                && let Some(formulae) = json.get("formulae").and_then(|f| f.as_array())
                && let Some(formula) = formulae.first()
                && let Some(full_name) = formula.get("full_name").and_then(|n| n.as_str())
            {
                return PackageManager::Homebrew {
                    formula: full_name.to_string(),
                };
            }
            // Fallback to just "feedo" if we can't determine the tap
            return PackageManager::Homebrew {
                formula: "feedo".to_string(),
            };
        }
    }

    // Default to cargo
    PackageManager::Cargo
}

/// Check if a newer version is available on GitHub.
pub async fn check_for_updates() -> VersionCheck {
    check_for_updates_timeout(std::time::Duration::from_secs(3)).await
}

/// Check if a newer version is available on GitHub with custom timeout.
pub async fn check_for_updates_timeout(timeout: std::time::Duration) -> VersionCheck {
    let url = format!("https://api.github.com/repos/{GITHUB_REPO}/releases/latest");

    let client = match reqwest::Client::builder().timeout(timeout).build() {
        Ok(c) => c,
        Err(e) => return VersionCheck::CheckFailed(format!("Failed to create client: {e}")),
    };

    let result = client
        .get(&url)
        .header("User-Agent", format!("feedo/{VERSION}"))
        .send()
        .await;

    match result {
        Ok(response) => match response.json::<serde_json::Value>().await {
            Ok(json) => json.get("tag_name").and_then(|v| v.as_str()).map_or_else(
                || VersionCheck::CheckFailed("Could not parse release info".to_string()),
                |tag| {
                    let latest = tag.trim_start_matches('v').to_string();
                    let current = VERSION.to_string();

                    if version_is_newer(&latest, &current) {
                        VersionCheck::UpdateAvailable { latest, current }
                    } else {
                        VersionCheck::UpToDate
                    }
                },
            ),
            Err(e) => VersionCheck::CheckFailed(format!("Failed to parse response: {e}")),
        },
        Err(e) => VersionCheck::CheckFailed(format!("Request failed: {e}")),
    }
}

/// Compare two semver strings, returns true if `latest` is newer than `current`.
fn version_is_newer(latest: &str, current: &str) -> bool {
    let parse = |v: &str| -> (u32, u32, u32) {
        let parts: Vec<u32> = v
            .split('.')
            .take(3)
            .map(|s| s.parse().unwrap_or(0))
            .collect();
        (
            parts.first().copied().unwrap_or(0),
            parts.get(1).copied().unwrap_or(0),
            parts.get(2).copied().unwrap_or(0),
        )
    };

    let (l_major, l_minor, l_patch) = parse(latest);
    let (c_major, c_minor, c_patch) = parse(current);

    (l_major, l_minor, l_patch) > (c_major, c_minor, c_patch)
}

/// Run the update command and return the result.
///
/// # Errors
///
/// Returns an error message if the update fails.
pub fn run_update(pm: &PackageManager) -> Result<(), String> {
    match pm {
        PackageManager::Cargo => {
            match std::process::Command::new("cargo")
                .args(["install", "feedo"])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
            {
                Ok(status) if status.success() => Ok(()),
                Ok(status) => Err(format!("Update failed with status: {status}")),
                Err(e) => Err(format!("Failed to run cargo: {e}")),
            }
        }
        PackageManager::Homebrew { formula } => {
            // First update the tap to get latest formula
            let _ = std::process::Command::new("brew")
                .args(["update"])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();

            // Then upgrade the formula
            match std::process::Command::new("brew")
                .args(["upgrade", formula])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
            {
                Ok(status) if status.success() => Ok(()),
                Ok(_) => {
                    // upgrade returns non-zero if already up to date, try reinstall
                    match std::process::Command::new("brew")
                        .args(["reinstall", formula])
                        .stdout(Stdio::null())
                        .stderr(Stdio::null())
                        .status()
                    {
                        Ok(status) if status.success() => Ok(()),
                        Ok(status) => Err(format!("Update failed with status: {status}")),
                        Err(e) => Err(format!("Failed to run brew: {e}")),
                    }
                }
                Err(e) => Err(format!("Failed to run brew: {e}")),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_is_newer() {
        assert!(version_is_newer("1.2.0", "1.1.0"));
        assert!(version_is_newer("1.1.1", "1.1.0"));
        assert!(version_is_newer("2.0.0", "1.9.9"));
        assert!(!version_is_newer("1.1.0", "1.1.0"));
        assert!(!version_is_newer("1.0.0", "1.1.0"));
        assert!(!version_is_newer("0.9.0", "1.0.0"));
    }

    #[test]
    fn test_detect_package_manager() {
        // Should default to cargo on this system
        let pm = detect_package_manager();
        // Just verify it doesn't panic and returns something
        assert!(!pm.name().is_empty());
        assert!(!pm.update_command().is_empty());
    }

    #[tokio::test]
    async fn test_check_for_updates_does_not_panic() {
        // This actually hits the GitHub API, but with a short timeout
        let result = check_for_updates_timeout(std::time::Duration::from_secs(5)).await;
        // Should either succeed or fail gracefully, not panic
        match result {
            VersionCheck::UpdateAvailable { latest, current } => {
                assert!(!latest.is_empty());
                assert!(!current.is_empty());
            }
            VersionCheck::UpToDate => {
                // That's fine too
            }
            VersionCheck::CheckFailed(msg) => {
                // Network issues are acceptable in tests
                assert!(!msg.is_empty());
            }
        }
    }
}
