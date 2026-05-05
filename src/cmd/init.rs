use std::io::IsTerminal;

use crate::auth;
use crate::mcp;
use crate::mcp::config::McpBindSource;
use anyhow::{Context, Result};
use clap::{ArgMatches, Args, Command, FromArgMatches};

/// Environment variables consumed by `--bootstrap`.
///
/// Both must be set; otherwise the command fails fast without touching
/// existing credentials on disk.
pub const ENV_BOOTSTRAP_BOT_ID: &str = "WECOM_CLI_BOOTSTRAP_BOT_ID";
pub const ENV_BOOTSTRAP_BOT_SECRET: &str = "WECOM_CLI_BOOTSTRAP_BOT_SECRET";

#[derive(Args)]
pub struct InitArgs {
    /// 跳过交互选择，直接使用扫码方式接入
    #[arg(long, conflicts_with = "bootstrap")]
    pub noninteractive: bool,

    /// 从环境变量读取 Bot 凭据完成初始化（用于 AI Agent / CI / 沙箱预配置）。
    ///
    /// 需要同时设置 `WECOM_CLI_BOOTSTRAP_BOT_ID` 和 `WECOM_CLI_BOOTSTRAP_BOT_SECRET`。
    /// 凭据校验失败时自动回滚（与交互式模式一致）。
    #[arg(long)]
    pub bootstrap: bool,
}

pub fn build_init_cmd() -> Command {
    InitArgs::augment_args(Command::new("init").about("初始化企业微信机器人配置"))
        .disable_help_flag(true)
}

/// Handle the `init` subcommand: prompt for bot credentials, persist them, and verify via MCP config fetch.
pub async fn handle_init_cmd(matches: &ArgMatches) -> Result<()> {
    let args = InitArgs::from_arg_matches(matches)?;

    // bootstrap 路径：完全跳过交互，从环境变量读取凭据。
    // 必须放在 TTY 检查之前——bootstrap 场景本来就没有终端。
    if args.bootstrap {
        return run_bootstrap().await;
    }

    if !args.noninteractive && !std::io::stderr().is_terminal() {
        anyhow::bail!(
            "当前环境不支持交互式操作，请使用 --noninteractive 或 --bootstrap：\n  \
             {bin} init --noninteractive    # 扫码接入\n  \
             {bin} init --bootstrap         # 通过环境变量预配置",
            bin = env!("CARGO_BIN_NAME"),
        );
    }

    cliclack::intro("企业微信机器人初始化")?;

    let (bot, bind_source) = if args.noninteractive {
        (init_qrcode().await?, McpBindSource::Qrcode)
    } else {
        let method: &str = cliclack::select("请选择企微机器人接入方式：")
            .item("qrcode", "扫码接入（推荐）", "")
            .item("manual", "手动输入 Bot ID 和 Secret", "")
            .interact()?;

        match method {
            "qrcode" => (init_qrcode().await?, McpBindSource::Qrcode),
            _ => (init_manual().await?, McpBindSource::Interactive),
        }
    };

    auth::set_bot_info(&bot)?;
    verify_and_finish(bind_source).await
}

/// `--bootstrap` 模式：从环境变量读取 Bot 凭据并完成初始化。
///
/// 走与交互模式完全相同的 [`auth::set_bot_info`] + [`verify_and_finish`]
/// 路径——加密格式、回滚逻辑、服务端验证全部复用，凭据无效或网络故障时
/// 同样会清掉本地 `bot.enc` 与 `mcp_config`，不会留下半成品状态。
///
/// 典型用法：
///
/// ```bash
/// export WECOM_CLI_BOOTSTRAP_BOT_ID="aibXXXXXX"
/// export WECOM_CLI_BOOTSTRAP_BOT_SECRET="..."
/// wecom-cli init --bootstrap
/// ```
async fn run_bootstrap() -> Result<()> {
    let bot_id = std::env::var(ENV_BOOTSTRAP_BOT_ID)
        .with_context(|| format!("缺少环境变量 {ENV_BOOTSTRAP_BOT_ID}（--bootstrap 模式必填）"))?;
    let bot_secret = std::env::var(ENV_BOOTSTRAP_BOT_SECRET).with_context(|| {
        format!("缺少环境变量 {ENV_BOOTSTRAP_BOT_SECRET}（--bootstrap 模式必填）")
    })?;

    if bot_id.trim().is_empty() {
        anyhow::bail!("{ENV_BOOTSTRAP_BOT_ID} 不能为空");
    }
    if bot_secret.trim().is_empty() {
        anyhow::bail!("{ENV_BOOTSTRAP_BOT_SECRET} 不能为空");
    }

    let bot = auth::Bot::new(bot_id, bot_secret);
    auth::set_bot_info(&bot)?;
    // bootstrap 在语义上与"手动输入"等价（用户在外部已确定凭据），
    // 沿用 Interactive 作为 bind_source，避免给服务端引入新枚举值。
    verify_and_finish(McpBindSource::Interactive).await
}

/// 扫码接入流程
async fn init_qrcode() -> Result<auth::Bot> {
    auth::scan_qrcode_for_bot().await
}

/// 手动输入 Bot ID 和 Secret
async fn init_manual() -> Result<auth::Bot> {
    let bot_id: String = cliclack::input("企业微信机器人 Bot ID")
        .placeholder("请输入企业微信机器人ID")
        .interact()?;

    let bot_secret: String = cliclack::password("企业微信机器人 Secret")
        .mask('*')
        .interact()?;

    Ok(auth::Bot::new(bot_id, bot_secret))
}

/// 验证凭证并完成初始化
async fn verify_and_finish(bind_source: McpBindSource) -> Result<()> {
    let spinner = cliclack::spinner();
    spinner.start("正在验证企业微信机器人凭证...");

    if let Err(e) = mcp::config::fetch_mcp_config(bind_source).await {
        spinner.stop("企业微信机器人凭证验证失败");

        let mut output_errmsg: String = "验证企业微信机器人凭证失败".to_owned();

        match &e {
            mcp::error::FetchMcpConfigError::Api(resp) => {
                if let Some(ref msg) = resp.errmsg {
                    if !msg.is_empty() {
                        output_errmsg = msg.clone();
                    }
                }
            }
            mcp::error::FetchMcpConfigError::Http(http_err) => {
                output_errmsg = format!("{} HTTP返回状态码 {}", output_errmsg, http_err.status);
            }
            mcp::error::FetchMcpConfigError::Other(other_err) => {
                output_errmsg = other_err.to_string();
            }
        }

        // Credentials invalid or server unreachable — rollback
        auth::clear_bot_info();
        mcp::config::clear_mcp_config();
        cliclack::outro("初始化失败 ❌")?;
        anyhow::bail!(output_errmsg);
    }

    spinner.stop("企业微信机器人凭证验证成功");
    cliclack::outro("初始化完成 ✅")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    //! `--bootstrap` 模式纯单元测试。
    //!
    //! 真正网络验证（[`verify_and_finish`]）和加密落盘（[`auth::set_bot_info`]）
    //! 各自有独立的测试，这里只覆盖 CLI 解析 + 环境变量读取边界，
    //! 用 [`serial_test`] 避免环境变量被并行测试相互覆盖——但项目当前
    //! 没引入这个 crate，所以测试用 [`std::sync::Mutex`] 自管串行。
    use super::*;
    use clap::Command;
    use std::sync::Mutex;

    /// 串行化所有读写 ``WECOM_CLI_BOOTSTRAP_*`` 环境变量的测试。
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    /// RAII 守卫：构造时设环境变量、析构时清理，避免测试间污染。
    struct EnvGuard {
        keys: Vec<&'static str>,
    }

    impl EnvGuard {
        fn set(pairs: &[(&'static str, &str)]) -> Self {
            let mut keys = Vec::with_capacity(pairs.len());
            for (k, v) in pairs {
                // SAFETY: 测试以 ``ENV_LOCK`` 串行执行，保证此处与 Drop
                //         不会与其他测试同时改 process-wide env 状态。
                unsafe { std::env::set_var(k, v) };
                keys.push(*k);
            }
            Self { keys }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            for k in &self.keys {
                // SAFETY: 同 ``set`` —— 串行下唯一改动者。
                unsafe { std::env::remove_var(k) };
            }
        }
    }

    fn parse(argv: &[&str]) -> Result<InitArgs, clap::Error> {
        let cmd = InitArgs::augment_args(Command::new("init"));
        let matches = cmd.try_get_matches_from(argv)?;
        Ok(InitArgs::from_arg_matches(&matches).unwrap())
    }

    // -------- CLI parsing --------

    #[test]
    fn parses_bootstrap_flag() {
        let a = parse(&["init", "--bootstrap"]).unwrap();
        assert!(a.bootstrap);
        assert!(!a.noninteractive);
    }

    #[test]
    fn parses_noninteractive_flag() {
        let a = parse(&["init", "--noninteractive"]).unwrap();
        assert!(a.noninteractive);
        assert!(!a.bootstrap);
    }

    #[test]
    fn bootstrap_and_noninteractive_are_mutually_exclusive() {
        // clap 在 conflicts_with 触发时返回 ArgumentConflict 错误。
        // 不给 InitArgs 派生 Debug 只为了 unwrap_err()，手动 match。
        let err = match parse(&["init", "--bootstrap", "--noninteractive"]) {
            Err(e) => e,
            Ok(_) => panic!("expected conflict, got Ok"),
        };
        assert_eq!(err.kind(), clap::error::ErrorKind::ArgumentConflict);
    }

    #[test]
    fn no_flags_defaults_to_interactive() {
        let a = parse(&["init"]).unwrap();
        assert!(!a.bootstrap);
        assert!(!a.noninteractive);
    }

    // -------- run_bootstrap env-var validation --------
    //
    // 注意：``run_bootstrap`` 末尾会调用 ``auth::set_bot_info`` 实际写盘
    // 并请求服务端验证；这些路径不在单元测试覆盖范围内（需要文件系统
    // 沙箱 + mock 服务器）。这里只验证"凭据缺失/为空时立刻 bail，不接触
    // 任何 I/O" 的快速失败语义。
    //
    // 用同步 [`#[test]`] + 内置 ``current_thread`` runtime block_on，
    // 避免 ``#[tokio::test]`` 的 ``ENV_LOCK`` 跨 await（clippy
    // ``await_holding_lock``）。env-var 缺失分支根本不 hit 任何 await
    // point，所以同步执行语义等价。

    fn block_run_bootstrap() -> Result<()> {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(run_bootstrap())
    }

    #[test]
    fn run_bootstrap_fails_when_bot_id_missing() {
        let _g = ENV_LOCK.lock().unwrap();
        let _env = EnvGuard::set(&[(ENV_BOOTSTRAP_BOT_SECRET, "secret-only")]);
        // SAFETY: serialized via ENV_LOCK
        unsafe { std::env::remove_var(ENV_BOOTSTRAP_BOT_ID) };

        let err = match block_run_bootstrap() {
            Err(e) => e,
            Ok(()) => panic!("expected error, got Ok"),
        };
        let msg = format!("{err:?}");
        assert!(
            msg.contains(ENV_BOOTSTRAP_BOT_ID),
            "应当提示缺少 BOT_ID 环境变量, got: {msg}",
        );
    }

    #[test]
    fn run_bootstrap_fails_when_secret_missing() {
        let _g = ENV_LOCK.lock().unwrap();
        let _env = EnvGuard::set(&[(ENV_BOOTSTRAP_BOT_ID, "aib_test")]);
        unsafe { std::env::remove_var(ENV_BOOTSTRAP_BOT_SECRET) };

        let err = match block_run_bootstrap() {
            Err(e) => e,
            Ok(()) => panic!("expected error, got Ok"),
        };
        let msg = format!("{err:?}");
        assert!(
            msg.contains(ENV_BOOTSTRAP_BOT_SECRET),
            "应当提示缺少 BOT_SECRET 环境变量, got: {msg}",
        );
    }

    #[test]
    fn run_bootstrap_rejects_empty_bot_id() {
        let _g = ENV_LOCK.lock().unwrap();
        let _env = EnvGuard::set(&[
            (ENV_BOOTSTRAP_BOT_ID, "   "),
            (ENV_BOOTSTRAP_BOT_SECRET, "valid"),
        ]);

        let err = match block_run_bootstrap() {
            Err(e) => e,
            Ok(()) => panic!("expected error, got Ok"),
        };
        let msg = format!("{err}");
        assert!(msg.contains("不能为空"), "got: {msg}");
        assert!(msg.contains(ENV_BOOTSTRAP_BOT_ID), "got: {msg}");
    }

    #[test]
    fn run_bootstrap_rejects_empty_secret() {
        let _g = ENV_LOCK.lock().unwrap();
        let _env = EnvGuard::set(&[
            (ENV_BOOTSTRAP_BOT_ID, "aib_test"),
            (ENV_BOOTSTRAP_BOT_SECRET, "\t\n  "),
        ]);

        let err = match block_run_bootstrap() {
            Err(e) => e,
            Ok(()) => panic!("expected error, got Ok"),
        };
        let msg = format!("{err}");
        assert!(msg.contains("不能为空"), "got: {msg}");
        assert!(msg.contains(ENV_BOOTSTRAP_BOT_SECRET), "got: {msg}");
    }
}
