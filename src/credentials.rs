//! Secure credential storage using AES-256-GCM encryption.
//!
//! Credentials (username + password) are encrypted and stored in ~/.config/feedo/.credentials

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use tracing::debug;

/// Encrypted credentials for a service.
#[derive(Debug, Serialize, Deserialize)]
struct EncryptedCredentials {
    username: String,
    password: String,
}

/// Store credentials securely (both username and password encrypted).
/// 
/// # Arguments
/// * `key` - Unique key for this credential (e.g., "sync@server.com")
/// * `username` - The username to store
/// * `password` - The password to store
pub fn store_credentials(key: &str, username: &str, password: &str) -> Result<(), String> {
    let path = credentials_file().ok_or("Cannot determine credentials path")?;
    
    // Load existing credentials or create new
    let mut creds: HashMap<String, String> = if path.exists() {
        let content = fs::read_to_string(&path).unwrap_or_default();
        serde_json::from_str(&content).unwrap_or_default()
    } else {
        HashMap::new()
    };
    
    // Create credentials object
    let credentials = EncryptedCredentials {
        username: username.to_string(),
        password: password.to_string(),
    };
    let plaintext = serde_json::to_string(&credentials).map_err(|e| e.to_string())?;
    
    // Encrypt
    let encryption_key = derive_key();
    let cipher = Aes256Gcm::new_from_slice(&encryption_key).map_err(|e| e.to_string())?;
    let nonce = make_nonce(key);
    
    let encrypted = cipher
        .encrypt(Nonce::from_slice(&nonce), plaintext.as_bytes())
        .map_err(|e| format!("Encryption failed: {e}"))?;
    
    let encoded = BASE64.encode(&encrypted);
    creds.insert(key.to_string(), encoded);
    
    // Ensure directory exists
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    
    // Write credentials file
    let content = serde_json::to_string(&creds).map_err(|e| e.to_string())?;
    fs::write(&path, &content).map_err(|e| e.to_string())?;
    
    // Set file permissions to owner-only (Unix)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o600);
        let _ = fs::set_permissions(&path, perms);
    }
    
    // Verify the credentials can be read back
    if get_credentials(key).is_none() {
        return Err("Credentials written but verification failed".to_string());
    }
    
    debug!("Stored encrypted credentials for: {key}");
    Ok(())
}

/// Retrieve credentials (decrypted username and password).
/// 
/// # Arguments
/// * `key` - Unique key for this credential
/// 
/// # Returns
/// Tuple of (username, password) if found, None otherwise.
pub fn get_credentials(key: &str) -> Option<(String, String)> {
    let path = credentials_file()?;
    if !path.exists() {
        return None;
    }
    
    let content = fs::read_to_string(&path).ok()?;
    let creds: HashMap<String, String> = serde_json::from_str(&content).ok()?;
    
    let encoded = creds.get(key)?;
    let encrypted = BASE64.decode(encoded).ok()?;
    
    let encryption_key = derive_key();
    let cipher = Aes256Gcm::new_from_slice(&encryption_key).ok()?;
    let nonce = make_nonce(key);
    
    let decrypted = cipher.decrypt(Nonce::from_slice(&nonce), encrypted.as_ref()).ok()?;
    let plaintext = String::from_utf8(decrypted).ok()?;
    let credentials: EncryptedCredentials = serde_json::from_str(&plaintext).ok()?;
    
    debug!("Retrieved encrypted credentials for: {key}");
    Some((credentials.username, credentials.password))
}

/// Delete stored credentials.
pub fn delete_credentials(key: &str) -> Result<(), String> {
    let path = credentials_file().ok_or("Cannot determine credentials path")?;
    if !path.exists() {
        return Ok(());
    }
    
    let content = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let mut creds: HashMap<String, String> = serde_json::from_str(&content).unwrap_or_default();
    
    creds.remove(key);
    
    let content = serde_json::to_string(&creds).map_err(|e| e.to_string())?;
    fs::write(&path, &content).map_err(|e| e.to_string())?;
    
    Ok(())
}

// Legacy compatibility - keep old API working
pub fn store_password(key: &str, password: &str) -> Result<(), String> {
    store_credentials(key, key, password)
}

pub fn get_password(key: &str) -> Option<String> {
    get_credentials(key).map(|(_, password)| password)
}

pub fn delete_password(key: &str) -> Result<(), String> {
    delete_credentials(key)
}

fn credentials_file() -> Option<PathBuf> {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .ok()?;
    Some(PathBuf::from(home).join(".config").join("feedo").join(".credentials"))
}

fn derive_key() -> [u8; 32] {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    
    let mut hasher = DefaultHasher::new();
    
    if let Ok(user) = std::env::var("USER").or_else(|_| std::env::var("USERNAME")) {
        user.hash(&mut hasher);
    }
    if let Ok(home) = std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")) {
        home.hash(&mut hasher);
    }
    "feedo-credential-salt-v1".hash(&mut hasher);
    let hash1 = hasher.finish();
    
    let mut hasher2 = DefaultHasher::new();
    hash1.hash(&mut hasher2);
    "feedo-credential-salt-v2".hash(&mut hasher2);
    let hash2 = hasher2.finish();
    
    let mut key = [0u8; 32];
    key[0..8].copy_from_slice(&hash1.to_le_bytes());
    key[8..16].copy_from_slice(&hash2.to_le_bytes());
    key[16..24].copy_from_slice(&hash1.to_be_bytes());
    key[24..32].copy_from_slice(&hash2.to_be_bytes());
    key
}

fn make_nonce(input: &str) -> [u8; 12] {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    
    let mut nonce = [0u8; 12];
    let mut h = DefaultHasher::new();
    input.hash(&mut h);
    nonce[0..8].copy_from_slice(&h.finish().to_le_bytes());
    nonce
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_credentials_roundtrip() {
        let key = "test_creds_key";
        let username = "test_user";
        let password = "test_password_123!@#";
        
        store_credentials(key, username, password).expect("Store failed");
        let retrieved = get_credentials(key);
        assert_eq!(retrieved, Some((username.to_string(), password.to_string())));
        
        let _ = delete_credentials(key);
    }
    
    #[test]
    fn test_key_consistent() {
        assert_eq!(derive_key(), derive_key());
    }
}
