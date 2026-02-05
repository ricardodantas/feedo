//! Secure credential storage using AES-256-GCM encryption.
//!
//! Passwords are encrypted and stored in ~/.config/feedo/.credentials

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use tracing::debug;

/// Store a password securely (encrypted).
pub fn store_password(username: &str, password: &str) -> Result<(), String> {
    let path = credentials_file().ok_or("Cannot determine credentials path")?;
    
    // Load existing credentials or create new
    let mut creds: HashMap<String, String> = if path.exists() {
        let content = fs::read_to_string(&path).unwrap_or_default();
        serde_json::from_str(&content).unwrap_or_default()
    } else {
        HashMap::new()
    };
    
    // Encrypt the password
    let key = derive_key();
    let cipher = Aes256Gcm::new_from_slice(&key).map_err(|e| e.to_string())?;
    
    // Use username hash as nonce (12 bytes)
    let nonce = make_nonce(username);
    
    let encrypted = cipher
        .encrypt(Nonce::from_slice(&nonce), password.as_bytes())
        .map_err(|e| format!("Encryption failed: {e}"))?;
    
    let encoded = BASE64.encode(&encrypted);
    creds.insert(username.to_string(), encoded);
    
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
    
    debug!("Stored encrypted password for: {username}");
    Ok(())
}

/// Retrieve a password (decrypted).
pub fn get_password(username: &str) -> Option<String> {
    let path = credentials_file()?;
    if !path.exists() {
        return None;
    }
    
    let content = fs::read_to_string(&path).ok()?;
    let creds: HashMap<String, String> = serde_json::from_str(&content).ok()?;
    
    let encoded = creds.get(username)?;
    let encrypted = BASE64.decode(encoded).ok()?;
    
    let key = derive_key();
    let cipher = Aes256Gcm::new_from_slice(&key).ok()?;
    let nonce = make_nonce(username);
    
    let decrypted = cipher.decrypt(Nonce::from_slice(&nonce), encrypted.as_ref()).ok()?;
    let password = String::from_utf8(decrypted).ok()?;
    
    debug!("Retrieved encrypted password for: {username}");
    Some(password)
}

/// Delete a stored password.
pub fn delete_password(username: &str) -> Result<(), String> {
    let path = credentials_file().ok_or("Cannot determine credentials path")?;
    if !path.exists() {
        return Ok(());
    }
    
    let content = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let mut creds: HashMap<String, String> = serde_json::from_str(&content).unwrap_or_default();
    
    creds.remove(username);
    
    let content = serde_json::to_string(&creds).map_err(|e| e.to_string())?;
    fs::write(&path, &content).map_err(|e| e.to_string())?;
    
    Ok(())
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
    
    // Derive key from machine-specific data
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
    
    // Combine into 32 bytes
    let mut key = [0u8; 32];
    key[0..8].copy_from_slice(&hash1.to_le_bytes());
    key[8..16].copy_from_slice(&hash2.to_le_bytes());
    key[16..24].copy_from_slice(&hash1.to_be_bytes());
    key[24..32].copy_from_slice(&hash2.to_be_bytes());
    key
}

fn make_nonce(username: &str) -> [u8; 12] {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    
    let mut nonce = [0u8; 12];
    let mut h = DefaultHasher::new();
    username.hash(&mut h);
    nonce[0..8].copy_from_slice(&h.finish().to_le_bytes());
    nonce
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_roundtrip() {
        let username = "test_roundtrip_user";
        let password = "test_password_123!@#";
        
        store_password(username, password).expect("Store failed");
        let retrieved = get_password(username);
        assert_eq!(retrieved, Some(password.to_string()));
        
        let _ = delete_password(username);
    }
    
    #[test]
    fn test_key_consistent() {
        assert_eq!(derive_key(), derive_key());
    }
}
