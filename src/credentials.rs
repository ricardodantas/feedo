//! Secure credential storage.
//!
//! Tries OS keychain first (macOS Keychain, Windows Credential Manager, Linux Secret Service).
//! Falls back to encrypted file storage if keychain is unavailable.

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use keyring::Entry;
use std::fs;
use std::path::PathBuf;
use tracing::{debug, warn};

const SERVICE_NAME: &str = "feedo";

/// Store a password securely.
/// Tries keychain first, falls back to encrypted file.
pub fn store_password(username: &str, password: &str) -> Result<(), String> {
    // Try keychain first
    if let Ok(()) = store_in_keychain(username, password) {
        return Ok(());
    }
    
    // Fall back to encrypted file
    store_encrypted(username, password)
}

/// Retrieve a password.
/// Tries keychain first, falls back to encrypted file.
pub fn get_password(username: &str) -> Option<String> {
    // Try keychain first
    if let Some(password) = get_from_keychain(username) {
        return Some(password);
    }
    
    // Fall back to encrypted file
    get_encrypted(username)
}

/// Delete a stored password.
pub fn delete_password(username: &str) -> Result<(), String> {
    // Try both storage methods
    let _ = delete_from_keychain(username);
    let _ = delete_encrypted(username);
    Ok(())
}

// === Keychain Storage ===

fn store_in_keychain(username: &str, password: &str) -> Result<(), String> {
    let entry = Entry::new(SERVICE_NAME, username).map_err(|e| format!("Keychain error: {e}"))?;
    entry
        .set_password(password)
        .map_err(|e| format!("Failed to store: {e}"))?;
    
    // Verify it was actually stored (macOS can silently fail for unsigned binaries)
    let verify = Entry::new(SERVICE_NAME, username).map_err(|e| format!("Keychain error: {e}"))?;
    match verify.get_password() {
        Ok(stored) if stored == password => {
            debug!("Stored password in keychain for: {username}");
            Ok(())
        }
        _ => Err("Keychain storage verification failed".to_string()),
    }
}

fn get_from_keychain(username: &str) -> Option<String> {
    let entry = Entry::new(SERVICE_NAME, username).ok()?;
    match entry.get_password() {
        Ok(password) => {
            debug!("Retrieved password from keychain for: {username}");
            Some(password)
        }
        Err(keyring::Error::NoEntry) => None,
        Err(e) => {
            warn!("Keychain retrieval failed: {e}");
            None
        }
    }
}

fn delete_from_keychain(username: &str) -> Result<(), String> {
    let entry = Entry::new(SERVICE_NAME, username).map_err(|e| e.to_string())?;
    entry.delete_credential().map_err(|e| e.to_string())
}

// === Encrypted File Storage ===

fn credentials_file() -> Option<PathBuf> {
    let home = std::env::var("HOME").ok()?;
    Some(PathBuf::from(home).join(".config").join("feedo").join(".credentials"))
}

fn derive_key() -> [u8; 32] {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    
    // Derive key from machine-specific data
    let mut hasher = DefaultHasher::new();
    
    // Use username and home directory as entropy sources
    if let Ok(user) = std::env::var("USER") {
        user.hash(&mut hasher);
    }
    if let Ok(home) = std::env::var("HOME") {
        home.hash(&mut hasher);
    }
    // Add hostname if available
    if let Ok(hostname) = std::env::var("HOSTNAME") {
        hostname.hash(&mut hasher);
    }
    // Add a salt
    "feedo-credential-salt-v1".hash(&mut hasher);
    
    let hash1 = hasher.finish();
    
    // Double hash for more entropy
    let mut hasher2 = DefaultHasher::new();
    hash1.hash(&mut hasher2);
    "feedo-credential-salt-v2".hash(&mut hasher2);
    let hash2 = hasher2.finish();
    
    // Combine into 32 bytes
    let mut key = [0u8; 32];
    key[0..8].copy_from_slice(&hash1.to_le_bytes());
    key[8..16].copy_from_slice(&hash2.to_le_bytes());
    key[16..24].copy_from_slice(&hash1.to_be_bytes());
    key[24..32].copy_from_slice(&hash2.to_be_bytes());
    key
}

fn store_encrypted(username: &str, password: &str) -> Result<(), String> {
    let path = credentials_file().ok_or("Cannot determine credentials path")?;
    
    // Load existing credentials or create new
    let mut creds: std::collections::HashMap<String, String> = if path.exists() {
        let content = fs::read_to_string(&path).unwrap_or_default();
        serde_json::from_str(&content).unwrap_or_default()
    } else {
        std::collections::HashMap::new()
    };
    
    // Encrypt the password
    let key = derive_key();
    let cipher = Aes256Gcm::new_from_slice(&key).map_err(|e| e.to_string())?;
    
    // Use username hash as nonce (12 bytes)
    let mut nonce_bytes = [0u8; 12];
    let username_hash = {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut h = DefaultHasher::new();
        username.hash(&mut h);
        h.finish()
    };
    nonce_bytes[0..8].copy_from_slice(&username_hash.to_le_bytes());
    let nonce = Nonce::from_slice(&nonce_bytes);
    
    let encrypted = cipher
        .encrypt(nonce, password.as_bytes())
        .map_err(|e| format!("Encryption failed: {e}"))?;
    
    let encoded = BASE64.encode(&encrypted);
    creds.insert(username.to_string(), encoded);
    
    // Ensure directory exists
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    
    // Write with restricted permissions
    let content = serde_json::to_string(&creds).map_err(|e| e.to_string())?;
    fs::write(&path, &content).map_err(|e| e.to_string())?;
    
    // Set file permissions to owner-only (Unix)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o600);
        let _ = fs::set_permissions(&path, perms);
    }
    
    debug!("Stored encrypted password for: {username}");
    Ok(())
}

fn get_encrypted(username: &str) -> Option<String> {
    let path = credentials_file()?;
    if !path.exists() {
        return None;
    }
    
    let content = fs::read_to_string(&path).ok()?;
    let creds: std::collections::HashMap<String, String> = serde_json::from_str(&content).ok()?;
    
    let encoded = creds.get(username)?;
    let encrypted = BASE64.decode(encoded).ok()?;
    
    let key = derive_key();
    let cipher = Aes256Gcm::new_from_slice(&key).ok()?;
    
    // Recreate nonce from username
    let mut nonce_bytes = [0u8; 12];
    let username_hash = {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut h = DefaultHasher::new();
        username.hash(&mut h);
        h.finish()
    };
    nonce_bytes[0..8].copy_from_slice(&username_hash.to_le_bytes());
    let nonce = Nonce::from_slice(&nonce_bytes);
    
    let decrypted = cipher.decrypt(nonce, encrypted.as_ref()).ok()?;
    let password = String::from_utf8(decrypted).ok()?;
    
    debug!("Retrieved encrypted password for: {username}");
    Some(password)
}

fn delete_encrypted(username: &str) -> Result<(), String> {
    let path = credentials_file().ok_or("Cannot determine credentials path")?;
    if !path.exists() {
        return Ok(());
    }
    
    let content = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let mut creds: std::collections::HashMap<String, String> = 
        serde_json::from_str(&content).unwrap_or_default();
    
    creds.remove(username);
    
    let content = serde_json::to_string(&creds).map_err(|e| e.to_string())?;
    fs::write(&path, &content).map_err(|e| e.to_string())?;
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypted_roundtrip() {
        let username = "test_user_encrypted";
        let password = "test_password_123!@#";
        
        // Store
        store_encrypted(username, password).expect("Store failed");
        
        // Retrieve
        let retrieved = get_encrypted(username);
        assert_eq!(retrieved, Some(password.to_string()));
        
        // Cleanup
        let _ = delete_encrypted(username);
    }
    
    #[test]
    fn test_key_derivation_consistent() {
        let key1 = derive_key();
        let key2 = derive_key();
        assert_eq!(key1, key2);
    }
}
