//! Error reporting utilities.
//!
//! Provides functionality to report errors to GitHub issues.

use std::env::consts::{ARCH, OS};

/// Application version from Cargo.toml.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// GitHub repository URL.
pub const REPO_URL: &str = "https://github.com/ricardodantas/feedo";

/// Generate a GitHub issue URL pre-filled with error information.
#[must_use]
pub fn create_issue_url(error: &str, context: Option<&str>) -> String {
    let title = urlencoding::encode("[Bug]: Application Error");
    
    let body = format!(
        "## Error Message\n\
        ```\n{error}\n```\n\n\
        ## Environment\n\
        - **Feedo Version:** {VERSION}\n\
        - **OS:** {OS}\n\
        - **Architecture:** {ARCH}\n\
        - **Terminal:** (please fill in)\n\n\
        ## Context\n\
        {}\n\n\
        ## Steps to Reproduce\n\
        1. (please describe what you were doing)\n\n\
        ## Additional Information\n\
        (add any other relevant details)",
        context.unwrap_or("(not provided)")
    );
    
    let encoded_body = urlencoding::encode(&body);
    
    format!("{REPO_URL}/issues/new?title={title}&body={encoded_body}&labels=bug,triage")
}

/// Open the GitHub issue page in the default browser.
///
/// # Errors
///
/// Returns an error if the browser cannot be opened.
pub fn open_issue(error: &str, context: Option<&str>) -> Result<(), std::io::Error> {
    let url = create_issue_url(error, context);
    open::that(&url).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
}
