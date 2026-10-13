// Process-level: 分页 NDJSON 在首页写入后被保留——次页失败仍保留已写入的首页。
#[cfg(feature = "custom-endpoint")]
#[test]
#[allow(clippy::disallowed_methods)] // Test fixture: probe/fixture files directly, not through the sandbox
fn run() {
    let cwd = tempfile::tempdir().unwrap();
    let cfg_dir = tempfile::tempdir().unwrap();
    seed_credentials(cfg_dir.path());

    let rt = tokio::runtime::Runtime::new().unwrap();
    let (server_url, first_page, second_page, _keep) = rt.block_on(async {
        let mut server = Server::new_async().await;
        let (catalog, hr) = setup_discovery_mocks(&mut server).await;
        // 服务方法请求体是 `{"payload": "<json字符串>"}` 封装：首页 payload 无 cursor，
        // 次页带 cursor_1。首页 mock 不带 body matcher（唯一匹配），次页 mock 用正则
        // 只匹配含 cursor_1 的请求（首页 mock 命中一次后，mockito 把优先级让给尚未
        // 命中 expect 的次页 mock）。
        let first_page = server
            .mock("POST", "/department/list")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(api_response(&json!({
                "departments": [{"id": "1"}],
                "has_more": true,
                "next_cursor": "cursor_1"
            })))
            .expect(1)
            .create_async()
            .await;
        let second_page = server
            .mock("POST", "/department/list")
            .match_body(Matcher::Regex("cursor_1".into()))
            .with_status(500)
            .with_header("content-type", "application/json")
            .with_body(api_response(&json!({"error": "boom"})))
            .expect(1)
            .create_async()
            .await;
        (server.url(), first_page, second_page, (server, catalog, hr))
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
            "--page-count",
            "2",
            "--output",
            "./out.ndjson",
        ])
        .assert();

    let output = assert.failure().get_output().clone();
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!combined.trim().is_empty(), "an error must be reported");

    let content = std::fs::read_to_string(cwd.path().join("out.ndjson")).unwrap_or_else(|e| {
        panic!("the first page must be retained after a later-page failure: {e}; cli = {combined}")
    });
    let lines: Vec<&str> = content.trim().lines().collect();
    assert_eq!(lines.len(), 1, "only the first page is on disk: {content}");
    let first: Value = serde_json::from_str(lines[0]).unwrap();
    assert_eq!(first["departments"][0]["id"], "1");

    first_page.assert();
    second_page.assert();
}
