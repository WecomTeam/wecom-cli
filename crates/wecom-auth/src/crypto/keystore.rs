//! 加密密钥管理：本地密钥文件（权威来源）+ 系统 keyring 回退。
//!
//! 密钥与凭证文件均位于调用方给定的配置目录；keyring user 名由目录路径
//! 规范化后 SHA-256 派生，多目录（沙箱）间互不串扰。

use std::fs;
use std::path::{Path, PathBuf};

use base64::prelude::*;
use rand::RngExt;
use sha2::{Digest, Sha256};

use crate::error::AuthError;

use super::cipher;

const KEYRING_SERVICE: &str = "wecom-cli";
const KEYRING_USER_PREFIX: &str = "encryption-key";

// ---------------------------------------------------------------------------
// Paths
// ---------------------------------------------------------------------------

/// Return the file path for the local encryption key fallback under `dir`.
pub fn encryption_key_path(dir: &Path) -> PathBuf {
    dir.join(".encryption_key")
}

// ---------------------------------------------------------------------------
// Encode / Decode
// ---------------------------------------------------------------------------

/// Encode a 32-byte key as a Base64 string.
pub(crate) fn encode_key(key: &[u8; 32]) -> String {
    BASE64_STANDARD.encode(key)
}

/// Decode a Base64 string into a 32-byte key, returning an error on invalid input.
pub(crate) fn decode_key(s: &str) -> Result<[u8; 32], AuthError> {
    let bytes = BASE64_STANDARD
        .decode(s)
        .map_err(|e| AuthError::Crypto(format!("加密密钥无效，base64 decode error: {e}")))?;
    <[u8; 32]>::try_from(bytes.as_slice())
        .map_err(|_| AuthError::Crypto("Invalid encryption key length".into()))
}

// ---------------------------------------------------------------------------
// Key generation / loading / saving
// ---------------------------------------------------------------------------

/// Generate a fresh random 256-bit key.
pub fn generate_random_key() -> [u8; 32] {
    rand::rng().random()
}

/// 由配置目录计算 keyring user 名（路径规范化后 SHA-256 hex 后缀，隔离沙箱）。
pub(crate) fn keyring_user_for(dir: &Path) -> String {
    let dir = normalize_path(&std::path::absolute(dir).unwrap_or_else(|_| dir.to_path_buf()));
    let digest = Sha256::digest(dir.to_string_lossy().as_bytes());
    format!("{KEYRING_USER_PREFIX}:{}", hex::encode(digest))
}

/// 路径规范化：消除 `.`、`..` 组件（不访问文件系统）。`absolute` 不处理 `..`，需手动回退。
fn normalize_path(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for comp in path
        .components()
        .filter(|c| *c != std::path::Component::CurDir)
    {
        match comp {
            std::path::Component::ParentDir => {
                out.pop();
            }
            other => out.push(other.as_os_str()),
        }
    }
    out
}

/// Load the key from keyring. Returns `None` if unavailable.
pub(crate) fn load_key_from_keyring(dir: &Path) -> Option<[u8; 32]> {
    let user = keyring_user_for(dir);
    let entry = keyring::Entry::new(KEYRING_SERVICE, &user).ok()?;
    let b64 = entry.get_password().ok()?;
    decode_key(b64.trim()).ok()
}

/// Load the key from the file fallback. Returns `None` if unavailable.
#[allow(clippy::disallowed_methods)]
pub(crate) fn load_key_from_file(dir: &Path) -> Option<[u8; 32]> {
    let contents = fs::read_to_string(encryption_key_path(dir)).ok()?;
    decode_key(contents.trim()).ok()
}

/// Persist the key under `dir`: always write the file fallback, and the
/// keyring when `use_keyring` is enabled.
pub(crate) fn save_key(dir: &Path, key: &[u8; 32], use_keyring: bool) -> Result<(), AuthError> {
    let b64 = encode_key(key);

    // Always write the file fallback.
    let key_path = encryption_key_path(dir);
    super::atomic_write(&key_path, b64.as_bytes(), 0o600)?;

    if !use_keyring {
        return Ok(());
    }

    let user = keyring_user_for(dir);
    if let Err(e) =
        keyring::Entry::new(KEYRING_SERVICE, &user).and_then(|entry| entry.set_password(&b64))
    {
        tracing::warn!(error = %e, "keyring unavailable, encryption key stored in file only");
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Encrypt / Decrypt helpers for serializable data
// ---------------------------------------------------------------------------

/// Encrypt serializable data: serialize → AES-256-GCM encrypt.
pub fn encrypt_data<T: serde::Serialize + ?Sized>(
    data: &T,
    key: &[u8; 32],
) -> Result<Vec<u8>, AuthError> {
    let json = serde_json::to_vec(data)
        .map_err(|e| AuthError::Crypto(format!("JSON serialize error: {e:#}")))?;
    cipher::encrypt(key, &json)
}

/// Decrypt data: AES-256-GCM decrypt → deserialize.
pub fn decrypt_data<T: serde::de::DeserializeOwned>(
    data: &[u8],
    key: &[u8; 32],
) -> Result<T, AuthError> {
    let decrypted = cipher::decrypt(key, data)?;
    serde_json::from_slice(&decrypted)
        .map_err(|e| AuthError::Crypto(format!("JSON deserialize error: {e:#}")))
}

/// Decrypt data under `dir`: the file key is authoritative, with the keyring
/// as fallback (when `use_keyring` is enabled).
pub(crate) fn try_decrypt_data<T: serde::de::DeserializeOwned>(
    dir: &Path,
    use_keyring: bool,
    data: &[u8],
) -> Result<T, AuthError> {
    // 1. Try file key (authoritative source)
    if let Some(key) = load_key_from_file(dir) {
        if let Ok(result) = decrypt_data::<T>(data, &key) {
            return Ok(result);
        }
        tracing::debug!("File key failed to decrypt, falling back to keyring key");
    }

    // 2. Fall back to keyring key
    let key = load_key_from_keyring(dir)
        .filter(|_| use_keyring)
        .ok_or_else(|| AuthError::Crypto("解密数据失败（未找到有效密钥）".into()))?;
    decrypt_data(data, &key)
}

#[cfg(test)]
mod tests {
    //! ## 模块摘要：keystore（加密密钥存储与数据加解密）
    //!
    //! ### 关键接口
    //! - [encode_key] / [decode_key] — 32 字节密钥的 Base64 编解码
    //! - [generate_random_key] — 生成 256-bit 随机密钥
    //! - [encrypt_data] / [decrypt_data] — 任意可序列化数据的 AES-256-GCM 加解密
    //! - [try_decrypt_data] — 文件密钥优先、keyring 回退的解密入口
    //!
    //! ### 关键分支与异常路径
    //! - Base64 非法 / 长度非 32 字节 → decode 报错
    //! - 错误密钥 / 损坏密文 → 解密失败
    //! - 空 Vec / 切片数据 roundtrip 正常
    //! - 随机 nonce → 每次加密输出不同

    use super::*;
    use serde::{Deserialize, Serialize};

    // -----------------------------------------------------------------------
    // encode_key / decode_key
    // -----------------------------------------------------------------------

    /// P0：encode → decode roundtrip 还原密钥
    /// 条件：随机生成 32 字节密钥并 Base64 编码
    /// 断言：解码后与原始密钥相等
    #[test]
    fn encode_decode_roundtrip() {
        let key = generate_random_key();
        let encoded = encode_key(&key);
        let decoded = decode_key(&encoded).unwrap();
        assert_eq!(key, decoded);
    }

    /// P1：decode_key 边界输入（非法 base64 / 长度非 32 字节 / 首尾空白容忍）
    /// 条件：非法字符串、16 字节短密钥、带首尾空白的合法编码
    /// 断言：非法与短密钥 → Err；空白经 trim → Ok 还原
    #[test]
    fn decode_key_handles_edge_cases() {
        assert!(decode_key("not-valid-base64!!!").is_err());
        // Valid base64 but only 16 bytes, not 32
        let short = base64::prelude::BASE64_STANDARD.encode([0u8; 16]);
        assert!(decode_key(&short).is_err());
        // Leading/trailing whitespace is tolerated via trim.
        let key = generate_random_key();
        let encoded = format!("  {}  \n", encode_key(&key));
        let decoded = decode_key(encoded.trim()).unwrap();
        assert_eq!(key, decoded);
    }

    // -----------------------------------------------------------------------
    // generate_random_key
    // -----------------------------------------------------------------------

    /// P0：随机密钥为 32 字节且连续生成不重复
    /// 条件：调用两次 generate_random_key
    /// 断言：长度 32；两次结果不相等
    #[test]
    fn random_key_has_expected_properties() {
        let key = generate_random_key();
        assert_eq!(key.len(), 32);
        let another = generate_random_key();
        assert_ne!(key, another);
    }

    // -----------------------------------------------------------------------
    // keyring_user_for（沙箱隔离 keyring user 名）
    // -----------------------------------------------------------------------

    /// P0：keyring_user_for 映射性质（同路径一致、异路径隔离）
    /// 条件：同一路径两次调用；两个不同路径
    /// 断言：同路径相等；异路径不等
    #[test]
    fn keyring_user_for_deterministic_and_isolated() {
        let dir = Path::new("/tmp/wecom-sandbox-a");
        assert_eq!(keyring_user_for(dir), keyring_user_for(dir));
        let b = keyring_user_for(Path::new("/tmp/wecom-sandbox-b"));
        assert_ne!(keyring_user_for(dir), b);
    }

    /// P1：keyring_user_for 输出形态（规范化一致 + 格式正确）
    /// 条件：`/tmp/x` 与 `/tmp/./x`、`/tmp/a/../x`（纯绝对路径）；格式断言
    /// 断言：三种写法 user 一致；前缀 "encryption-key:" + 64 位小写 hex
    #[test]
    fn keyring_user_for_normalizes_and_formats() {
        let base = keyring_user_for(Path::new("/tmp/wecom-sandbox-a"));
        assert_eq!(base, keyring_user_for(Path::new("/tmp/./wecom-sandbox-a")));
        assert_eq!(
            base,
            keyring_user_for(Path::new("/tmp/wecom-sandbox-b/../wecom-sandbox-a"))
        );
        let suffix = base.strip_prefix("encryption-key:").unwrap();
        assert_eq!(suffix.len(), 64);
        assert!(
            suffix
                .bytes()
                .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase()),
            "expected lowercase hex, got: {suffix}"
        );
    }

    // -----------------------------------------------------------------------
    // encrypt_data / decrypt_data
    // -----------------------------------------------------------------------

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct TestPayload {
        name: String,
        value: u64,
    }

    /// P0：encrypt_data → decrypt_data roundtrip（对象 / 切片 / 空 Vec）
    /// 条件：同一密钥加密三类数据
    /// 断言：解密结果与原始负载相等（空 Vec 解为空）
    #[test]
    fn encrypt_decrypt_data_roundtrips() {
        let key = generate_random_key();

        let payload = TestPayload {
            name: "test".into(),
            value: 42,
        };
        let encrypted = encrypt_data(&payload, &key).unwrap();
        let decrypted: TestPayload = decrypt_data(&encrypted, &key).unwrap();
        assert_eq!(payload, decrypted);

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

        let empty: Vec<TestPayload> = vec![];
        let encrypted = encrypt_data(&empty, &key).unwrap();
        let decrypted: Vec<TestPayload> = decrypt_data(&encrypted, &key).unwrap();
        assert!(decrypted.is_empty());
    }

    /// P1：decrypt_data 拒绝无效密文（错误密钥 / 损坏数据）
    /// 条件：密钥 1 加密密钥 2 解密；非法密文
    /// 断言：均返回 Err
    #[test]
    fn decrypt_data_rejects_invalid() {
        let key1 = generate_random_key();
        let key2 = generate_random_key();
        let payload = TestPayload {
            name: "secret".into(),
            value: 99,
        };
        let encrypted = encrypt_data(&payload, &key1).unwrap();
        assert!(decrypt_data::<TestPayload>(&encrypted, &key2).is_err());

        let key = generate_random_key();
        assert!(decrypt_data::<TestPayload>(b"garbage", &key).is_err());
    }
}
