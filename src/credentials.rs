//! Secure credential storage using OS keychain.
//!
//! Uses the system keychain (macOS Keychain, Windows Credential Manager,
//! Linux Secret Service) to store sync credentials securely.

use keyring::Entry;
use tracing::{debug, warn};

const SERVICE_NAME: &str = "feedo";

/// Store a password in the system keychain.
///
/// # Arguments
/// * `username` - The username (used as the keychain entry identifier)
/// * `password` - The password to store
///
/// # Returns
/// `Ok(())` on success, or an error if the keychain is unavailable.
pub fn store_password(username: &str, password: &str) -> Result<(), String> {
    let entry = Entry::new(SERVICE_NAME, username).map_err(|e| format!("Keychain error: {e}"))?;
    entry
        .set_password(password)
        .map_err(|e| format!("Failed to store password: {e}"))?;
    debug!("Stored password in keychain for user: {username}");
    Ok(())
}

/// Retrieve a password from the system keychain.
///
/// # Arguments
/// * `username` - The username to look up
///
/// # Returns
/// The password if found, or `None` if not stored or keychain unavailable.
pub fn get_password(username: &str) -> Option<String> {
    let entry = Entry::new(SERVICE_NAME, username).ok()?;
    match entry.get_password() {
        Ok(password) => {
            debug!("Retrieved password from keychain for user: {username}");
            Some(password)
        }
        Err(keyring::Error::NoEntry) => {
            debug!("No password found in keychain for user: {username}");
            None
        }
        Err(e) => {
            warn!("Failed to retrieve password from keychain: {e}");
            None
        }
    }
}

/// Delete a password from the system keychain.
///
/// # Arguments
/// * `username` - The username whose password should be deleted
///
/// # Returns
/// `Ok(())` on success (including if entry didn't exist).
pub fn delete_password(username: &str) -> Result<(), String> {
    let entry = Entry::new(SERVICE_NAME, username).map_err(|e| format!("Keychain error: {e}"))?;
    match entry.delete_credential() {
        Ok(()) => {
            debug!("Deleted password from keychain for user: {username}");
            Ok(())
        }
        Err(keyring::Error::NoEntry) => {
            debug!("No password to delete for user: {username}");
            Ok(())
        }
        Err(e) => Err(format!("Failed to delete password: {e}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keyring_entry_creation() {
        // Just test that we can create an entry without panicking
        let result = Entry::new(SERVICE_NAME, "test_user");
        assert!(result.is_ok());
    }
}
