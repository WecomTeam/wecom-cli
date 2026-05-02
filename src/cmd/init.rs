use std::io::IsTerminal;

use crate::auth;
use crate::mcp;
use crate::mcp::config::McpBindSource;
use crate::constants;
use anyhow::Result;
use clap::{ArgMatches, Args, Command, FromArgMatches};

#[derive(Args)]
pub struct InitArgs {
    /// 跳过交互选择，直接使用扫码方式接入
    #[arg(long)]
    pub noninteractive: bool,
}

pub fn build_init_cmd() -> Command {
    InitArgs::augment_args(Command::new("init").about("初始化企业微信机器人配置"))
        .disable_help_flag(true)
}

/// Handle the `init` subcommand: prompt for bot credentials, persist them, and verify via MCP config fetch.
pub async fn handle_init_cmd(matches: &ArgMatches) -> Result<()> {
    let args = InitArgs::from_arg_matches(matches)?;

    if !args.noninteractive && !std::io::stderr().is_terminal() {
        anyhow::bail!(
            "当前环境不支持交互式操作，请使用 --noninteractive 参数：\n  {} init --noninteractive",
            env!("CARGO_BIN_NAME")
        );
    }

    cliclack::intro("企业微信机器人初始化")?;

    let (bot, bind_source) = if args.noninteractive {
        init_noninteractive().await?
    } else {
        let env_available = std::env::var(constants::env::BOT_ID).is_ok()
            && std::env::var(constants::env::BOT_SECRET).is_ok();

        let method: &str = if env_available {
            cliclack::select("请选择企微机器人接入方式：")
                .item("qrcode", "扫码接入（推荐）", "")
                .item("manual", "手动输入 Bot ID 和 Secret", "")
                .item("env", "从环境变量读取（WECOM_BOT_ID / WECOM_SECRET）", "")
                .interact()?
        } else {
            cliclack::select("请选择企微机器人接入方式：")
                .item("qrcode", "扫码接入（推荐）", "")
                .item("manual", "手动输入 Bot ID 和 Secret", "")
                .interact()?
        };

        match method {
            "qrcode" => (init_qrcode().await?, McpBindSource::Qrcode),
            "env" => (init_env()?, McpBindSource::Env),
            _ => (init_manual().await?, McpBindSource::Interactive),
        }
    };

    auth::set_bot_info(&bot)?;
    verify_and_finish(bind_source).await
}

/// 扫码接入流程
async fn init_qrcode() -> Result<auth::Bot> {
    auth::scan_qrcode_for_bot().await
}

/// 环境变量接入流程
fn init_env() -> Result<auth::Bot> {
    let bot_id = std::env::var(constants::env::BOT_ID)
        .map_err(|_| anyhow::anyhow!("环境变量 {} 未设置", constants::env::BOT_ID))?;
    let bot_secret = std::env::var(constants::env::BOT_SECRET)
        .map_err(|_| anyhow::anyhow!("环境变量 {} 未设置", constants::env::BOT_SECRET))?;
    Ok(auth::Bot::new(bot_id, bot_secret))
}

/// 非交互式接入流程：优先从环境变量读取，否则走扫码
async fn init_noninteractive() -> Result<(auth::Bot, McpBindSource)> {
    if let (Ok(bot_id), Ok(bot_secret)) = (
        std::env::var(constants::env::BOT_ID),
        std::env::var(constants::env::BOT_SECRET),
    ) {
        cliclack::log::info("使用环境变量 WECOM_BOT_ID / WECOM_SECRET 初始化")?;
        return Ok((auth::Bot::new(bot_id, bot_secret), McpBindSource::Env));
    }
    Ok((init_qrcode().await?, McpBindSource::Qrcode))
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
