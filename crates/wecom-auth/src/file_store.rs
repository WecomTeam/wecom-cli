//! 加密文件凭据存储：AES-256-GCM 加密的 `credentials.enc` + keyring 回退。
//!
//! 适合本地开发机；容器 / 服务器部署建议改用
//! [`MemoryCredentialStore`](crate::credentials::MemoryCredentialStore) 配合
//! 环境变量 / Secret Manager 注入，或自行实现
//! [`CredentialStore`](crate::credentials::CredentialStore) 对接 KMS / Vault。

use std::fs;
use std::path::PathBuf;

use crate::credentials::{CredentialStore, Credentials};
use crate::crypto;
use crate::error::AuthError;

/// 加密文件凭据存储。
///
/// bot 信息与 Bearer token 共存于目录内单一加密文件 `credentials.enc`
/// （AES-256-GCM，密钥存 `.encryption_key` 文件 + 系统 keyring 回退）；
/// bot 与 token 均空时不落盘（删除既有文件，避免残留空凭据）。
#[derive(Debug, Clone)]
pub struct EncryptedFileCredentialStore {
    dir: PathBuf,
    use_keyring: bool,
}

impl EncryptedFileCredentialStore {
    /// 在 `dir` 下创建存储（keyring 默认启用）。
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self {
            dir: dir.into(),
            use_keyring: true,
        }
    }

    /// 凭据所在目录。
    pub fn dir(&self) -> &std::path::Path {
        &self.dir
    }

    /// keyring 回退是否启用。
    pub fn keyring_enabled(&self) -> bool {
        self.use_keyring
    }

    /// 是否使用系统 keyring 持久化加密密钥（默认启用）。
    ///
    /// 容器等无 keyring 环境可关闭；关闭后密钥仅存 `.encryption_key` 文件。
    #[must_use]
    pub fn with_keyring(mut self, enabled: bool) -> Self {
        self.use_keyring = enabled;
        self
    }

    /// 加密凭据文件路径（`<dir>/credentials.enc`）。
    pub fn credentials_path(&self) -> PathBuf {
        self.dir.join("credentials.enc")
    }

    /// 旧版独立凭据文件（`bot.enc` / `token.enc`）路径。
    ///
    /// 凭据统一存放于 `credentials.enc`；旧文件**不主动清理**——仅由
    /// [`try_migrate_legacy_credentials`](crate::legacy_migration) 在迁移时读取。
    pub fn legacy_paths(&self) -> [PathBuf; 2] {
        [self.dir.join("bot.enc"), self.dir.join("token.enc")]
    }

    /// 加密密钥文件路径（`<dir>/.encryption_key`）。
    pub fn encryption_key_path(&self) -> PathBuf {
        crypto::encryption_key_path(&self.dir)
    }

    /// 生成新的随机加密密钥。
    pub fn generate_key(&self) -> [u8; 32] {
        crypto::generate_random_key()
    }
}

impl CredentialStore for EncryptedFileCredentialStore {
    /// 读取凭据文件。文件缺失或解密失败（密文损坏 / 密钥不符）返回 `None`。
    #[allow(clippy::disallowed_methods)]
    fn load(&self) -> Result<Option<Credentials>, AuthError> {
        let path = self.credentials_path();
        let data = match fs::read(&path) {
            Ok(data) => data,
            // 凭据缺失不算错误：与「未授权」表现一致。
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => {
                return Err(AuthError::Storage(format!(
                    "读取凭据文件 {} 失败: {e}",
                    path.display()
                )));
            }
        };
        let creds = crypto::try_decrypt_data(&self.dir, self.use_keyring, &data)
            .inspect_err(|e| {
                tracing::warn!(path = %path.display(), error = %e, "failed to decrypt credentials");
            })
            .ok();
        Ok(creds)
    }

    /// 加密并持久化凭据；bot 与 token 均空时删除凭据文件。
    fn save(&self, creds: &Credentials) -> Result<(), AuthError> {
        if creds.bot.is_none() && creds.token.is_none() {
            return self.clear();
        }
        let key = crypto::load_key_from_file(&self.dir)
            .or_else(|| crypto::load_key_from_keyring(&self.dir).filter(|_| self.use_keyring))
            .unwrap_or_else(|| {
                let k = crypto::generate_random_key();
                tracing::info!("generated a new encryption key");
                k
            });
        crypto::save_key(&self.dir, &key, self.use_keyring)?;
        let encrypted = crypto::encrypt_data(creds, &key)?;
        crypto::atomic_write(&self.credentials_path(), &encrypted, 0o600)?;
        tracing::info!("credentials saved");
        Ok(())
    }

    /// 删除凭据文件（不存在时 no-op）。
    #[allow(clippy::disallowed_methods)]
    fn clear(&self) -> Result<(), AuthError> {
        let path = self.credentials_path();
        if path.exists() {
            fs::remove_file(&path).map_err(|e| {
                AuthError::Storage(format!("删除凭据文件 {} 失败: {e}", path.display()))
            })?;
            tracing::info!("credentials file removed: {}", path.display());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    //! ## 模块摘要：file_store（加密文件凭据存储）
    //!
    //! ### 关键接口
    //! - [EncryptedFileCredentialStore] — `credentials.enc` 加密总账实现 [CredentialStore]
    //! - [EncryptedFileCredentialStore::with_keyring] — 关闭 keyring（容器 / 测试）
    //!
    //! ### 关键分支与异常路径
    //! - 文件缺失 / 密文损坏 / 密钥不符 → load 返回 None
    //! - bot 与 token 均空 → save 删除文件而非写入
    //! - 独立 `bot.enc` / `token.enc` → 永不主动删除，仅迁移读取

    use base64::Engine as _;

    use super::*;

    /// 创建 keyring 关闭的临时存储（隔离用户 keyring）。
    fn store(dir: &std::path::Path) -> EncryptedFileCredentialStore {
        EncryptedFileCredentialStore::new(dir).with_keyring(false)
    }

    fn write_key(dir: &std::path::Path, key: &[u8; 32]) {
        #[allow(clippy::disallowed_methods)] // 测试写入临时目录。
        std::fs::write(
            dir.join(".encryption_key"),
            base64::prelude::BASE64_STANDARD.encode(key),
        )
        .unwrap();
    }

    fn bot(id: &str) -> crate::bot::BotCredential {
        crate::bot::BotCredential::new(id.to_string(), "secret".into())
    }

    /// P0：bot + token 保存后可完整读回
    #[test]
    fn save_load_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let store = store(dir.path());
        write_key(dir.path(), &store.generate_key());

        let creds = Credentials {
            bot: Some(bot("bot1")),
            token: Some("tok-1".into()),
        };
        store.save(&creds).unwrap();
        let loaded = store.load().unwrap().unwrap();
        assert_eq!(loaded.bot.as_ref().map(|b| b.id.as_str()), Some("bot1"));
        assert_eq!(loaded.token.as_deref(), Some("tok-1"));
    }

    /// P0：bot 与 token 均空时删除凭据文件
    #[test]
    fn save_empty_creds_deletes_file() {
        let dir = tempfile::tempdir().unwrap();
        let store = store(dir.path());
        write_key(dir.path(), &store.generate_key());

        store.save(&Credentials::default()).unwrap();
        assert!(!dir.path().join("credentials.enc").exists());
    }

    /// P0：bot 与 token 可独立更新互不影响
    #[test]
    fn bot_and_token_independent() {
        let dir = tempfile::tempdir().unwrap();
        let store = store(dir.path());
        write_key(dir.path(), &store.generate_key());

        let mut c = store.load().unwrap().unwrap_or_default();
        c.bot = Some(bot("bot2"));
        store.save(&c).unwrap();
        let mut c = store.load().unwrap().unwrap_or_default();
        c.token = Some("tok-2".into());
        store.save(&c).unwrap();
        let loaded = store.load().unwrap().unwrap();
        assert_eq!(loaded.bot.as_ref().map(|b| b.id.as_str()), Some("bot2"));
        assert_eq!(loaded.token.as_deref(), Some("tok-2"));
    }

    /// P0：clear 删除已存在的凭据文件；对缺失文件 no-op
    #[test]
    fn clear_deletes_file_and_is_noop_when_missing() {
        let dir = tempfile::tempdir().unwrap();
        let store = store(dir.path());
        write_key(dir.path(), &store.generate_key());

        let c = Credentials {
            bot: Some(bot("bot4")),
            token: Some("tok-4".into()),
        };
        store.save(&c).unwrap();
        store.clear().unwrap();
        assert!(!dir.path().join("credentials.enc").exists());
        store.clear().unwrap();
    }

    /// P0：无 credentials.enc 时 load 不清除 legacy（迁移失败场景，legacy 是唯一凭据来源）
    #[test]
    fn legacy_files_kept_when_no_credentials() {
        let dir = tempfile::tempdir().unwrap();
        let store = store(dir.path());
        write_key(dir.path(), &store.generate_key());
        #[allow(clippy::disallowed_methods)] // 测试写入临时目录。
        std::fs::write(dir.path().join("bot.enc"), b"legacy-bot").unwrap();
        #[allow(clippy::disallowed_methods)] // 测试写入临时目录。
        std::fs::write(dir.path().join("token.enc"), b"legacy-token").unwrap();

        assert!(store.load().unwrap().is_none());
        assert!(dir.path().join("bot.enc").exists(), "legacy must be kept");
        assert!(dir.path().join("token.enc").exists(), "legacy must be kept");
    }

    /// P0：credentials.enc 已就位时 load/save 也不清理 legacy
    #[test]
    fn legacy_files_kept_even_with_credentials() {
        let dir = tempfile::tempdir().unwrap();
        let store = store(dir.path());
        write_key(dir.path(), &store.generate_key());

        let c = Credentials {
            bot: Some(bot("bot1")),
            token: Some("tok-1".into()),
        };
        store.save(&c).unwrap();
        // save 后手动放置 legacy（模拟迁移完成后仍残留的场景）。
        #[allow(clippy::disallowed_methods)] // 测试写入临时目录。
        std::fs::write(dir.path().join("bot.enc"), b"legacy-bot").unwrap();

        assert!(store.load().unwrap().is_some());
        assert!(dir.path().join("bot.enc").exists(), "legacy must be kept");
    }

    /// P1：凭据文件缺失时 load 返回 None
    #[test]
    fn load_missing_returns_none() {
        let dir = tempfile::tempdir().unwrap();
        let store = store(dir.path());
        assert!(store.load().unwrap().is_none());
    }

    /// P1：凭据文件内容损坏时 load 返回 None
    #[test]
    fn load_corrupted_returns_none() {
        let dir = tempfile::tempdir().unwrap();
        let store = store(dir.path());
        write_key(dir.path(), &store.generate_key());
        #[allow(clippy::disallowed_methods)] // 测试写入临时目录。
        std::fs::write(dir.path().join("credentials.enc"), b"garbage").unwrap();
        assert!(store.load().unwrap().is_none());
    }

    /// P1：密钥不符时 load 返回 None（密文无法解密）
    #[test]
    fn load_wrong_key_returns_none() {
        let dir = tempfile::tempdir().unwrap();
        let store = store(dir.path());
        write_key(dir.path(), &store.generate_key());

        let c = Credentials {
            bot: Some(bot("bot1")),
            token: Some("tok-secret".into()),
        };
        store.save(&c).unwrap();

        write_key(dir.path(), &store.generate_key()); // 替换密钥 → 密文无法解密
        assert!(store.load().unwrap().is_none());
    }
}
