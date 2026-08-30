//! Token Provider：访问令牌的读取与刷新抽象。
//!
//! [`TokenProvider`] 是运行时（鉴权 Transport）与认证来源之间的边界：
//! 运行时只依赖该 trait 注入 token、刷新失效 token，不关心凭据存于何处。
//! [`BotGatewayTokenProvider`] 为 AI Bot CLI 网关的默认实现
//! （botid+secret 签名引导换取 Bearer token）。

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use wecom_transport::{
    Endpoint, HttpTransportBackend, RequestOptions, Transport, TransportBackend,
};

use crate::bootstrap::{BindSource, fetch_auth};
use crate::bot::BotCredential;
use crate::credentials::{CredentialStore, Credentials};
use crate::error::AuthError;
use crate::gateway;

/// Boxed async future（`Send`），TokenProvider 异步方法的返回类型。
pub type TokenFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T, AuthError>> + Send + 'a>>;

/// 访问令牌（Bearer token）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccessToken(String);

impl AccessToken {
    pub fn new(token: impl Into<String>) -> Self {
        Self(token.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_inner(self) -> String {
        self.0
    }
}

impl std::fmt::Display for AccessToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

// 不输出 token 本体。
impl serde::Serialize for AccessToken {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str("***")
    }
}

/// Token Provider 抽象：运行时经它获取与刷新访问令牌。
///
/// - [`access_token`](TokenProvider::access_token)：读取当前可用 token
///   （无凭据时 `None`；实现方**不得**发起网络请求）；
/// - [`refresh`](TokenProvider::refresh)：换取新 token 并持久化。
pub trait TokenProvider: Send + Sync {
    /// 当前可用 token；无凭据时返回 `None`（不发起网络请求）。
    fn access_token<'a>(&'a self) -> TokenFuture<'a, Option<AccessToken>>;

    /// 刷新（换取）新 token 并持久化。
    ///
    /// `stale_token` 为本次失败请求所用的失效 token；实现方应做**并发刷新
    /// 合并**——若存储中的 token 已不同于 `stale_token`，说明并发请求已完成
    /// 刷新，直接复用、不再重复换取。
    ///
    /// `options` 为触发刷新的请求携带的请求选项（含 transport 默认叠加的
    /// headers / timeout / extensions）；实现方应让引导请求复用它们，保证
    /// 传输配置一致（并剥离其中已注入的失效 Authorization 头）。
    fn refresh<'a>(
        &'a self,
        stale_token: Option<&'a str>,
        options: Option<RequestOptions>,
    ) -> TokenFuture<'a, AccessToken>;
}

/// AI Bot CLI 网关 Token Provider：botid+secret 签名调用
/// `/cgi-bin/aibot/cli/get_cli_config` 换取 Bearer token。
///
/// - token 读取 / 刷新均经 [`CredentialStore`]（刷新结果写回存储）；
/// - 并发刷新合并：存储中的 token 已不同于 `stale_token` 时直接复用；
/// - 引导请求复用触发请求的 `options`（剥离失效 Authorization 头），
///   经内部原始 HTTP 后端发出（端点 URL 为绝对地址，已挂扁平信封与
///   [`SuppressAuth`](crate::gateway::SuppressAuth) 抑制标记）。
#[derive(Clone)]
pub struct BotGatewayTokenProvider {
    store: Arc<dyn CredentialStore>,
    auth_endpoint: Endpoint,
    /// 引导请求的原始 HTTP 后端（默认 [`HttpTransportBackend::default`]，
    /// 测试可注入指向 mock server 的实例）。
    backend: Arc<dyn TransportBackend>,
    bind_source: BindSource,
    /// bot 凭据覆写：`None` 时每次刷新从 [`CredentialStore`] 读取。
    bot: Option<BotCredential>,
}

impl std::fmt::Debug for BotGatewayTokenProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // 不输出 bot secret 与存储内容。
        f.debug_struct("BotGatewayTokenProvider")
            .field("bind_source", &self.bind_source)
            .finish_non_exhaustive()
    }
}

impl BotGatewayTokenProvider {
    /// 用直接给定的 bot 凭据创建 Provider（默认引导端点）。
    pub fn new(
        bot_id: impl Into<String>,
        secret: impl Into<String>,
        store: Arc<dyn CredentialStore>,
    ) -> Self {
        Self::from_store(store).with_bot(BotCredential::new(bot_id.into(), secret.into()))
    }

    /// 用凭据存储创建 Provider（bot 凭据每次刷新时从存储读取；
    /// 引导端点默认指向 product/正式环境）。
    pub fn from_store(store: Arc<dyn CredentialStore>) -> Self {
        Self {
            store,
            auth_endpoint: gateway::auth_endpoint(gateway::DEFAULT_AUTH_ENDPOINT),
            backend: Arc::new(HttpTransportBackend::default()),
            bind_source: BindSource::Interactive,
            bot: None,
        }
    }

    /// 覆写鉴权引导端点（如测试注入 mock URL、`custom-endpoint` 环境）。
    #[must_use]
    pub fn with_auth_endpoint(mut self, endpoint: Endpoint) -> Self {
        self.auth_endpoint = endpoint;
        self
    }

    /// 覆写引导请求的原始 HTTP 后端（测试注入）。
    #[must_use]
    pub fn with_backend(mut self, backend: Arc<dyn TransportBackend>) -> Self {
        self.backend = backend;
        self
    }

    /// 覆写 bot 凭据（固定值，优先于存储中的 bot）。
    #[must_use]
    pub fn with_bot(mut self, bot: BotCredential) -> Self {
        self.bot = Some(bot);
        self
    }

    /// 覆写绑定来源（默认 [`BindSource::Interactive`]）。
    #[must_use]
    pub fn with_bind_source(mut self, bind_source: BindSource) -> Self {
        self.bind_source = bind_source;
        self
    }

    /// 解析本次刷新使用的 bot 凭据：显式覆写优先，否则从存储读取。
    fn resolve_bot(&self, creds: Option<&Credentials>) -> Option<BotCredential> {
        self.bot
            .clone()
            .or_else(|| creds.and_then(|c| c.bot.clone()))
    }
}

impl TokenProvider for BotGatewayTokenProvider {
    fn access_token<'a>(&'a self) -> TokenFuture<'a, Option<AccessToken>> {
        Box::pin(async move {
            Ok(self
                .store
                .load()?
                .and_then(|c| c.token)
                .filter(|t| !t.is_empty())
                .map(AccessToken::new))
        })
    }

    fn refresh<'a>(
        &'a self,
        stale_token: Option<&'a str>,
        options: Option<RequestOptions>,
    ) -> TokenFuture<'a, AccessToken> {
        Box::pin(async move {
            let creds = self.store.load()?;

            // 并发刷新合并：凭据中的 token 已不同于失效值 → 直接复用。
            if let Some(stored) = creds.as_ref().and_then(|c| c.token.as_deref())
                && Some(stored) != stale_token
            {
                tracing::debug!("token already refreshed by a concurrent request, reusing it");
                return Ok(AccessToken::new(stored));
            }

            let bot = self.resolve_bot(creds.as_ref()).ok_or_else(|| {
                AuthError::MissingCredentials("无 bot 凭据，无法静默刷新 token".into())
            })?;

            // 引导请求复用触发请求的 options（headers / timeout / extensions），
            // 保证传输配置一致；但不携带业务 token：剥离其中注入的失效
            // Authorization 头。
            let mut options = options.unwrap_or_default();
            options.headers_mut().remove(reqwest::header::AUTHORIZATION);
            let transport = Transport::new(self.backend.clone(), options);

            let resp = fetch_auth(&transport, &bot, self.bind_source, &self.auth_endpoint).await?;
            let token = match resp.token.clone().filter(|t| !t.is_empty()) {
                Some(token) => token,
                None => {
                    return Err(AuthError::from(wecom_transport::Error::Parse {
                        message: "token 刷新响应缺少访问令牌".to_string(),
                        endpoint: wecom_transport::EndpointHttpExt::full_url(&self.auth_endpoint),
                        body: Box::new(serde_json::to_value(&resp).unwrap_or_default()),
                        source: None,
                    }));
                }
            };

            // 落盘：bot 凭据保持不变，原子更新 token。
            let mut creds = creds.unwrap_or_default();
            creds.bot = creds.bot.or(Some(bot));
            creds.token = Some(token.clone());
            self.store.save(&creds)?;
            tracing::info!("access token refreshed (853004) and persisted");

            Ok(AccessToken::new(token))
        })
    }
}

#[cfg(test)]
mod tests {
    //! ## 模块摘要：provider（TokenProvider 抽象与 BotGatewayTokenProvider）
    //!
    //! ### 关键接口
    //! - [TokenProvider] — token 读取 / 刷新抽象
    //! - [BotGatewayTokenProvider] — botid+secret 签名引导换取 Bearer token
    //!
    //! ### 关键分支与异常路径
    //! - access_token：存储有 token → Some；无凭据 → None
    //! - refresh 并发合并：存储 token != stale → 直接复用（不发引导请求）
    //! - refresh 无 bot 凭据 → MissingCredentials
    //! - refresh 引导端点返回 token → 持久化并返回

    use serde_json::json;

    use super::*;
    use crate::bot::BotCredential;
    use crate::credentials::MemoryCredentialStore;
    use crate::gateway::auth_endpoint;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    /// P0：access_token 读取存储中的 token；无凭据返回 None
    #[tokio::test]
    async fn access_token_reads_store() {
        let store = MemoryCredentialStore::new(Credentials {
            bot: Some(BotCredential::new("b".into(), "s".into())),
            token: Some("tok-1".into()),
        })
        .shared();
        let provider = BotGatewayTokenProvider::from_store(store);
        let token = provider.access_token().await.unwrap();
        assert_eq!(token.as_ref().map(AccessToken::as_str), Some("tok-1"));

        let empty = BotGatewayTokenProvider::from_store(MemoryCredentialStore::default().shared());
        assert!(empty.access_token().await.unwrap().is_none());
    }

    /// P0：refresh 引导端点返回 token → 持久化并返回
    #[tokio::test]
    async fn refresh_fetches_and_persists() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/get_cli_config"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "errcode": 0, "errmsg": "ok", "token": "tok-new",
            })))
            .expect(1)
            .mount(&server)
            .await;

        let store = MemoryCredentialStore::new(Credentials {
            bot: Some(BotCredential::new("bot1".into(), "secret1".into())),
            token: None,
        })
        .shared();
        let provider = BotGatewayTokenProvider::from_store(store.clone())
            .with_auth_endpoint(auth_endpoint(&format!("{}/get_cli_config", server.uri())));

        let token = provider.refresh(Some("tok-old"), None).await.unwrap();
        assert_eq!(token.as_str(), "tok-new");
        assert_eq!(
            store.load().unwrap().unwrap().token.as_deref(),
            Some("tok-new")
        );
        server.verify().await;
    }

    /// P0：refresh 并发合并——存储中的 token 已不同于 stale 时直接复用
    #[tokio::test]
    async fn refresh_reuses_concurrently_refreshed_token() {
        // 引导端点不存在（localhost 空端口）：命中即说明未发引导请求。
        let store = MemoryCredentialStore::new(Credentials {
            bot: Some(BotCredential::new("bot1".into(), "secret1".into())),
            token: Some("tok-fresh".into()),
        })
        .shared();
        let provider = BotGatewayTokenProvider::from_store(store)
            .with_auth_endpoint(auth_endpoint("http://localhost/get_cli_config"));

        let token = provider.refresh(Some("tok-stale"), None).await.unwrap();
        assert_eq!(token.as_str(), "tok-fresh");
    }

    /// P1：refresh 无 bot 凭据 → MissingCredentials
    #[tokio::test]
    async fn refresh_without_bot_fails() {
        let store = MemoryCredentialStore::new(Credentials {
            bot: None,
            token: None,
        })
        .shared();
        let provider = BotGatewayTokenProvider::from_store(store);
        let err = provider.refresh(None, None).await.unwrap_err();
        assert!(matches!(err, AuthError::MissingCredentials(_)));
    }

    /// P1：refresh 引导请求剥离 Authorization 头
    #[tokio::test]
    async fn refresh_strips_authorization_header() {
        struct NoAuthorization;
        impl wiremock::Match for NoAuthorization {
            fn matches(&self, request: &wiremock::Request) -> bool {
                request.headers.get("authorization").is_none()
            }
        }

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/get_cli_config"))
            .and(NoAuthorization)
            .respond_with(
                ResponseTemplate::new(200).set_body_json(json!({"errcode": 0, "token": "tok-2"})),
            )
            .expect(1)
            .mount(&server)
            .await;

        let store = MemoryCredentialStore::new(Credentials {
            bot: Some(BotCredential::new("bot1".into(), "secret1".into())),
            token: None,
        })
        .shared();
        let provider = BotGatewayTokenProvider::from_store(store)
            .with_auth_endpoint(auth_endpoint(&format!("{}/get_cli_config", server.uri())));

        let mut options = RequestOptions::default();
        options.headers_mut().insert(
            reqwest::header::AUTHORIZATION,
            reqwest::header::HeaderValue::from_static("Bearer stale"),
        );
        let token = provider
            .refresh(Some("stale"), Some(options))
            .await
            .unwrap();
        assert_eq!(token.as_str(), "tok-2");
        server.verify().await;
    }

    /// P1：Debug 输出不含 bot secret
    #[test]
    fn debug_does_not_leak_secret() {
        let provider = BotGatewayTokenProvider::new(
            "bot1",
            "super-secret",
            MemoryCredentialStore::default().shared(),
        );
        let dbg = format!("{provider:?}");
        assert!(!dbg.contains("super-secret"), "secret 泄露: {dbg}");
    }
}
