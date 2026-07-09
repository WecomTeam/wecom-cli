use std::fs;
use std::path::PathBuf;

use anyhow::Result;
use base64::prelude::*;
use rand::Rng;

use crate::{fs_util, paths};

use super::cipher;

const KEYRING_SERVICE: &str = "wecom-cli";
const KEYRING_USER: &str = "encryption-key";

// ---------------------------------------------------------------------------
// Paths
// ---------------------------------------------------------------------------

/// Return the file path for the local encryption key fallback.
pub fn encryption_key_path() -> PathBuf {
    paths::wecom_home_dir().join(".encryption_key")
}

// ---------------------------------------------------------------------------
// Encode / Decode
// ---------------------------------------------------------------------------

/// Encode a 32-byte key as a Base64 string.
fn encode_key(key: &[u8; 32]) -> String {
    BASE64_STANDARD.encode(key)
}

/// Decode a Base64 string into a 32-byte key, returning an error on invalid input.
fn decode_key(s: &str) -> Result<[u8; 32]> {
    let bytes = BASE64_STANDARD
        .decode(s)
        .map_err(|e| anyhow::anyhow!("base64 decode error: {e}"))?;
    <[u8; 32]>::try_from(bytes.as_slice())
        .map_err(|_| anyhow::anyhow!("Invalid encryption key length"))
}

// ---------------------------------------------------------------------------
// Key generation / loading / saving
// ---------------------------------------------------------------------------

/// Generate a fresh random 256-bit key.
pub fn generate_random_key() -> [u8; 32] {
    rand::rng().random()
}

/// Load the key from keyring. Returns `None` if unavailable.
fn load_key_from_keyring() -> Option<[u8; 32]> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER).ok()?;
    let b64 = entry.get_password().ok()?;
    decode_key(b64.trim()).ok()
}

/// Load the key from the file fallback. Returns `None` if unavailable.
fn load_key_from_file() -> Option<[u8; 32]> {
    let contents = fs::read_to_string(encryption_key_path()).ok()?;
    decode_key(contents.trim()).ok()
}

/// Try to load an existing key.
///
/// Priority: process cache → file → keyring (last resort, may prompt).
/// The result is cached for the lifetime of the process.
pub fn load_existing_key() -> Option<[u8; 32]> {
    load_key_from_file().or_else(load_key_from_keyring)
}

/// Persist the key. Writes to the file fallback always; writes to keyring
/// at most once per process to avoid repeated macOS Keychain prompts.
///
/// If the key is already cached and identical, this is a no-op.
pub fn save_key(key: &[u8; 32]) -> Result<()> {
    let b64 = encode_key(key);

    // Always write the file fallback.
    let key_path = encryption_key_path();
    fs_util::atomic_write(&key_path, b64.as_bytes(), Some(0o600))?;

    if keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER)
        .and_then(|entry| entry.set_password(&b64))
        .is_err()
    {
        tracing::warn!("Keyring unavailable – encryption key stored in file only");
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Encrypt / Decrypt helpers for serializable data
// ---------------------------------------------------------------------------

/// Encrypt serializable data: serialize → AES-256-GCM encrypt.
pub fn encrypt_data<T: serde::Serialize + ?Sized>(data: &T, key: &[u8; 32]) -> Result<Vec<u8>> {
    let json =
        serde_json::to_vec(data).map_err(|e| anyhow::anyhow!("JSON serialize error: {e:#}"))?;
    cipher::encrypt(key, &json)
}

/// Decrypt data: AES-256-GCM decrypt → deserialize.
pub fn decrypt_data<T: serde::de::DeserializeOwned>(data: &[u8], key: &[u8; 32]) -> Result<T> {
    let decrypted = cipher::decrypt(key, data)?;
    serde_json::from_slice(&decrypted).map_err(|e| anyhow::anyhow!("JSON deserialize error: {e:#}"))
}

/// Try to decrypt data using the file key first; on failure, fall back to the keyring key.
pub fn try_decrypt_data<T: serde::de::DeserializeOwned>(data: &[u8]) -> Result<T> {
    // 1. Try file key first
    if let Some(key) = load_key_from_file() {
        if let Ok(result) = decrypt_data::<T>(data, &key) {
            return Ok(result);
        }
        tracing::debug!("File key failed to decrypt, trying keyring key…");
    }

    // 2. Fall back to keyring key (in case file key is stale or unavailable)
    if let Some(key) = load_key_from_keyring() {
        if let Ok(result) = decrypt_data::<T>(data, &key) {
            return Ok(result);
        }
        tracing::debug!("Keyring key also failed to decrypt");
    }

    Err(anyhow::anyhow!("解密数据失败（未找到有效密钥）"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    // -----------------------------------------------------------------------
    // encode_key / decode_key
    // -----------------------------------------------------------------------

    #[test]
    fn encode_decode_roundtrip() {
        let key = generate_random_key();
        let encoded = encode_key(&key);
        let decoded = decode_key(&encoded).unwrap();
        assert_eq!(key, decoded);
    }

    #[test]
    fn decode_invalid_base64_fails() {
        assert!(decode_key("not-valid-base64!!!").is_err());
    }

    #[test]
    fn decode_wrong_length_fails() {
        // Valid base64 but only 16 bytes, not 32
        let short = base64::prelude::BASE64_STANDARD.encode([0u8; 16]);
        assert!(decode_key(&short).is_err());
    }

    #[test]
    fn decode_trims_whitespace() {
        let key = generate_random_key();
        let encoded = format!("  {}  \n", encode_key(&key));
        let decoded = decode_key(encoded.trim()).unwrap();
        assert_eq!(key, decoded);
    }

    // -----------------------------------------------------------------------
    // generate_random_key
    // -----------------------------------------------------------------------

    #[test]
    fn random_keys_are_unique() {
        let a = generate_random_key();
        let b = generate_random_key();
        assert_ne!(a, b);
    }

    #[test]
    fn random_key_is_32_bytes() {
        let key = generate_random_key();
        assert_eq!(key.len(), 32);
    }

    // -----------------------------------------------------------------------
    // encrypt_data / decrypt_data
    // -----------------------------------------------------------------------

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct TestPayload {
        name: String,
        value: u64,
    }

    #[test]
    fn encrypt_decrypt_data_roundtrip() {
        let key = generate_random_key();
        let payload = TestPayload {
            name: "test".into(),
            value: 42,
        };

        let encrypted = encrypt_data(&payload, &key).unwrap();
        let decrypted: TestPayload = decrypt_data(&encrypted, &key).unwrap();

        assert_eq!(payload, decrypted);
    }

    #[test]
    fn encrypt_decrypt_data_with_slice() {
        let key = generate_random_key();
        let items = vec![
            TestPayload {
                name: "a".into(),
                value: 1,
            },
            TestPayload {
                name: "b".into(),
                value: 2,
            },
        ];

        let encrypted = encrypt_data(&items, &key).unwrap();
        let decrypted: Vec<TestPayload> = decrypt_data(&encrypted, &key).unwrap();

        assert_eq!(items, decrypted);
    }

    #[test]
    fn decrypt_data_with_wrong_key_fails() {
        let key1 = generate_random_key();
        let key2 = generate_random_key();
        let payload = TestPayload {
            name: "secret".into(),
            value: 99,
        };

        let encrypted = encrypt_data(&payload, &key1).unwrap();
        assert!(decrypt_data::<TestPayload>(&encrypted, &key2).is_err());
    }

    #[test]
    fn decrypt_data_with_corrupted_data_fails() {
        let key = generate_random_key();
        assert!(decrypt_data::<TestPayload>(b"garbage", &key).is_err());
    }

    #[test]
    fn encrypt_decrypt_empty_vec() {
        let key = generate_random_key();
        let items: Vec<TestPayload> = vec![];

        let encrypted = encrypt_data(&items, &key).unwrap();
        let decrypted: Vec<TestPayload> = decrypt_data(&encrypted, &key).unwrap();

        assert!(decrypted.is_empty());
    }
}
