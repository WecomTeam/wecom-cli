use aes_gcm::aead::Aead;
use aes_gcm::{Aes256Gcm, KeyInit};
use rand::Rng;

use crate::error::AuthError;

/// AES-GCM nonce size (96 bits).
const NONCE_SIZE: usize = 12;
/// AES-GCM authentication tag size (128 bits).
const TAG_SIZE: usize = 16;

/// Encrypt `plaintext` with AES-256-GCM. Returns `nonce || ciphertext || tag`.
pub fn encrypt(key: &[u8; 32], plaintext: &[u8]) -> Result<Vec<u8>, AuthError> {
    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|e| AuthError::Crypto(format!("数据加密失败：{e}")))?;

    let mut nonce_bytes = [0u8; NONCE_SIZE];
    rand::rng().fill_bytes(&mut nonce_bytes);
    let nonce = aes_gcm::Nonce::from(nonce_bytes);

    let ciphertext = cipher
        .encrypt(&nonce, plaintext)
        .map_err(|e| AuthError::Crypto(format!("数据加密失败：{e}")))?;

    let mut out = nonce_bytes.to_vec();
    out.extend(ciphertext);
    Ok(out)
}

/// Decrypt `data` (expected format: `nonce || ciphertext || tag`) with AES-256-GCM.
pub fn decrypt(key: &[u8; 32], data: &[u8]) -> Result<Vec<u8>, AuthError> {
    if data.len() < NONCE_SIZE + TAG_SIZE {
        return Err(AuthError::Crypto(
            "数据解密失败（数据可能已损坏或被截断）".into(),
        ));
    }
    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|e| AuthError::Crypto(format!("数据解密失败：{e}")))?;

    let mut nonce_bytes = [0u8; NONCE_SIZE];
    nonce_bytes.copy_from_slice(&data[..NONCE_SIZE]);
    let nonce = aes_gcm::Nonce::from(nonce_bytes);

    cipher
        .decrypt(&nonce, &data[NONCE_SIZE..])
        .map_err(|e| AuthError::Crypto(format!("数据解密失败：{e}")))
}

#[cfg(test)]
mod tests {
    //! ## 模块摘要：cipher（AES-256-GCM 原始加解密原语）
    //!
    //! ### 关键接口
    //! - [encrypt] — 加密，输出 `nonce || ciphertext || tag`（12 + 明文 + 16 字节）
    //! - [decrypt] — 解密 `nonce || ciphertext || tag` 格式数据
    //! - [NONCE_SIZE] / [TAG_SIZE] — GCM 参数常量（12 / 16）
    //!
    //! ### 关键分支与异常路径
    //! - 数据过短（< 28 字节）→ decrypt 直接报错
    //! - 错误密钥 / 密文被篡改 → 解密失败（GCM 认证失败）
    //! - 空明文 roundtrip 正常
    //! - 随机 nonce → 相同明文每次加密输出不同

    use super::*;
    use crate::crypto::keystore::generate_random_key;

    /// P0：encrypt → decrypt roundtrip 还原明文
    /// 条件：用同一密钥加密明文
    /// 断言：解密结果与明文相等
    #[test]
    fn encrypt_decrypt_roundtrip() {
        let key = generate_random_key();
        let plaintext = b"hello, AES-256-GCM!";

        let encrypted = encrypt(&key, plaintext).unwrap();
        let decrypted = decrypt(&key, &encrypted).unwrap();

        assert_eq!(decrypted, plaintext);
    }

    /// P1：空明文 roundtrip 正常
    /// 条件：加密空字节串
    /// 断言：解密结果为空字节串
    #[test]
    fn encrypt_decrypt_empty_plaintext() {
        let key = generate_random_key();

        let encrypted = encrypt(&key, b"").unwrap();
        let decrypted = decrypt(&key, &encrypted).unwrap();

        assert_eq!(decrypted, b"");
    }

    /// P0：密文长度为 nonce + 明文 + tag
    /// 条件：加密 "test data"（9 字节）
    /// 断言：密文长度 == 12 + 9 + 16 == 37
    #[test]
    fn encrypted_output_has_expected_length() {
        let key = generate_random_key();
        let plaintext = b"test data";

        let encrypted = encrypt(&key, plaintext).unwrap();
        // nonce (12) + plaintext (9) + tag (16) = 37
        assert_eq!(encrypted.len(), NONCE_SIZE + plaintext.len() + TAG_SIZE);
    }

    /// P1：错误密钥解密失败
    /// 条件：用密钥 1 加密、密钥 2 解密
    /// 断言：decrypt 返回 Err（GCM 认证失败）
    #[test]
    fn decrypt_with_wrong_key_fails() {
        let key1 = generate_random_key();
        let key2 = generate_random_key();

        let encrypted = encrypt(&key1, b"secret").unwrap();
        assert!(decrypt(&key2, &encrypted).is_err());
    }

    /// P1：数据过短时解密报错
    /// 条件：输入长度小于 NONCE_SIZE + TAG_SIZE 的数据
    /// 断言：decrypt 返回 Err
    #[test]
    fn decrypt_too_short_data_fails() {
        let key = generate_random_key();

        // Less than NONCE_SIZE + TAG_SIZE = 28
        assert!(decrypt(&key, &[0u8; 27]).is_err());
        assert!(decrypt(&key, &[]).is_err());
        assert!(decrypt(&key, &[0u8; 11]).is_err());
    }

    /// P1：密文被篡改时解密失败
    /// 条件：翻转密文末尾一个字节
    /// 断言：decrypt 返回 Err（认证失败）
    #[test]
    fn decrypt_corrupted_data_fails() {
        let key = generate_random_key();
        let encrypted = encrypt(&key, b"important data").unwrap();

        // Flip a byte in the ciphertext portion
        let mut corrupted = encrypted.clone();
        let last = corrupted.len() - 1;
        corrupted[last] ^= 0xFF;

        assert!(decrypt(&key, &corrupted).is_err());
    }

    /// P1：随机 nonce 使相同明文每次加密输出不同
    /// 条件：对同一明文加密两次
    /// 断言：两次密文不同，但都能解密回同一明文
    #[test]
    fn each_encryption_produces_different_output() {
        let key = generate_random_key();
        let plaintext = b"same plaintext";

        let a = encrypt(&key, plaintext).unwrap();
        let b = encrypt(&key, plaintext).unwrap();

        // Different nonces → different ciphertext
        assert_ne!(a, b);
        // But both decrypt to the same plaintext
        assert_eq!(decrypt(&key, &a).unwrap(), plaintext);
        assert_eq!(decrypt(&key, &b).unwrap(), plaintext);
    }
}
