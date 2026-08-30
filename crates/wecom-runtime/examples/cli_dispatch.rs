//! CLI 风格 argv 调度示例：与 `wecom-cli` 完全同一套命令模型。
//!
//! 适合把 CLI 命令字符串（如来自 Skill / Agent 的工具调用）直接投递给
//! Library 处理，输出经自定义 [`wecom::Writer`](wecom::Writer) 回收。
//!
//! 运行（需真实凭据）：
//!
//! ```bash
//! WECOM_BOT_ID=xxx WECOM_BOT_SECRET=yyy \
//!   cargo run -p wecom-runtime --example cli_dispatch -- hr users list
//! ```

use std::sync::Arc;

use wecom_runtime::{
    BotGatewayTokenProvider, Credentials, MemoryCredentialStore, WecomClientBuilder,
};

#[tokio::main]
async fn main() -> Result<(), wecom::Error> {
    let bot_id = std::env::var("WECOM_BOT_ID").expect("missing WECOM_BOT_ID");
    let secret = std::env::var("WECOM_BOT_SECRET").expect("missing WECOM_BOT_SECRET");
    let store = Arc::new(MemoryCredentialStore::new(Credentials::default()));
    let provider = BotGatewayTokenProvider::new(bot_id, secret, store);

    let client = WecomClientBuilder::new()
        .token_provider(Arc::new(provider))
        .build()
        .await?;

    // argv 与 CLI 完全一致：`wecom-cli <service> [resource...] <method> [flags]`
    let mut argv: Vec<String> = std::env::args().collect();
    argv[0] = "wecom".to_string(); // help/版本输出中的命令名

    // CliRun 为 IntoFuture，.await 即执行；stdout JSON 与 CLI 输出同构。
    client.run(argv).await?;

    Ok(())
}
