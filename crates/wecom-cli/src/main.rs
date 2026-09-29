mod auth;
mod cmd;
mod config;
mod env;
mod error;
mod logging;
mod telemetry;
#[cfg(feature = "call-chain")]
mod trace;
mod transport;

use error::Error;
use tracing::Instrument;

pub(crate) type Result<T> = std::result::Result<T, Error>;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    let root_span = logging::init_logging();

    // 挂载 json repair 提示监听：repair 成功时向 stderr 输出提示。
    let scope = wecom_transport::telemetry::CaptureScope::attach(&root_span);
    telemetry::install_json_repair_listener(&scope);

    let run = async {
        // 沙箱接线：双域实例。
        //
        // 两阶段构造（config_dir 不依赖 config 内容，无鸡生蛋问题）：
        // 1. 算 config_dir（env / 默认）→ 经 default_private_fs 构造 PrivateFs
        //    （roots = [config_dir]），用它读 config.json；
        // 2. 构造 WorkspaceFs（读写同 roots = [cwd, 固定临时目录]——上传类
        //    调用合法引用 mktemp/截图等外部产物，下载默认落盘 cwd；临时目录
        //    用 pinned_temp_dir() 的固定取值而非 env 驱动的 temp_dir()；
        //    配置目录叠加进其 deny 规则屏蔽，即使落在 cwd 内也不放行；
        //    内建 glob 表含系统目录与凭据形状，root 落入 deny 时该 root
        //    上的操作被逐次拒绝）。
        //
        // roots 与 deny 只能在构造期注入，无 env/flag/config 扩展口。
        let fs_cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
        let config_dir = config::default_home_dir();

        let private_fs = wecom::default_private_fs(&config_dir);

        let cfg = config::load_config_file(private_fs.as_ref(), &config::default_config_path())
            .await?
            .unwrap_or_default();

        let builder = wecom::Client::builder();
        let builder = config::apply_config(builder, &cfg)?;
        let builder = builder.endpoint_catalog(transport::endpoint_catalog());

        let mut allowed_roots = vec![fs_cwd.as_path()];
        let pinned_tmp = config::pinned_temp_dir();
        if let Some(tmp) = &pinned_tmp {
            allowed_roots.push(tmp.as_path());
        }
        let workspace_fs = wecom_fs::SandboxedFs::new().with_policy(
            wecom_fs::Policy::new()
                .with_allowed_dirs(&allowed_roots)
                .with_deny(wecom_fs::recommended_deny_rule())
                .with_deny(wecom_fs::DenyRule::prefix(&config_dir)),
        );

        let transport = transport::build(&cfg).await?.with_extension(cfg);

        #[cfg(feature = "call-chain")]
        let transport = {
            let mut transport = transport;
            if let Ok(value) =
                reqwest::header::HeaderValue::from_str(&trace::build_trace_header_value())
            {
                transport.headers_mut().insert(
                    reqwest::header::HeaderName::from_static(trace::TRACE_HEADER),
                    value,
                );
            }
            transport
        };

        let builder = builder
            .private_fs(private_fs)
            .workspace_fs(std::sync::Arc::new(workspace_fs))
            .transport(transport)
            .bin_name(env!("CARGO_BIN_NAME"))
            .cwd(&fs_cwd);

        // skip-auth-check 构建不注册 auth 命令：顶层 help 不出现；
        // 调用落入"命令未找到"路径（InvalidSubcommand，退出码 2）。
        #[cfg(not(feature = "skip-auth-check"))]
        let builder = builder.command(cmd::auth::custom_command());

        let client = builder.build()?;

        client.run(std::env::args().collect()).await
    };

    if let Err(err) = run.instrument(root_span).await {
        println!("{}", err.render());
        // 命令未找到时提示更新 SKILL（stderr，不污染 stdout）。
        if is_subcommand_not_found(&err) {
            eprintln!();
            eprintln!("{SKILL_HINT}");
        }
        std::process::exit(err.exit_code());
    }
}

/// 命令未找到时的提示文案。
const SKILL_HINT: &str = "该接口不存在，可能为接口命令错误，请更新到最新 skill 后重试";

/// 是否为"命令未找到"类错误。
fn is_subcommand_not_found(err: &wecom::Error) -> bool {
    match err {
        wecom::Error::CliOutput {
            source: Some(e), ..
        } => e.kind() == clap::error::ErrorKind::InvalidSubcommand,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cli_output(kind: clap::error::ErrorKind) -> wecom::Error {
        wecom::Error::CliOutput {
            code: 2,
            message: String::new(),
            source: Some(clap::Error::raw(kind, "boom")),
        }
    }

    /// unknown subcommand → 命中提示
    #[test]
    fn invalid_subcommand_hits() {
        assert!(is_subcommand_not_found(&cli_output(
            clap::error::ErrorKind::InvalidSubcommand
        )));
    }

    /// 其它 clap 错误（如参数缺失）→ 不命中
    #[test]
    fn other_clap_kind_misses() {
        assert!(!is_subcommand_not_found(&cli_output(
            clap::error::ErrorKind::MissingRequiredArgument
        )));
    }

    /// 非 clap 来源（如后端 10021 用法错误）→ 不命中
    #[test]
    fn cli_output_without_source_misses() {
        assert!(!is_subcommand_not_found(&wecom::Error::CliOutput {
            code: 2,
            message: String::new(),
            source: None,
        }));
    }

    /// 其它错误变体 → 不命中
    #[test]
    fn other_errors_miss() {
        assert!(!is_subcommand_not_found(&wecom::Error::validation("x")));
    }
}
