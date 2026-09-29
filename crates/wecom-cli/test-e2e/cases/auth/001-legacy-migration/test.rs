// Process-level: legacy bot.enc auto-migration on startup.
//
// 预置旧版凭据（bot.enc + .encryption_key，无 credentials.enc），
// 启动 CLI（auth show 触发 transport::build → 自动迁移）：
// 读取旧 botid/secret → 引导端点换 token → 落盘 credentials.enc。
// 旧文件不主动清理。
#[cfg(all(feature = "custom-endpoint", not(feature = "skip-auth-check")))]
use aes_gcm::KeyInit;
#[cfg(all(feature = "custom-endpoint", not(feature = "skip-auth-check")))]
use aes_gcm::aead::Aead;
#[cfg(all(feature = "custom-endpoint", not(feature = "skip-auth-check")))]
use base64::Engine;
#[cfg(all(feature = "custom-endpoint", not(feature = "skip-auth-check")))]
use std::path::Path;

/// 预置旧版凭据：`.encryption_key` + `bot.enc`（AES-256-GCM：nonce(12) ‖ ciphertext(含 tag16)）。
#[cfg(all(feature = "custom-endpoint", not(feature = "skip-auth-check")))]
fn setup_legacy_bot(dir: &Path, key: &[u8; 32]) {
    #[allow(clippy::disallowed_methods)]
    std::fs::write(
        dir.join(".encryption_key"),
        base64::prelude::BASE64_STANDARD.encode(key),
    )
    .unwrap();

    let bot = serde_json::json!({
        "id": "bot-e2e",
        "secret": "secret-e2e",
        "create_time": 0,
    });
    let plaintext = serde_json::to_vec(&bot).unwrap();
    let cipher = aes_gcm::Aes256Gcm::new_from_slice(key).unwrap();
    let nonce_bytes = [0u8; 12];
    let nonce = aes_gcm::Nonce::from(nonce_bytes);
    let ciphertext = cipher
        .encrypt(&nonce, plaintext.as_ref())
        .expect("encrypt legacy bot");
    let mut encrypted = nonce_bytes.to_vec();
    encrypted.extend(ciphertext);
    #[allow(clippy::disallowed_methods)]
    std::fs::write(dir.join("bot.enc"), encrypted).unwrap();
}

/// 启动 CLI 并返回输出。
#[cfg(all(feature = "custom-endpoint", not(feature = "skip-auth-check")))]
fn run_cli(dir: &Path, auth_endpoint: &str) -> std::process::Output {
    assert_cmd::Command::cargo_bin("wecom-cli")
        .unwrap()
        .env("WECOM_CLI_CONFIG_DIR", dir)
        .env("WECOM_CLI_AUTH_ENDPOINT", auth_endpoint)
        .args(["auth", "show"])
        .output()
        .unwrap()
}

/// P0：迁移成功路径——引导返回 token → 落盘 credentials.enc；legacy 保留。
#[cfg(all(feature = "custom-endpoint", not(feature = "skip-auth-check")))]
#[test]
fn migration_succeeds_keeps_legacy() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path();
        setup_legacy_bot(dir, &[7u8; 32]);

        // mock 引导端点：返回 token（FlatRes 整体 body 即业务结果）。
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("POST", "/get_cli_config")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"errcode":0,"errmsg":"ok","token":"tok-e2e"}"#)
            .expect(1)
            .create_async()
            .await;

        let output = run_cli(dir, &format!("{}/get_cli_config", server.url()));

        // 断言：命令成功、授权状态可见、凭据迁移落盘、legacy 保留、引导恰一次。
        assert_eq!(output.status.code(), Some(0));
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("Status: authorized"), "got: {stdout}");
        assert!(stdout.contains("Bot ID: bot-e2e"), "got: {stdout}");

        assert!(
            dir.join("credentials.enc").exists(),
            "credentials.enc should be created"
        );
        assert!(dir.join("bot.enc").exists(), "legacy bot.enc kept");

        mock.assert();
    });
}

/// P0：迁移失败路径——引导返回业务错误时静默降级
/// 条件：引导端点返回业务错误（errcode != 0）→ 迁移静默降级
/// 断言：退出码 0（未授权启动表现）、legacy bot.enc **保留**、credentials.enc 不生成
#[cfg(all(feature = "custom-endpoint", not(feature = "skip-auth-check")))]
#[test]
fn migration_failure_keeps_legacy() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path();
        setup_legacy_bot(dir, &[8u8; 32]);

        // mock 引导端点：返回业务错误。
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("POST", "/get_cli_config")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"errcode":853000,"errmsg":"invalid credential"}"#)
            .expect(1)
            .create_async()
            .await;

        let output = run_cli(dir, &format!("{}/get_cli_config", server.url()));

        // 迁移失败 → 按未授权启动：命令仍成功，状态 unauthorized。
        assert_eq!(output.status.code(), Some(0));
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("Status: unauthorized"), "got: {stdout}");

        // 核心断言：legacy 必须保留。
        assert!(
            dir.join("bot.enc").exists(),
            "legacy bot.enc must be kept on migration failure"
        );
        assert!(
            !dir.join("credentials.enc").exists(),
            "credentials.enc should not be created"
        );

        mock.assert();
    });
}
