//! 第三方应用最小接入示例：鉴权客户端 + 程序化服务方法调用。
//!
//! 凭据来源：环境变量 `WECOM_BOT_ID` / `WECOM_BOT_SECRET`（botid + secret），
//! token 经 `MemoryCredentialStore` 缓存在进程内存（刷新自动写回）。
//!
//! 运行（需真实凭据）：
//!
//! ```bash
//! WECOM_BOT_ID=xxx WECOM_BOT_SECRET=yyy cargo run -p wecom-runtime --example service_invoke
//! ```

use std::sync::Arc;
use std::time::Duration;

use serde_json::json;
use wecom_runtime::{
    BotGatewayTokenProvider, Credentials, MemoryCredentialStore, TokenProvider, WecomClientBuilder,
};

#[tokio::main]
async fn main() -> Result<(), wecom::Error> {
    // 1. 凭据与 token provider
    //
    // 服务端/容器：凭据经环境变量（或 Secret Manager）注入，token 存内存。
    let bot_id = std::env::var("WECOM_BOT_ID").expect("missing WECOM_BOT_ID");
    let secret = std::env::var("WECOM_BOT_SECRET").expect("missing WECOM_BOT_SECRET");
    let store = Arc::new(MemoryCredentialStore::new(Credentials::default()));
    let provider = BotGatewayTokenProvider::new(bot_id, secret, store);

    // 本地开发可与 wecom-cli 共享加密凭据文件（credentials.enc）：
    //   let store = Arc::new(EncryptedFileCredentialStore::new("~/.config/wecom"));
    //   let provider = BotGatewayTokenProvider::from_store(store);

    // 2. 构建客户端（鉴权 transport + 网关协议 + 动态服务发现，一步到位）
    let client = WecomClientBuilder::new()
        .token_provider(Arc::new(provider) as Arc<dyn TokenProvider>)
        .timeout(Duration::from_secs(10))
        .build()
        .await?;

    // 3. 列出 discovery 下发的服务目录
    for svc in client.list_services().await? {
        println!("service: {}", svc.name);
    }

    // 4. 按路径调用方法（服务名/资源段/方法名以 discovery 下发为准）
    let method = client.method(&["hr", "users", "list"]).await?;
    let result = method.invoke(json!({})).await?;
    println!(
        "{}",
        serde_json::to_string_pretty(&result).unwrap_or_default()
    );

    Ok(())
}
