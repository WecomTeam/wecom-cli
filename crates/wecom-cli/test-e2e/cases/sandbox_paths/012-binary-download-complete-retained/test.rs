// Process-level: 完整二进制下载落盘且大小与 body 一致——回退只在失败路径
// 触发，成功产物必须完整保留。
#[cfg(feature = "custom-endpoint")]
#[test]
#[allow(clippy::disallowed_methods)] // Test fixture: probe/fixture files directly, not through the sandbox
fn run() {
    let cwd = tempfile::tempdir().unwrap();
    let cfg_dir = tempfile::tempdir().unwrap();
    seed_credentials(cfg_dir.path());

    let body = vec![0x5Au8; 4096];

    let rt = tokio::runtime::Runtime::new().unwrap();
    let (server_url, method, _keep) = rt.block_on(async {
        let mut server = Server::new_async().await;
        let (catalog, hr) = setup_discovery_mocks(&mut server).await;
        let method = server
            .mock("POST", "/department/list")
            .with_status(200)
            .with_header("content-type", "application/octet-stream")
            .with_header(
                "content-disposition",
                "attachment; filename=\"complete.bin\"",
            )
            .with_body(body.clone())
            .expect(1)
            .create_async()
            .await;
        (server.url(), method, (server, catalog, hr))
    });

    let assert = assert_cmd::Command::cargo_bin("wecom-cli")
        .unwrap()
        .current_dir(cwd.path())
        .env("WECOM_CLI_BASE_URL", &server_url)
        .env("WECOM_CLI_CONFIG_DIR", cfg_dir.path())
        .args([
            "hr",
            "department",
            "list",
            "--id",
            "root",
            "--output",
            "./saved.bin",
        ])
        .assert();

    let output = assert.success().get_output().clone();
    let stdout: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(stdout["content_type"], "application/octet-stream");
    assert_eq!(stdout["size"].as_u64(), Some(body.len() as u64));

    let saved =
        std::fs::read(cwd.path().join("saved.bin")).expect("a complete download must be kept");
    assert_eq!(saved, body, "saved bytes must match the response body");

    method.assert();
}
