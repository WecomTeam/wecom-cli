//! [`WecomClientBuilder`]：一键构建带鉴权的 WeCom 客户端 / transport。
//!
//! 组合底层 HTTP 传输配置（base_url / 默认头 / 超时）与
//! [`TokenProvider`](wecom_auth::TokenProvider)（token 注入、853004 静默
//! 刷新），产出可直接使用的 [`WecomClient`](crate::WecomClient)（或仅 transport）。

use std::sync::Arc;
use std::time::Duration;

use wecom_auth::TokenProvider;
use wecom_transport::{HttpTransportBackend, Transport};

use crate::backend::WecomBackend;
use crate::client::WecomClient;

/// 默认 API base URL。
///
/// 缺省时用于兜底；CLI 侧可经 `WECOM_CLI_BASE_URL` / `config.json::base_url`
/// 覆盖（`custom-endpoint` feature）。
pub const DEFAULT_BASE_URL: &str = "https://qyapi.weixin.qq.com/cli";

/// 错误提示中的默认命令名（如「请先运行 `wecom-cli auth init` 登录」）。
pub const DEFAULT_BIN_NAME: &str = "wecom-cli";

/// 带鉴权的客户端构建器。
///
/// # Example
///
/// ```ignore
/// let client = WecomClientBuilder::new()
///     .token_provider(Arc::new(provider))
///     .timeout(Duration::from_secs(10))
///     .build()
///     .await?;
/// ```
#[derive(Clone)]
pub struct WecomClientBuilder {
    base_url: String,
    provider: Option<Arc<dyn TokenProvider>>,
    initial_token: Option<String>,
    timeout: Option<Duration>,
    headers: Vec<(String, String)>,
    bin_name: String,
}

impl Default for WecomClientBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl WecomClientBuilder {
    /// 创建构建器：默认指向 product/正式环境网关。
    pub fn new() -> Self {
        Self {
            base_url: DEFAULT_BASE_URL.to_string(),
            provider: None,
            initial_token: None,
            timeout: None,
            headers: Vec::new(),
            bin_name: DEFAULT_BIN_NAME.to_string(),
        }
    }

    /// 覆写服务 base URL。
    #[must_use]
    pub fn base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into();
        self
    }

    /// 设置 token provider（token 注入与 853004 静默刷新的来源）。
    #[must_use]
    pub fn token_provider(mut self, provider: Arc<dyn TokenProvider>) -> Self {
        self.provider = Some(provider);
        self
    }

    /// 预置初始 token（缺省时经 provider 读取；如 CLI 的环境变量覆写）。
    #[must_use]
    pub fn initial_token(mut self, token: impl Into<String>) -> Self {
        self.initial_token = Some(token.into());
        self
    }

    /// 设置每请求超时。
    #[must_use]
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// 添加默认请求头（对每个请求生效；header 名/值在 build 时统一校验）。
    #[must_use]
    pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.push((name.into(), value.into()));
        self
    }

    /// 设置错误提示中的命令名（影响 `auth init` 引导文案）。
    #[must_use]
    pub fn bin_name(mut self, bin_name: impl Into<String>) -> Self {
        self.bin_name = bin_name.into();
        self
    }

    /// Build the authenticated [`WecomClient`](crate::WecomClient).
    ///
    /// 等价于 [`build_transport`](Self::build_transport) 之后经
    /// [`WecomClient::from_transport`](crate::WecomClient::from_transport)
    /// 组装完整客户端（自动挂网关协议端点目录，直接可调服务方法）。
    ///
    /// # Errors
    ///
    /// 底层传输配置非法（header 校验等）或客户端构建失败时返回
    /// [`wecom::Error`]。
    pub async fn build(self) -> Result<WecomClient, wecom::Error> {
        let transport = self.build_transport().await.map_err(wecom::Error::from)?;
        WecomClient::from_transport(transport)
    }

    /// Build only the authenticated [`Transport`]（不组装客户端）。
    ///
    /// 适合需要完全掌控 [`wecom::Client`] 配置（沙箱 FS、helper、扩展命令、
    /// 端点目录覆写等）的调用方。
    ///
    /// 初始 token 优先取 [`initial_token`](Self::initial_token)，否则经
    /// provider 读取（无凭据为 None，不报错）；transport 装饰为
    /// [`WecomBackend`]（动态注入 + 门禁 + 853004 静默刷新）。
    ///
    /// # Errors
    ///
    /// 底层传输配置非法（header 校验等）或初始 token 读取失败时返回
    /// [`wecom_transport::Error`]。
    pub async fn build_transport(self) -> Result<Transport, wecom_transport::Error> {
        let mut builder = HttpTransportBackend::builder().base_url(self.base_url);
        for (name, value) in &self.headers {
            builder = builder.header(name.as_str(), value.as_str());
        }
        if let Some(timeout) = self.timeout {
            builder = builder.timeout(timeout);
        }
        let transport = builder.build()?;

        let initial_token = match self.initial_token {
            Some(token) => Some(token),
            None => match &self.provider {
                Some(provider) => provider
                    .access_token()
                    .await
                    .map_err(wecom_transport::Error::from)?
                    .map(|t| t.into_inner()),
                None => None,
            },
        };

        Ok(transport.wrap_backend(move |backend| {
            Arc::new(
                WecomBackend::new(backend, self.provider, initial_token)
                    .with_bin_name(self.bin_name),
            )
        }))
    }
}

#[cfg(test)]
pub(crate) mod tests {
    //! ## 模块摘要：builder（WecomClientBuilder）
    //!
    //! ### 关键接口
    //! - [WecomClientBuilder::build] — 组合 HTTP 配置与 token provider，
    //!   产出 WecomClient（含网关端点目录）
    //! - [WecomClientBuilder::build_transport] — 仅产出带鉴权的 transport

    use std::sync::Arc;

    use serde_json::json;
    use wecom_auth::{BotCredential, BotGatewayTokenProvider, Credentials, MemoryCredentialStore};
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::*;

    /// 装配鉴权引导端点（FlatRes 信封 + SuppressAuth）。
    pub(crate) fn auth_ep(url: &str) -> wecom_transport::Endpoint {
        wecom_auth::auth_endpoint(url)
    }

    /// P0：build 后 token 经 provider 从存储读取，RequireAuth 端点可正常注入
    /// 条件：存储含 bot + token；业务 mock 要求 Bearer tok-stored
    /// 断言：经 client.transport 调用成功
    #[tokio::test]
    async fn build_reads_initial_token_from_provider() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api"))
            .and(wiremock::matchers::header(
                "authorization",
                "Bearer tok-stored",
            ))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(json!({"result": "{\"ok\":true}"})),
            )
            .expect(1)
            .mount(&server)
            .await;

        let store = MemoryCredentialStore::new(Credentials {
            bot: Some(BotCredential::new("bot1".into(), "secret1".into())),
            token: Some("tok-stored".into()),
        })
        .shared();
        let provider = BotGatewayTokenProvider::from_store(store)
            .with_auth_endpoint(auth_ep(&format!("{}/bootstrap", server.uri())));

        let client = WecomClientBuilder::new()
            .base_url(server.uri())
            .token_provider(Arc::new(provider))
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .await
            .unwrap();

        let endpoint = wecom_transport::Endpoint::new()
            .with(wecom_transport::HttpEndpoint::new("/api").with_service(server.uri()));
        let v = client
            .transport()
            .invoke(&endpoint, json!({}))
            .await
            .unwrap()
            .into_result()
            .unwrap();
        assert_eq!(v, json!({"ok": true}));
        server.verify().await;
    }

    /// P1：initial_token 覆盖 provider 读取的存储 token
    /// 条件：builder 传 initial_token("tok-override")，存储 token 为 tok-stored
    /// 断言：请求携带 Bearer tok-override
    #[tokio::test]
    async fn initial_token_overrides_provider() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api"))
            .and(wiremock::matchers::header(
                "authorization",
                "Bearer tok-override",
            ))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(json!({"result": "{\"ok\":true}"})),
            )
            .expect(1)
            .mount(&server)
            .await;

        let store = MemoryCredentialStore::new(Credentials {
            bot: Some(BotCredential::new("bot1".into(), "secret1".into())),
            token: Some("tok-stored".into()),
        })
        .shared();
        let provider = BotGatewayTokenProvider::from_store(store);

        let transport = WecomClientBuilder::new()
            .base_url(server.uri())
            .token_provider(Arc::new(provider))
            .initial_token("tok-override")
            .build_transport()
            .await
            .unwrap();

        let endpoint = wecom_transport::Endpoint::new()
            .with(wecom_transport::HttpEndpoint::new("/api").with_service(server.uri()));
        let v = transport
            .invoke(&endpoint, json!({}))
            .await
            .unwrap()
            .into_result()
            .unwrap();
        assert_eq!(v, json!({"ok": true}));
        server.verify().await;
    }

    /// P1：无 provider 时 build 成功，transport 无鉴权能力（无 token 缓存）
    #[tokio::test]
    async fn build_without_provider() {
        let client = WecomClientBuilder::new()
            .base_url("http://localhost")
            .build()
            .await
            .unwrap();
        assert_eq!(client.transport().name(), "http");
    }

    /// P1：build 产出的 WecomClient 挂有网关端点目录（`from_transport` 路径）
    /// 条件：经 build 组装
    /// 断言：client.inner() 可正常产出 endpoint、暴露 service API
    #[tokio::test]
    async fn build_attaches_gateway_catalog() {
        let client = WecomClientBuilder::new()
            .base_url("http://localhost")
            .build()
            .await
            .unwrap();
        // 端点目录由 Client 内部持有；此处仅验证客户端可正常产出 endpoint。
        let _endpoint = client.inner().endpoint("media/upload");
    }
}
