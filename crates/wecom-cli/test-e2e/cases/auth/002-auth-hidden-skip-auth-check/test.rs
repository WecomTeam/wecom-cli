// Process-level: skip-auth-check 构建屏蔽 auth 命令（注册点条件化）。
//
// 屏蔽后：顶层 help 不含 auth；任何 `auth ...` 调用落入"命令未找到"路径
// （clap InvalidSubcommand，退出码 2）。服务目录经 mock 固定
// （custom-endpoint），避免依赖真实网关与凭据。

use std::path::Path;

/// 启动 CLI 并返回输出（隔离配置目录 + mock discovery 端点 + 显式剔除环境变量 token）。
#[cfg(feature = "custom-endpoint")]
fn run_cli(dir: &Path, base_url: &str, args: &[&str]) -> std::process::Output {
    assert_cmd::Command::cargo_bin("wecom-cli")
        .unwrap()
        .env("WECOM_CLI_CONFIG_DIR", dir)
        .env("WECOM_CLI_BASE_URL", base_url)
        .env_remove("WECOM_CLI_ACCESS_TOKEN")
        .args(args)
        .output()
        .unwrap()
}

/// P0：`auth` 子命令调用随构建 feature 分叉：常规构建进入命令处理；
/// skip-auth-check 构建未注册该命令，落入"命令未找到"路径。
#[cfg(feature = "custom-endpoint")]
#[test]
fn auth_invocation_reflects_build_feature() {
    let tmp = tempfile::tempdir().unwrap();
    let (base_url, _server) = setup_sync_discovery_server();
    let output = run_cli(tmp.path(), &base_url, &["auth", "show", "--status"]);

    #[cfg(not(feature = "skip-auth-check"))]
    {
        assert_eq!(output.status.code(), Some(0));
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("unauthorized"), "got: {stdout}");
    }
    #[cfg(feature = "skip-auth-check")]
    {
        assert_eq!(output.status.code(), Some(2));
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("unrecognized subcommand 'auth'"),
            "got: {stdout}"
        );
    }
}

/// P0：顶层 help 的 auth 行随构建 feature 分叉（首 token 精确比较，防子串误判）。
#[cfg(feature = "custom-endpoint")]
#[test]
fn top_level_help_reflects_build_feature() {
    let tmp = tempfile::tempdir().unwrap();
    let (base_url, _server) = setup_sync_discovery_server();
    let output = run_cli(tmp.path(), &base_url, &["--help"]);
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    // 以行首 token 判定命令名：服务名可能以 auth 开头，前缀匹配会误判。
    let has_auth_cmd = stdout
        .lines()
        .any(|line| line.split_whitespace().next() == Some("auth"));

    #[cfg(not(feature = "skip-auth-check"))]
    assert!(has_auth_cmd, "expect auth in help");
    #[cfg(feature = "skip-auth-check")]
    assert!(!has_auth_cmd, "auth must not appear in help");
}
