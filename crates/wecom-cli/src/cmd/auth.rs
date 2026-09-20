use std::io::IsTerminal;
use std::path::{Path, PathBuf};

use clap::{ArgMatches, Args, CommandFactory, FromArgMatches, Parser, Subcommand};
use qrcode::QrCode;

use crate::auth;
use crate::browser;
use crate::{Error, Result};

/// 企业微信机器人授权管理
#[derive(Debug, Parser)]
#[command(
    name = "auth",
    subcommand_required = true,
    arg_required_else_help = true
)]
struct Auth {
    #[command(subcommand)]
    sub: AuthSubcmds,
}

#[derive(Debug, Subcommand)]
enum AuthSubcmds {
    /// 初始化企业微信机器人配置
    Init(InitArgs),
    /// 显示当前授权状态
    Show(ShowArgs),
}

#[derive(Debug, Args)]
struct InitArgs {
    /// 跳过交互选择，直接手动输入 Bot ID 和 Secret
    #[arg(long, conflicts_with = "noninteractive")]
    manual: bool,
    /// 扫码模式下不自动打开浏览器
    #[arg(long)]
    no_browser: bool,
    /// 跳过交互选择，直接使用扫码方式接入（非交互环境适用）
    #[arg(long)]
    noninteractive: bool,
    /// 直接指定 Bot ID
    #[arg(long, hide = true, requires = "secret")]
    bot_id: Option<String>,
    /// 直接指定 Secret
    #[arg(long, hide = true, requires = "bot_id")]
    secret: Option<String>,
    /// 将扫码二维码输出为 PNG 文件（相对路径基于当前目录解析）
    #[arg(long, value_name = "PATH", conflicts_with_all = ["manual", "bot_id"])]
    output_qrcode: Option<PathBuf>,
}

#[derive(Debug, Args)]
struct ShowArgs {
    /// 显示企业微信机器人授权状态
    #[arg(long)]
    status: bool,
}

/// 将 `auth` 命令注册为 wecom 扩展命令（由 `Client::run` 统一调度）。
///
/// 配置文件不在此透传：`main.rs` 已通过 `Transport::with_extension` 将
/// [`ConfigFile`](crate::config::ConfigFile) 注入 transport 默认扩展袋，
/// 子命令经 [`wecom::CliRun`] 读取，无需逐层传参。
pub fn custom_command() -> wecom::CustomCommand {
    wecom::CustomCommand::new(Auth::command(), |run, matches| {
        Box::pin(async move {
            handle_auth_cmd(run, matches)
                .await
                .map_err(wecom::Error::from)
        })
    })
}

async fn handle_auth_cmd(run: &wecom::CliRun<'_>, matches: &ArgMatches) -> Result<()> {
    // 无子命令时 clap 已在解析阶段展示 help（arg_required_else_help），
    // 能走到这里 matches 必然含子命令。
    match Auth::from_arg_matches(matches)?.sub {
        AuthSubcmds::Init(args) => handle_init(run, args).await,
        AuthSubcmds::Show(args) => handle_show(run, args),
    }
}

fn managed_auth_enabled() -> bool {
    cfg!(feature = "managed-auth")
}

fn managed_access_token_present() -> bool {
    #[cfg(feature = "managed-auth")]
    {
        std::env::var(crate::env::ACCESS_TOKEN)
            .ok()
            .is_some_and(|token| !token.is_empty())
    }
    #[cfg(not(feature = "managed-auth"))]
    false
}

/// 输出当前授权状态（纯文本，经 [`wecom::CliRunOutput`] 写出以支持 writer 注入）。
fn handle_show(run: &wecom::CliRun<'_>, args: ShowArgs) -> Result<()> {
    let output = run.get_output();

    if managed_auth_enabled() {
        let authorized = managed_access_token_present();
        if args.status {
            output.print(if authorized {
                "authorized"
            } else {
                "unauthorized"
            });
        } else {
            output.print(if authorized {
                "Status: authorized (managed runtime)"
            } else {
                "Status: unauthorized"
            });
        }
        return Ok(());
    }

    let bot = auth::get_bot_info();

    if args.status {
        output.print(if bot.is_some() {
            "authorized"
        } else {
            "unauthorized"
        });
        return Ok(());
    }

    match bot {
        Some(bot) => {
            output.print("Status: authorized");
            output.print(&format!("Bot ID: {}", bot.id));
        }
        None => output.print("Status: unauthorized"),
    }

    Ok(())
}

async fn handle_init(run: &wecom::CliRun<'_>, args: InitArgs) -> Result<()> {
    if managed_auth_enabled() {
        return Err(Error::Auth(
            "当前 CLI 由托管运行时提供授权，请在宿主产品的连接器页面完成企业微信授权".to_string(),
        ));
    }

    if let (Some(botid), Some(secret)) = (args.bot_id, args.secret)
        && !std::io::stderr().is_terminal()
    {
        return init_with_bot(
            run,
            auth::Bot::new(botid, secret),
            auth::BindSource::Interactive,
        )
        .await;
    }

    // 接入方式解析：--noninteractive 直连扫码、--manual 直连手动输入（需 TTY）、
    // 默认 TTY 下交互选择、非 TTY 自动降级扫码。纯函数化便于单测（见模块末尾用例）。
    let bind_mode = resolve_bind_mode(
        args.noninteractive,
        args.manual,
        std::io::stderr().is_terminal(),
    )?;

    cliclack::intro("企业微信机器人配置")?;

    // 收敛接入路径：参数（--noninteractive / --manual）直连，默认 TTY 下经交互选择。
    // "qrcode"/"manual" 与下方 cliclack::select 的 item value 保持一致。
    let method: &str = match bind_mode {
        BindMode::Qrcode => "qrcode",
        BindMode::Manual => "manual",
        BindMode::Select => cliclack::select("请选择企微机器人接入方式：")
            .item("qrcode", "扫码接入（推荐）", "")
            .item("manual", "手动输入 Bot ID 和 Secret", "")
            .interact()?,
    };

    let (bot, bind_source) = match method {
        "qrcode" => (
            scan_qrcode_for_bot(
                run.get_fs().as_ref(),
                run.get_cwd(),
                args.no_browser,
                args.output_qrcode,
            )
            .await?,
            auth::BindSource::Qrcode,
        ),
        // select 的 item value 仅有 "qrcode" / "manual" 两个取值
        _ => {
            // 手动输入模式不产出二维码：显式提示，避免参数被静默忽略。
            if args.output_qrcode.is_some() {
                tracing::warn!("手动输入模式不输出二维码，--output-qrcode 已忽略");
            }
            (prompt_bot_credentials()?, auth::BindSource::Interactive)
        }
    };

    init_with_bot(run, bot, bind_source).await
}

/// 用给定 Bot 完成凭证验证与原子写入（交互路径与直接参数路径共用）。
async fn init_with_bot(
    run: &wecom::CliRun<'_>,
    bot: auth::Bot,
    bind_source: auth::BindSource,
) -> Result<()> {
    let spinner = cliclack::spinner();
    spinner.start("正在验证企业微信机器人凭证…");

    let cfg = run
        .get_client()
        .transport()
        .extensions()
        .get::<crate::config::ConfigFile>();

    let auth_endpoint = auth::resolve_auth_endpoint(cfg);

    let resp = match auth::fetch_auth(
        run.get_client().transport(),
        &bot,
        bind_source,
        &auth_endpoint,
    )
    .await
    {
        Ok(resp) => resp,
        Err(e) => {
            // 错误文案由 fetch_auth 统一生成（业务 errmsg/专项提示/HTTP 状态等），
            // 失败提示经 spinner 展示，错误原样向上传播（保持类型与错误码）。
            spinner.stop("企业微信机器人凭证验证失败");
            return Err(e);
        }
    };

    // 服务端未返回 token 视为失败：不写入任何凭据，旧凭据保留
    let Some(token) = resp.token.as_deref().filter(|t| !t.is_empty()) else {
        spinner.stop("鉴权未返回访问令牌");
        return Err(Error::protocol(
            "鉴权成功但未获取到访问令牌，请重试或检查账号状态",
            wecom_transport::EndpointHttpExt::full_url(&auth_endpoint),
            serde_json::to_value(resp).unwrap_or_default(),
        ));
    };

    // 统一原子写入：bot 凭据 + 引导换取 token
    let mut creds = auth::load_credentials().unwrap_or_default();
    creds.bot = Some(bot);
    creds.token = Some(token.to_string());
    auth::save_credentials(&creds).await?;

    spinner.stop("企业微信机器人凭证验证成功");
    cliclack::outro("初始化完成 ✅")?;
    tracing::info!("auth init completed");

    Ok(())
}

// ---------------------------------------------------------------------------
// 接入方式解析（纯函数，便于单测）
// ---------------------------------------------------------------------------

/// 接入方式：直接扫码 / 直接手动输入 / 需交互选择。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BindMode {
    Qrcode,
    Manual,
    Select,
}

/// 按参数 + TTY 状态解析接入方式。
///
/// - `--noninteractive` 始终直连扫码（不依赖 TTY）；
/// - `--manual` 的手动输入需要终端，非 TTY 下给出定向提示；
/// - 默认（两者皆否）：TTY 下进入交互选择，非 TTY 下自动降级为扫码
///   （二维码可渲染到 stdout，用户仍可扫码完成绑定）。
fn resolve_bind_mode(noninteractive: bool, manual: bool, is_tty: bool) -> Result<BindMode> {
    if noninteractive {
        return Ok(BindMode::Qrcode);
    }
    if manual {
        if is_tty {
            return Ok(BindMode::Manual);
        }
        return Err(wecom::Error::validation(
            "手动输入需要终端，非交互环境请使用 --noninteractive 直接扫码接入",
        )
        .into());
    }
    if is_tty {
        return Ok(BindMode::Select);
    }
    // 非 TTY：无法做交互选择，自动降级为扫码（render_qrcode_unicode 已支持非 TTY 渲染）。
    Ok(BindMode::Qrcode)
}

/// 手动输入 Bot ID 和 Secret。
fn prompt_bot_credentials() -> Result<auth::Bot> {
    let botid: String = cliclack::input("Bot ID")
        .placeholder("请输入企业微信机器人 Bot ID")
        .validate(|input: &String| {
            if input.trim().is_empty() {
                Err("Bot ID 不能为空")
            } else {
                Ok(())
            }
        })
        .interact()?;

    let secret = cliclack::password("Secret")
        .mask('▪')
        .validate(|input: &String| {
            if input.trim().is_empty() {
                Err("Secret 不能为空")
            } else {
                Ok(())
            }
        })
        .interact()?;

    Ok(auth::Bot::new(botid, secret))
}

// ---------------------------------------------------------------------------
// QR 扫码登录（表现层；网络流程在 crate::auth::QrSession）
// ---------------------------------------------------------------------------

/// 扫码接入完整流程：创建会话 → 终端渲染二维码 → 可选输出 PNG → 浏览器打开 → 轮询结果。
///
/// 输出路径是外部输入：与 `--output` 同模式「先创建后请求」——会话创建前经
/// 注入的 workspace fs（生产为 `CliRun` 的实例）排他预留文件，创建即完成沙箱
/// 授权（roots + deny），后续经句柄写入，无 TOCTOU 窗口。
async fn scan_qrcode_for_bot(
    fs: &dyn wecom::Fs,
    base: &Path,
    no_browser: bool,
    output_qrcode: Option<PathBuf>,
) -> Result<auth::Bot> {
    // 先创建后请求：预留输出文件，确定性错误（deny / 越界 / 文件已存在）在扫码前暴露。
    let output_file = match output_qrcode {
        Some(path) => Some(
            fs.create_file(&wecom::absolutize(base, &path))
                .await
                .map_err(wecom::Error::from)?,
        ),
        None => None,
    };

    println!("正在获取二维码...");
    let session = auth::QrSession::create().await?;

    println!("请打开二维码链接扫码: \n{}", session.page_url);

    println!("也可以使用企业微信扫描以下二维码：");
    if std::io::stdout().is_terminal() {
        render_qrcode(&session.auth_url)?;
    } else {
        render_qrcode_unicode(&session.auth_url)?;
    }

    // 输出失败不中断扫码流程，warn 后继续轮询。
    if let Some((resolved, mut file)) = output_file {
        match render_qrcode_png(&session.auth_url, &mut file).await {
            Ok(()) => println!("二维码已保存到: {}", resolved.display()),
            Err(e) => tracing::warn!(error = %e, path = %resolved.display(), "二维码 PNG 输出失败"),
        }
    }

    if !no_browser {
        browser::open_url_by_browser(&session.page_url);
    }

    println!("等待扫码中...");

    let bot = session.poll().await?;

    println!("✔ 扫码成功！Bot ID 和 Secret 已自动获取。");
    Ok(bot)
}

/// 在终端渲染二维码（TTY，带 ANSI 色彩）
fn render_qrcode(url: &str) -> Result<()> {
    println!();
    // 渲染失败为非预期的系统级失败（终端能力不足等），归入兜底 Other。
    qr2term::print_qr(url).map_err(|e| Error::Other(format!("二维码渲染失败: {e}").into()))?;
    Ok(())
}

/// 在 non-TTY 环境下用纯 Unicode 半块字符渲染二维码（无 ANSI escape）
fn render_qrcode_unicode(url: &str) -> Result<()> {
    use qrcode::QrCode;
    use qrcode::render::unicode::Dense1x2;

    let code = QrCode::new(url).map_err(|e| Error::Other(format!("二维码渲染失败: {e}").into()))?;
    let string = code
        .render::<Dense1x2>()
        .dark_color(Dense1x2::Dark)
        .light_color(Dense1x2::Light)
        .build();
    println!();
    println!("{string}");
    Ok(())
}

/// 渲染二维码为黑白 PNG，写入预留文件的写端。
///
/// 文件在会话创建前已经由 workspace fs 排他创建（创建即完成沙箱授权），
/// 此处只负责编码与写数据，句柄级写入天然无 TOCTOU 窗口。
async fn render_qrcode_png(url: &str, file: &mut wecom::FileWriter) -> Result<()> {
    use tokio::io::AsyncWriteExt;

    let code = QrCode::new(url).map_err(|e| Error::Other(format!("二维码渲染失败: {e}").into()))?;
    let image: image::ImageBuffer<image::Luma<u8>, Vec<u8>> = code
        .render::<image::Luma<u8>>()
        .quiet_zone(true) // 安静区（默认 4 模块，显式声明）
        .module_dimensions(8, 8) // 8px/模块，便于移动端扫码
        .build();

    let mut buf = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageLuma8(image)
        .write_to(&mut buf, image::ImageFormat::Png)
        .map_err(|e| Error::Other(format!("二维码 PNG 编码失败: {e}").into()))?;

    file.write_all(buf.get_ref())
        .await
        .map_err(|e| Error::Other(format!("二维码 PNG 写入失败: {e}").into()))?;

    file.flush()
        .await
        .map_err(|e| Error::Other(format!("二维码 PNG 写入失败: {e}").into()))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    //! ## 模块摘要：auth init 接入方式解析（resolve_bind_mode）
    //!
    //! ### 关键接口
    //! - [BindMode] — 接入方式枚举（Qrcode / Manual / Select）
    //! - [resolve_bind_mode] — 按 `--noninteractive` / `--manual` 参数与 TTY 状态解析接入方式
    //!
    //! ### 关键分支与异常路径
    //! - `--noninteractive` 始终直连扫码，不受 TTY 影响
    //! - `--manual` 需要终端；非 TTY 时报错并提示 --noninteractive
    //! - 默认（两者皆否）：TTY 下进入交互选择；非 TTY 下自动降级为扫码
    //! - `--noninteractive` 与 `--manual` 互斥由 clap `conflicts_with` 在解析期拦截，纯函数内不处理该组合

    use super::*;

    /// P0：`--noninteractive` 在 TTY / 非 TTY 下均直连扫码
    /// 条件：noninteractive=true，manual=false，is_tty 分别取 true/false
    /// 断言：均返回 Ok(BindMode::Qrcode)
    #[test]
    fn noninteractive_always_skips_tty_check() {
        assert_eq!(
            resolve_bind_mode(true, false, false).unwrap(),
            BindMode::Qrcode
        );
        assert_eq!(
            resolve_bind_mode(true, false, true).unwrap(),
            BindMode::Qrcode
        );
    }

    /// P0：`--manual` 在 TTY 下直连手动输入
    /// 条件：noninteractive=false，manual=true，is_tty=true
    /// 断言：返回 Ok(BindMode::Manual)
    #[test]
    fn manual_requires_tty() {
        assert_eq!(
            resolve_bind_mode(false, true, true).unwrap(),
            BindMode::Manual
        );
    }

    /// P1：`--manual` 在非 TTY 下报错并提示 --noninteractive
    /// 条件：noninteractive=false，manual=true，is_tty=false
    /// 断言：返回 Err，错误文案含 "--noninteractive" 且区分于默认文案
    #[test]
    fn manual_non_tty_bails_with_hint() {
        let err = resolve_bind_mode(false, true, false).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("--noninteractive"), "got: {msg}");
        assert!(
            msg.contains("手动输入需要终端"),
            "expected manual-specific hint, got: {msg}"
        );
    }

    /// P0：默认（无直连参数）在非 TTY 下自动降级为扫码
    /// 条件：noninteractive=false，manual=false，is_tty=false
    /// 断言：返回 Ok(BindMode::Qrcode)（二维码可渲染到 stdout，用户仍可扫码）
    #[test]
    fn default_non_tty_falls_back_to_qrcode() {
        assert_eq!(
            resolve_bind_mode(false, false, false).unwrap(),
            BindMode::Qrcode
        );
    }

    /// P0：默认在 TTY 下进入交互选择
    /// 条件：noninteractive=false，manual=false，is_tty=true
    /// 断言：返回 Ok(BindMode::Select)
    #[test]
    fn default_tty_selects_interactively() {
        assert_eq!(
            resolve_bind_mode(false, false, true).unwrap(),
            BindMode::Select
        );
    }

    /// P0：--bot-id 与 --secret 必须同时给出（clap `requires`）
    /// 条件：仅传 --bot-id，不传 --secret
    /// 断言：clap 解析失败，错误类型为 MissingRequiredArgument
    #[test]
    fn direct_bot_args_require_both() {
        let err = Auth::command()
            .try_get_matches_from(["auth", "init", "--bot-id", "x"])
            .unwrap_err();
        assert_eq!(err.kind(), clap::error::ErrorKind::MissingRequiredArgument);
    }

    /// P1：--bot-id / --secret 为隐藏参数，不出现在 help 输出中
    /// 条件：渲染 `auth init` 的 help
    /// 断言：help 文本不含 "--bot-id" / "--secret"
    #[test]
    fn direct_bot_args_hidden_from_help() {
        let help = Auth::command()
            .find_subcommand_mut("init")
            .unwrap()
            .render_help()
            .to_string();
        assert!(!help.contains("--bot-id"), "bot-id leaked into help");
        assert!(!help.contains("--secret"), "secret leaked into help");
    }

    // ── --output-qrcode ──

    /// P1：--output-qrcode 出现在 help 输出中
    /// 条件：渲染 `auth init` 的 help
    /// 断言：help 文本包含 "--output-qrcode"
    #[test]
    fn output_qrcode_shown_in_help() {
        let help = Auth::command()
            .find_subcommand_mut("init")
            .unwrap()
            .render_help()
            .to_string();
        assert!(help.contains("--output-qrcode"), "missing in help");
    }

    /// P1：--output-qrcode 与 --manual 冲突（clap 解析期拦截）
    /// 条件：同时传 --output-qrcode 与 --manual
    /// 断言：解析失败，错误类型为 ArgumentConflict
    #[test]
    fn output_qrcode_conflicts_with_manual() {
        let err = Auth::command()
            .try_get_matches_from(["auth", "init", "--output-qrcode", "q.png", "--manual"])
            .unwrap_err();
        assert_eq!(err.kind(), clap::error::ErrorKind::ArgumentConflict);
    }

    /// P1：--output-qrcode 与 --bot-id 冲突（clap 解析期拦截，避免早返回分支静默丢弃）
    /// 条件：同时传 --output-qrcode 与 --bot-id --secret
    /// 断言：解析失败，错误类型为 ArgumentConflict
    #[test]
    fn output_qrcode_conflicts_with_bot_id() {
        let err = Auth::command()
            .try_get_matches_from([
                "auth",
                "init",
                "--output-qrcode",
                "q.png",
                "--bot-id",
                "x",
                "--secret",
                "y",
            ])
            .unwrap_err();
        assert_eq!(err.kind(), clap::error::ErrorKind::ArgumentConflict);
    }

    // ── --output-qrcode 预留 + 写入 ──

    /// 测试夹具：与生产一致的 workspace 域 fs（roots=[base, 系统临时目录] + deny 规则）。
    fn workspace_fs(base: &Path) -> std::sync::Arc<dyn wecom::Fs> {
        wecom::default_workspace_fs(base)
    }

    /// P0：预留文件命中 deny → Err（创建即授权，敏感形状在请求前被拦截）
    /// 条件：base 下 .config/wecom 形状路径
    /// 断言：create_file 返回 Err
    #[tokio::test]
    async fn reserve_output_file_rejects_deny_rule_hit() {
        let dir = tempfile::tempdir().unwrap();
        let base = dir.path();
        let result = workspace_fs(base)
            .create_file(&base.join(".config/wecom/qr.png"))
            .await;
        assert!(result.is_err(), "deny shape must be rejected");
    }

    /// P0：预留文件已存在 → Err（排他创建，与 --output 一致不覆盖）
    /// 条件：base 下 qr.png 已存在
    /// 断言：create_file 返回 Err
    #[tokio::test]
    async fn reserve_output_file_rejects_existing_file() {
        let dir = tempfile::tempdir().unwrap();
        let base = dir.path();
        #[allow(clippy::disallowed_methods)] // 测试写入临时目录。
        std::fs::write(base.join("qr.png"), b"old").unwrap();
        let result = workspace_fs(base).create_file(&base.join("qr.png")).await;
        assert!(result.is_err(), "existing file must be rejected");
    }

    /// P1：render_qrcode_png 写入预留文件生成有效 PNG
    /// 条件：经 workspace fs 预留 base 下 qr.png 后写入
    /// 断言：文件以 PNG 魔数 \x89PNG 开头
    #[tokio::test]
    async fn render_qrcode_png_writes_valid_file() {
        let dir = tempfile::tempdir().unwrap();
        let base = dir.path();
        let (resolved, mut file) = workspace_fs(base)
            .create_file(&base.join("qr.png"))
            .await
            .unwrap();
        render_qrcode_png("https://example.com/auth", &mut file)
            .await
            .unwrap();
        drop(file);
        #[allow(clippy::disallowed_methods)] // 测试读取临时目录文件。
        let data = std::fs::read(&resolved).unwrap();
        assert_eq!(&data[..4], b"\x89PNG", "not a valid PNG file");
    }

    /// P1：render_qrcode_png 输出可被 image 解码回读，尺寸为 (模块数+8)×8
    /// 条件：固定 URL 生成 PNG 并解码
    /// 断言：解码成功，宽高相等（正方形）且为 8 的倍数（含 4 模块安静区）
    #[tokio::test]
    async fn render_qrcode_png_is_decodable_square() {
        let dir = tempfile::tempdir().unwrap();
        let base = dir.path();
        let (resolved, mut file) = workspace_fs(base)
            .create_file(&base.join("qr.png"))
            .await
            .unwrap();
        render_qrcode_png("https://example.com/auth", &mut file)
            .await
            .unwrap();
        drop(file);
        let img = image::open(&resolved).unwrap();
        let (w, h) = (img.width(), img.height());
        assert_eq!(w, h, "QR should be square");
        assert!(w % 8 == 0, "width {} should be multiple of 8", w);
    }
}
