mod auth;
mod browser;
mod cmd;
mod constants;
mod crypto;
mod fs_util;
mod helpers;
mod json_rpc;
mod logging;
mod mcp;
mod media;
mod paths;
mod registry;
mod service;

use anyhow::Result;
use clap::Command;

/// Entry point: parse CLI arguments and dispatch to the corresponding subcommand handler.
#[tokio::main]
async fn main() -> Result<()> {
    // OHOS: aws-lc-rs assembly does not link, use ring as TLS crypto backend.
    // Other platforms: use aws-lc-rs (reqwest's default).
    #[cfg(target_env = "ohos")]
    let _ = rustls::crypto::ring::default_provider().install_default();

    dotenvy::dotenv().ok();

    logging::init_logging();

    let helper_registry = helpers::HelperRegistry::new();

    let subcmd_name = std::env::args()
        .skip(1)
        .find(|a| a == "-V" || a == "--version" || !a.starts_with("-"))
        .unwrap_or_default();

    if subcmd_name == "-V" || subcmd_name == "--version" {
        println!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    let mut cmd = Command::new(env!("CARGO_BIN_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .arg_required_else_help(true)
        .disable_help_flag(true)
        .disable_version_flag(true)
        .disable_help_subcommand(true)
        .arg(
            clap::arg!(-h --help "查看帮助信息")
                .action(clap::ArgAction::Help)
                .global(true),
        )
        .arg(clap::arg!(-V --version "查看版本号").action(clap::ArgAction::Version))
        .subcommand(cmd::init::build_init_cmd())
        .subcommand(cmd::auth::build_auth_cmd());

    for category in service::categories::get_categories().iter() {
        let tools = if category.name == subcmd_name {
            let tools = registry::get_category_tools(category.name).await?;
            Some(tools)
        } else {
            None
        };
        cmd = cmd.subcommand(service::command::build_service_cmd(
            &helper_registry,
            category,
            tools.as_ref(),
        ));
    }

    cmd = cmd.subcommand(cmd::cache::build_cache_cmd());

    let matches = cmd.get_matches();

    match matches.subcommand() {
        Some(("init", matches)) => cmd::init::handle_init_cmd(matches).await,
        Some(("auth", matches)) => cmd::auth::handle_auth_cmd(matches).await,
        Some(("cache", matches)) => cmd::cache::handle_cache_cmd(matches).await,
        Some((category, matches)) => {
            service::handler::handle_service_cmd(&helper_registry, category, matches).await
        }
        _ => anyhow::bail!("未知命令"),
    }
}
