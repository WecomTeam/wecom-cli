//! 旧版凭据自动迁移（legacy `bot.enc`/`token.enc` → `credentials.enc`）。
//!
//! 历史版本将 bot 凭据与 token 分存于独立加密文件 `bot.enc` / `token.enc`；
//! 现版本收敛为单一 `credentials.enc`。本模块在启动时检测旧文件：无新凭据
//! 文件但存在 `bot.enc` 时，读取旧 botid/secret 自动走 auth 引导换取
//! Bearer token，落盘为新格式（`credentials.enc`）。旧文件**不主动清理**：
//! 残留无功能影响，读取全走新文件，同密钥加密无安全差异。
//!
//! 失败策略：除最终落盘 IO 错误向上传播外，一切迁移语义失败（文件缺失 /
//! 解密失败 / 网络 / 业务错误 / 无 token）均静默降级（仅日志，不提示用户），
//! legacy 文件保留、下次启动重试——与「未授权启动」表现一致。

use std::fs;

use wecom_transport::Transport;

use crate::bootstrap::{BindSource, fetch_auth};
use crate::bot::BotCredential;
use crate::credentials::{CredentialStore, Credentials};
use crate::crypto;
use crate::error::AuthError;
use crate::file_store::EncryptedFileCredentialStore;

/// 启动时尝试迁移旧版凭据（`bot.enc` → `credentials.enc`）。
///
/// `endpoint` 为鉴权引导端点（由调用方经 [`crate::gateway::auth_endpoint`]
/// 装配后传入，测试可传指向 mock server 的端点）；`store` 为加密文件凭据
/// 存储（迁移目标 `credentials.enc` 所在目录由它决定）。
///
/// 返回是否发生了迁移；迁移语义失败一律返回 `Ok(false)`（静默降级，见模块 doc）。
///
/// # Errors
///
/// 仅当最终落盘（[`CredentialStore::save`](crate::credentials::CredentialStore::save)）
/// 失败时返回 `Err`（本地 IO 属系统级异常）。
///
/// 读取旧凭据文件为 app 内部存储，不经沙箱。
#[allow(clippy::disallowed_methods)]
pub async fn try_migrate_legacy_credentials(
    store: &EncryptedFileCredentialStore,
    transport: &Transport,
    auth_endpoint: &wecom_transport::Endpoint,
) -> Result<bool, AuthError> {
    // 1. 已有新凭据 → 不迁移（不碰 legacy 文件）。
    if store.credentials_path().exists() {
        return Ok(false);
    }

    // 2. 无 bot.enc（token-only 无法自动 auth）→ 不迁移、不清理。
    let bot_path = store.legacy_paths()[0].clone();
    if !bot_path.exists() {
        return Ok(false);
    }

    // 3. 读取并解密旧 bot 凭据。
    let data = match fs::read(&bot_path) {
        Ok(data) => data,
        Err(e) => {
            tracing::warn!(path = %bot_path.display(), error = %e, "read legacy bot.enc failed");
            return Ok(false);
        }
    };
    let bot: BotCredential = match crypto::try_decrypt_data(
        store.dir(),
        store.keyring_enabled(),
        &data,
    ) {
        Ok(bot) => bot,
        Err(e) => {
            tracing::warn!(path = %bot_path.display(), error = %e, "decrypt legacy bot.enc failed");
            return Ok(false);
        }
    };

    // 4. 自动 auth：botid+secret 签名换取 Bearer token。
    let resp = match fetch_auth(transport, &bot, BindSource::Interactive, auth_endpoint).await {
        Ok(resp) => resp,
        Err(e) => {
            tracing::warn!(error = %e, "auto auth bootstrap failed during legacy migration");
            return Ok(false);
        }
    };

    // 5. 服务端未返回 token → 不落盘（legacy 保留，下次重试）。
    let Some(token) = resp.token.filter(|t| !t.is_empty()) else {
        tracing::warn!("legacy migration: auth response missing access token");
        return Ok(false);
    };

    // 6. 落盘新格式（旧文件不主动清理）。
    let creds = Credentials {
        bot: Some(bot),
        token: Some(token),
    };
    store.save(&creds)?;

    tracing::info!("legacy credentials migrated to credentials.enc");
    Ok(true)
}

#[cfg(test)]
mod tests {
    //! ## 模块摘要：legacy_migration（旧版凭据自动迁移）
    //!
    //! ### 关键接口
    //! - [try_migrate_legacy_credentials] — 无 credentials.enc 且存在 bot.enc 时：
    //!   解密旧凭据 → fetch_auth 换 token → save_credentials 落盘（legacy 保留）
    //!
    //! ### 关键分支与异常路径
    //! - 已有 credentials.enc → Ok(false)，legacy 保留
    //! - 无任何凭据 / 仅 token.enc → Ok(false)，文件保留
    //! - bot.enc 读取/解密失败 → Ok(false)，legacy 保留
    //! - fetch_auth 失败（业务 errcode）→ Ok(false)，legacy 保留
    //! - 成功 → Ok(true)，credentials.enc 生成、legacy 保留、可读回

    use base64::prelude::*;
    use serde_json::json;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use wecom_transport::HttpTransportBackend;

    use super::*;
    use crate::gateway::auth_endpoint;

    /// 手动写 `.encryption_key`（base64 编码），使文件 key 就位、隔离 keyring。
    fn write_key(dir: &std::path::Path, key: &[u8; 32]) {
        #[allow(clippy::disallowed_methods)] // 测试写入临时目录。
        std::fs::write(dir.join(".encryption_key"), BASE64_STANDARD.encode(key)).unwrap();
    }

    /// 用给定密钥加密 bot 并写入 `bot.enc`（模拟旧版凭据）。
    fn write_legacy_bot(dir: &std::path::Path, bot: &BotCredential, key: &[u8; 32]) {
        let data = crypto::encrypt_data(bot, key).unwrap();
        #[allow(clippy::disallowed_methods)] // 测试写入临时目录。
        std::fs::write(dir.join("bot.enc"), data).unwrap();
    }

    /// 构造指向 mock 服务器的裸 Transport。
    fn test_transport(base_url: &str) -> Transport {
        HttpTransportBackend::builder()
            .base_url(base_url)
            .build()
            .expect("valid transport")
    }

    fn store(dir: &std::path::Path) -> EncryptedFileCredentialStore {
        EncryptedFileCredentialStore::new(dir).with_keyring(false)
    }

    /// P0：无 credentials.enc + 有 bot.enc + 引导端点返回 token → 迁移成功
    /// 条件：预置旧 bot.enc（含 botid/secret），wiremock 返回 token
    /// 断言：返回 true；credentials.enc 生成；legacy 文件**保留**；
    ///       后续 load_credentials/load_token 可读回新凭据
    #[tokio::test]
    async fn migrates_legacy_credentials() {
        let dir = tempfile::tempdir().unwrap();
        let store = store(dir.path());
        let key = crypto::generate_random_key();
        write_key(dir.path(), &key);
        let bot = BotCredential::new("bot-legacy".into(), "secret-legacy".into());
        write_legacy_bot(dir.path(), &bot, &key);
        // 预置旧 token.enc（迁移不读取，仅验证不主动清理）。
        #[allow(clippy::disallowed_methods)] // 测试写入临时目录。
        std::fs::write(dir.path().join("token.enc"), b"legacy-token").unwrap();

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/get_cli_config"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "errcode": 0,
                "errmsg": "ok",
                "token": "tok-migrated",
            })))
            .expect(1)
            .mount(&server)
            .await;

        let transport = test_transport(&server.uri());
        let migrated = try_migrate_legacy_credentials(
            &store,
            &transport,
            &auth_endpoint(&format!("{}/get_cli_config", server.uri())),
        )
        .await
        .unwrap();

        assert!(migrated, "expected migration to succeed");
        assert!(dir.path().join("credentials.enc").exists());
        // 不主动清理 legacy 文件。
        assert!(dir.path().join("bot.enc").exists(), "legacy bot.enc kept");
        assert!(
            dir.path().join("token.enc").exists(),
            "legacy token.enc kept"
        );

        // 后续流程可读回新凭据。
        let creds = store.load().unwrap().expect("credentials readable");
        assert_eq!(creds.bot.as_ref().unwrap().id, "bot-legacy");
        assert_eq!(creds.token.as_deref(), Some("tok-migrated"));

        server.verify().await;
    }

    /// P0：已有 credentials.enc → 不触发迁移、legacy 文件原样保留
    /// 条件：先保存新凭据，再手动放置 legacy bot.enc
    /// 断言：返回 false；bot.enc 仍存在
    #[tokio::test]
    async fn existing_credentials_skips_migration() {
        let dir = tempfile::tempdir().unwrap();
        let store = store(dir.path());
        write_key(dir.path(), &crypto::generate_random_key());
        let creds = Credentials {
            bot: Some(BotCredential::new("bot-new".into(), "secret-new".into())),
            token: Some("tok-new".into()),
        };
        store.save(&creds).unwrap();

        // 模拟遗留文件（在 save_credentials 之后手动放置，避免被其清理）。
        #[allow(clippy::disallowed_methods)]
        std::fs::write(dir.path().join("bot.enc"), b"legacy").unwrap();

        let transport = test_transport("http://localhost");
        let migrated = try_migrate_legacy_credentials(
            &store,
            &transport,
            &auth_endpoint("http://localhost/get_cli_config"),
        )
        .await
        .unwrap();

        assert!(!migrated, "expected no migration");
        assert!(dir.path().join("bot.enc").exists(), "legacy file untouched");
    }

    /// P0：无任何凭据 → 返回 false、无副作用
    /// 条件：空配置目录
    /// 断言：返回 false；不生成 credentials.enc；无 legacy 文件
    #[tokio::test]
    async fn no_credentials_noop() {
        let dir = tempfile::tempdir().unwrap();
        let store = store(dir.path());

        let transport = test_transport("http://localhost");
        let migrated = try_migrate_legacy_credentials(
            &store,
            &transport,
            &auth_endpoint("http://localhost/get_cli_config"),
        )
        .await
        .unwrap();

        assert!(!migrated);
        assert!(!dir.path().join("credentials.enc").exists());
        assert!(!dir.path().join("bot.enc").exists());
    }

    /// P1：解密失败（密钥不符）→ 返回 false、legacy 保留
    /// 条件：用密钥 A 加密 bot.enc，但 .encryption_key 写入密钥 B
    /// 断言：返回 false；bot.enc 仍存在
    #[tokio::test]
    async fn decrypt_failure_keeps_legacy() {
        let dir = tempfile::tempdir().unwrap();
        let store = store(dir.path());
        let key_a = crypto::generate_random_key();
        let key_b = crypto::generate_random_key();
        write_key(dir.path(), &key_b);
        let bot = BotCredential::new("bot-legacy".into(), "secret-legacy".into());
        write_legacy_bot(dir.path(), &bot, &key_a);

        let transport = test_transport("http://localhost");
        let migrated = try_migrate_legacy_credentials(
            &store,
            &transport,
            &auth_endpoint("http://localhost/get_cli_config"),
        )
        .await
        .unwrap();

        assert!(!migrated, "expected silent downgrade");
        assert!(dir.path().join("bot.enc").exists(), "legacy file kept");
    }

    /// P1：fetch_auth 失败（业务 errcode）→ 返回 false、legacy 保留
    /// 条件：预置合法 bot.enc，wiremock 返回 errcode != 0
    /// 断言：返回 false；bot.enc 仍存在；mock 被命中一次
    #[tokio::test]
    async fn fetch_auth_failure_keeps_legacy() {
        let dir = tempfile::tempdir().unwrap();
        let store = store(dir.path());
        let key = crypto::generate_random_key();
        write_key(dir.path(), &key);
        let bot = BotCredential::new("bot-legacy".into(), "secret-legacy".into());
        write_legacy_bot(dir.path(), &bot, &key);

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/get_cli_config"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "errcode": 853000,
                "errmsg": "invalid credential",
            })))
            .expect(1)
            .mount(&server)
            .await;

        let transport = test_transport(&server.uri());
        let migrated = try_migrate_legacy_credentials(
            &store,
            &transport,
            &auth_endpoint(&format!("{}/get_cli_config", server.uri())),
        )
        .await
        .unwrap();

        assert!(!migrated, "expected silent downgrade");
        assert!(dir.path().join("bot.enc").exists(), "legacy file kept");
        server.verify().await;
    }

    /// P1：仅 token.enc → 不迁移、保留文件
    /// 条件：仅预置 token.enc（无 bot.enc）
    /// 断言：返回 false；token.enc 仍存在
    #[tokio::test]
    async fn token_only_keeps_legacy() {
        let dir = tempfile::tempdir().unwrap();
        let store = store(dir.path());
        #[allow(clippy::disallowed_methods)]
        std::fs::write(dir.path().join("token.enc"), b"legacy-token").unwrap();

        let transport = test_transport("http://localhost");
        let migrated = try_migrate_legacy_credentials(
            &store,
            &transport,
            &auth_endpoint("http://localhost/get_cli_config"),
        )
        .await
        .unwrap();

        assert!(!migrated);
        assert!(dir.path().join("token.enc").exists(), "token-only kept");
    }
}
