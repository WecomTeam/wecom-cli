// Process-level: 二进制下载流中断后不残留半成品——`--output` 与默认落盘
// 两条路径分别由预留方 / 创建方回退。
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
            .with_status(200)
            .with_header("content-type", "application/octet-stream")
            .with_header("content-disposition", "attachment; filename=\"trunc.bin\"")
            .with_chunked_body(|w| {
                w.write_all(b"partial body")?;
                Err(std::io::Error::other("connection reset by peer"))
            })
            .expect(2)
            .create_async()
            .await;
        (server.url(), method, (server, catalog, hr))
    });

    let run_wecom = |args: &[&str]| {
        assert_cmd::Command::cargo_bin("wecom-cli")
            .unwrap()
            .current_dir(cwd.path())
            .env("WECOM_CLI_BASE_URL", &server_url)
            .env("WECOM_CLI_CONFIG_DIR", cfg_dir.path())
            .args(args)
            .assert()
    };

    // 1. 显式 --output：预留方回退
    let assert = run_wecom(&[
        "hr",
        "department",
        "list",
        "--id",
        "root",
        "--output",
        "./trunc.bin",
    ]);
    let output = assert.failure().get_output().clone();
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!combined.trim().is_empty(), "an error must be reported");
    assert!(
        !cwd.path().join("trunc.bin").exists(),
        "an interrupted --output download must leave nothing behind"
    );

    // 2. 无 --output（默认落盘 cwd）：创建方本地回退
    let assert = run_wecom(&["hr", "department", "list", "--id", "root"]);
    assert.failure();
    assert!(
        !cwd.path().join("trunc.bin").exists(),
        "an interrupted default download must leave nothing behind"
    );

    method.assert();
}
