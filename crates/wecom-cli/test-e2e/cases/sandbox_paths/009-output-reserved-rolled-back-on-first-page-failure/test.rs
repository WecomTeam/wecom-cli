// Process-level: 首页请求失败后 --output 预留文件被回退——只有真实二进制
// 入口才把「预留 → 未保留 → 回退」这条链路跑全。
#[cfg(feature = "custom-endpoint")]
#[test]
#[allow(clippy::disallowed_methods)] // Test fixture: probe/fixture files directly, not through the sandbox
fn run() {
    let cwd = tempfile::tempdir().unwrap();
    let cfg_dir = tempfile::tempdir().unwrap();
    seed_credentials(cfg_dir.path());

    let rt = tokio::runtime::Runtime::new().unwrap();
    let (server_url, method, _keep) = rt.block_on(async {
        let mut server = Server::new_async().await;
        let (catalog, hr) = setup_discovery_mocks(&mut server).await;
        let method = server
            .mock("POST", "/department/list")
            .with_status(500)
            .with_header("content-type", "application/json")
            .with_body(api_response(&json!({"error": "boom"})))
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
            "./out.json",
        ])
        .assert();

    let output = assert.failure().get_output().clone();
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!combined.trim().is_empty(), "an error must be reported");

    assert!(
        !cwd.path().join("out.json").exists(),
        "the reserved shell must be rolled back when the first page fails"
    );
    method.assert();
}
