//! 鉴权出网后端：动态 Authorization 注入 + 853004 静默刷新 + 重放。
//!
//! 所有请求统一经 [`WecomBackend`] 转发，它实现两类跨切面能力：
//! - **authorization**：持有 token 即注入 `Authorization: Bearer <token>`
//!   （无 token 则忽略）；挂 [`RequireAuth`] 的端点先过门禁——无可用 token
//!   直接报 [`AuthError::MissingCredentials`] 且请求不发出。换取 token 的
//!   引导端点挂 [`SuppressAuth`] 抑制注入。
//! - **token refresh**：命中 853004 时经
//!   [`TokenProvider`](wecom_auth::TokenProvider) 静默换 token
//!   （存储持久化 + 内存缓存）并重放原请求一次。

use std::borrow::Cow;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, RwLock};

use wecom_auth::{AuthError, RequireAuth, SuppressAuth, TokenProvider};
use wecom_transport::{
    Endpoint, HttpRequestPayload, RequestOptions, TransportBackend, TransportResponse,
};

/// token 失效业务错误码（后台下发）：命中后经 token provider 静默换 token 并重试。
pub const TOKEN_EXPIRED_ERRCODE: i64 = 853004;

// ── wecom-runtime 统一出网后端 ──────────────────────────────

/// 统一出网后端：所有请求都经它转发，负责
/// - 持有 token 即注入 `Authorization: Bearer <token>`（无论端点是否挂
///   [`RequireAuth`]；无 token 则忽略不注入）；挂 [`RequireAuth`] 的端点在
///   **前置门禁** 校验：无可用 token 直接报 [`AuthError::MissingCredentials`]，
///   请求不发出。携带 [`SuppressAuth`] 的端点（换取 token 的引导接口）即使有
///   token 也不注入；
/// - 捕获 853004（token 失效）→ 经 [`TokenProvider`] 重新换取 token
///   （存储持久化 + 内存缓存）→ 重放原请求一次（未注入 token 的请求不参与刷新）。
///
/// 扁平响应等请求/响应封装由 wecom-transport 的 endpoint envelope 驱动，
/// 本层不做特殊分流。
///
/// 所有载荷均可重放：经 [`HttpRequestPayload`](wecom_transport::HttpRequestPayload)
/// 工厂克隆（Arc 零成本），重放 = 再次 build。
#[derive(Clone)]
pub struct WecomBackend {
    /// 底层 HTTP 传输（信封解析 + 长任务轮询路径）。
    inner: Arc<dyn TransportBackend>,
    /// Token provider（无认证来源时为 None：不注入、不刷新）。
    provider: Option<Arc<dyn TokenProvider>>,
    /// 缓存的 token（初始由构建方注入，刷新后更新）：
    /// 需要授权的请求在调用时经它注入 Authorization 头。
    token: Arc<RwLock<Option<String>>>,
    /// 串行化刷新，避免并发请求重复换取 token。
    refresh_lock: Arc<tokio::sync::Mutex<()>>,
    /// 错误提示中的命令名（如 `wecom-cli`，用于引导用户运行 `auth init`）。
    bin_name: Arc<str>,
}

// 不输出 token 与 provider 内部凭据。
impl std::fmt::Debug for WecomBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WecomBackend")
            .field("backend", &self.inner.name())
            .finish_non_exhaustive()
    }
}

impl WecomBackend {
    /// Wrap an inner backend with authentication.
    ///
    /// `provider` 为认证来源（`None` 表示无凭据：不注入、不刷新）；
    /// `token` 为初始缓存的 token（如来自凭据存储 / 环境变量）。
    pub fn new(
        inner: Arc<dyn TransportBackend>,
        provider: Option<Arc<dyn TokenProvider>>,
        token: Option<String>,
    ) -> Self {
        Self {
            inner,
            provider,
            token: Arc::new(RwLock::new(token)),
            refresh_lock: Arc::new(tokio::sync::Mutex::new(())),
            bin_name: Arc::from(crate::DEFAULT_BIN_NAME),
        }
    }

    /// 设置错误提示中的命令名（影响门禁 / 刷新失败的引导文案）。
    #[must_use]
    pub fn with_bin_name(mut self, bin_name: impl Into<String>) -> Self {
        self.bin_name = Arc::from(bin_name.into().as_str());
        self
    }

    /// 当前缓存的 token（构建时注入或刷新后更新）。
    pub fn cached_token(&self) -> Option<String> {
        self.token.read().unwrap_or_else(|e| e.into_inner()).clone()
    }

    fn store_token(&self, token: &str) {
        *self.token.write().unwrap_or_else(|e| e.into_inner()) = Some(token.to_owned());
    }

    /// 经 token provider 静默刷新：provider 内部做并发刷新合并与落盘，
    /// 结果写入内存缓存。
    ///
    /// `stale_token` 为本次失败请求所用的 token；`options` 为触发刷新的
    /// 请求选项（含 transport 默认叠加的 headers / timeout / extensions），
    /// 引导请求复用它们，保证传输配置一致。
    async fn refresh_token(
        &self,
        stale_token: Option<&str>,
        options: RequestOptions,
    ) -> Result<String, AuthError> {
        let Some(provider) = &self.provider else {
            return Err(AuthError::MissingCredentials(format!(
                "无 bot 凭据，无法静默刷新 token，请重新运行 `{}` auth init",
                self.bin_name
            )));
        };

        // 串行化刷新由 provider 内部的并发合并语义兜底；本层锁进一步避免
        // 多个并发请求同时进入 provider 刷新路径。
        let _guard = self.refresh_lock.lock().await;
        let token = provider.refresh(stale_token, Some(options)).await?;
        self.store_token(token.as_str());
        tracing::info!("access token refreshed (853004) and cached");
        Ok(token.into_inner())
    }
}

impl TransportBackend for WecomBackend {
    fn execute<'a>(
        &'a self,
        endpoint: Cow<'a, Endpoint>,
        payload: HttpRequestPayload,
        options: RequestOptions,
    ) -> Pin<
        Box<
            dyn Future<Output = std::result::Result<TransportResponse, wecom_transport::Error>>
                + Send
                + 'a,
        >,
    > {
        Box::pin(async move {
            // 所有载荷均可重放：clone 工厂（Arc 零成本），重放 = 再次 build。
            let replay_payload = payload.clone();

            let mut options = options;

            // 抑制注入：换取 token 的引导端点即使持有 token 也不携带 Authorization。
            let sent_token = if endpoint.as_ref().get::<SuppressAuth>().is_some() {
                None
            } else {
                let token = self.cached_token();

                // 门禁前置：挂 RequireAuth 的端点必须已有可用 token，否则请求不发出。
                if endpoint.as_ref().get::<RequireAuth>().is_some() && token.is_none() {
                    tracing::debug!("endpoint requires auth but no token available");
                    return Err(AuthError::MissingCredentials(format!(
                        "该请求需要授权，请先运行 `{}` auth init 登录",
                        self.bin_name
                    ))
                    .into());
                }

                // 有 token 就注入（无论是否挂 RequireAuth），无 token 则忽略；
                // 记下本次发送值供 853004 刷新去重。
                token
                    .clone()
                    .inspect(|token| set_bearer_token(&mut options, token))
            };

            let err = match self
                .inner
                .execute(endpoint.clone(), payload, options.clone())
                .await
            {
                Ok(resp) => return Ok(resp),
                Err(err) => err,
            };
            if !is_token_expired(&err) {
                return Err(err);
            }
            // 未注入 token 的请求不可能因 token 过期失败（无 token / 抑制注入的
            // 引导端点）——不参与刷新，直接返回原错误。
            if sent_token.is_none() {
                tracing::warn!("token expired but no token was sent");
                return Err(err);
            }
            // 无认证来源时无法刷新 token，不参与自动刷新。
            if self.provider.is_none() {
                tracing::warn!("missing token provider, cannot refresh token");
                return Err(err);
            }

            tracing::info!("token expired (853004), attempting silent refresh");
            match self
                .refresh_token(sent_token.as_deref(), options.clone())
                .await
            {
                Ok(token) => {
                    tracing::info!("token refreshed, retrying the original request");
                    set_bearer_token(&mut options, &token);
                    // 重放 = 重新走完整流水线：发送链会再次 build。
                    self.inner.execute(endpoint, replay_payload, options).await
                }
                Err(refresh_err) => {
                    tracing::warn!(error = %refresh_err, "token refresh failed, returning the original error");
                    Err(err)
                }
            }
        })
    }

    fn name(&self) -> &str {
        self.inner.name()
    }
}

/// 是否为可触发静默刷新的 token 失效错误（ApiError 853004）。
pub fn is_token_expired(err: &wecom_transport::Error) -> bool {
    matches!(
        err,
        wecom_transport::Error::Api {
            code: Some(TOKEN_EXPIRED_ERRCODE),
            ..
        }
    )
}

/// 在请求选项上覆写 `Authorization: Bearer <token>` 头（标记敏感）。
pub fn set_bearer_token(options: &mut RequestOptions, token: &str) {
    let Ok(mut value) = reqwest::header::HeaderValue::from_str(&format!("Bearer {token}")) else {
        return;
    };
    value.set_sensitive(true);
    options
        .wire
        .headers
        .insert(reqwest::header::AUTHORIZATION, value);
}

#[cfg(test)]
mod tests {
    //! ## 模块摘要：backend（鉴权出网后端）
    //!
    //! ### 关键接口
    //! - [WecomBackend] — 动态注入 / RequireAuth 门禁 / 853004 静默刷新与重放
    //! - [is_token_expired] / [set_bearer_token] — 错误判定与 token 注入原语
    //!
    //! ### 关键分支与异常路径
    //! - 挂 RequireAuth + 有 token → 注入；无 token → 门禁报错且请求不发出
    //! - 不挂标记：有 token 注入、无 token 忽略
    //! - SuppressAuth：持有 token 也不注入
    //! - 853004：刷新复用触发请求 options（剥离旧 Authorization），以新 token 重放一次

    use assert_json_diff::assert_json_eq;
    use serde_json::json;
    use std::sync::Arc;
    use wiremock::matchers::{method, path};
    use wiremock::{Match, Mock, MockServer, Request, ResponseTemplate};

    use wecom_auth::{BotCredential, Credentials, MemoryCredentialStore};
    use wecom_transport::{HttpTransportBackend, RequestOptions, Transport};

    use super::*;

    /// 装配鉴权引导端点（与 provider 侧引导端点等价：FlatRes 信封 + SuppressAuth）。
    fn auth_ep(url: &str) -> wecom_transport::Endpoint {
        wecom_auth::auth_endpoint(url)
    }

    /// 匹配器：请求不含 Authorization 头。
    struct NoAuthorization;
    impl Match for NoAuthorization {
        fn matches(&self, request: &Request) -> bool {
            request.headers.get("authorization").is_none()
        }
    }

    /// 构造装饰了 [WecomBackend] 的 Transport（内层为真实 HttpTransportBackend）。
    ///
    /// `bot`/`token` 经内存凭据存储 + BotGatewayTokenProvider 注入。
    fn wrapped_transport(
        base_url: &str,
        bot: Option<BotCredential>,
        token: Option<&str>,
        auth_endpoint_url: &str,
    ) -> Transport {
        let store = MemoryCredentialStore::new(Credentials { bot, token: None }).shared();
        let provider = wecom_auth::BotGatewayTokenProvider::from_store(store)
            .with_auth_endpoint(auth_ep(auth_endpoint_url));
        HttpTransportBackend::builder()
            .base_url(base_url)
            .build()
            .expect("valid")
            .wrap_backend(|backend| {
                Arc::new(WecomBackend::new(
                    backend,
                    Some(Arc::new(provider)),
                    token.map(str::to_owned),
                ))
            })
    }

    /// 构造带 base_url / envelope 的 HTTP endpoint（鉴权能力由各用例自行挂载）。
    fn ep(base: &str, path_str: &str) -> wecom_transport::Endpoint {
        wecom_transport::Endpoint::new()
            .with(wecom_transport::HttpEndpoint::new(path_str).with_service(base))
    }

    fn api_error(code: Option<i64>) -> wecom_transport::Error {
        wecom_transport::Error::Api {
            message: "err".into(),
            action: "test".into(),
            code,
            body: Box::new(serde_json::Value::Null),
        }
    }

    /// P0：[is_token_expired] 853004 命中刷新
    /// 条件：构造 code=853004 的 Api 错误
    /// 断言：is_token_expired() 返回 true
    #[test]
    fn token_expired_errcode_matches() {
        assert!(is_token_expired(&api_error(Some(TOKEN_EXPIRED_ERRCODE))));
    }

    /// P0：[is_token_expired] 其它业务错误码 / code 缺失 / 非 Api 变体均不命中
    /// 条件：分别构造 code=40001、code=None、Error::Other
    /// 断言：is_token_expired() 均返回 false
    #[test]
    fn other_errors_do_not_match() {
        assert!(!is_token_expired(&api_error(Some(40001))));
        assert!(!is_token_expired(&api_error(None)));
        assert!(!is_token_expired(&wecom_transport::Error::Other(
            "x".into()
        )));
    }

    /// P0：[set_bearer_token] 写入 Bearer 头且标记敏感
    /// 条件：默认 options 写入 tok-1
    /// 断言：写入后 Authorization == "Bearer tok-1"，且 is_sensitive()
    #[test]
    fn set_bearer_token_marks_sensitive() {
        let mut options = RequestOptions::default();
        set_bearer_token(&mut options, "tok-1");
        let value = options
            .wire
            .headers
            .get(reqwest::header::AUTHORIZATION)
            .unwrap();
        assert_eq!(value.to_str().unwrap(), "Bearer tok-1");
        assert!(value.is_sensitive(), "token 头应标记敏感");
    }

    /// P1：[set_bearer_token] 覆写已有 Authorization 头
    /// 条件：先写 "old" 再写 "new"
    /// 断言：Authorization == "Bearer new"
    #[test]
    fn set_bearer_token_overwrites() {
        let mut options = RequestOptions::default();
        set_bearer_token(&mut options, "old");
        set_bearer_token(&mut options, "new");
        let value = options
            .wire
            .headers
            .get(reqwest::header::AUTHORIZATION)
            .unwrap();
        assert_eq!(value.to_str().unwrap(), "Bearer new");
    }

    /// P1：[WecomBackend] 经 wrap_backend 装饰后 name 委托内层
    /// 条件：对 HttpTransportBackend 调用 wrap_backend 包上 WecomBackend
    /// 断言：transport.name() == "http"
    #[test]
    fn wrap_backend_decorates_in_place() {
        let transport = HttpTransportBackend::builder()
            .base_url("http://localhost")
            .build()
            .expect("valid");
        let transport = transport.wrap_backend(|backend| {
            Arc::new(WecomBackend::new(backend, None, Some("tok-1".into())))
        });
        assert_eq!(transport.name(), "http");
    }

    /// P1：[WecomBackend::Debug] 不泄露缓存 token
    /// 条件：构造含 "cached-token" 的 WecomBackend 并格式化
    /// 断言：Debug 输出不含该敏感值
    #[test]
    fn debug_does_not_leak_secrets() {
        let backend = WecomBackend::new(
            Arc::new(HttpTransportBackend::default()),
            None,
            Some("cached-token".into()),
        );
        let dbg = format!("{backend:?}");
        assert!(!dbg.contains("cached-token"), "token 泄露: {dbg}");
    }

    /// P1：[WecomBackend] 无 provider 时 provider 为 None，token 缓存保持正常
    #[test]
    fn no_provider_token_cached() {
        let backend = WecomBackend::new(
            Arc::new(HttpTransportBackend::default()),
            None,
            Some("cached-token".into()),
        );
        assert!(backend.provider.is_none());
        assert_eq!(backend.cached_token().as_deref(), Some("cached-token"));
    }

    // ── 动态 Authorization 注入 ───────────────────────────────

    /// P0：[WecomBackend] 挂 RequireAuth + 有 token → 调用时注入 Authorization 头
    /// 条件：endpoint 挂 RequireAuth，token=tok-x；mock 要求 authorization: Bearer tok-x
    /// 断言：invoke 成功，into_result()=={"ok":true}，mock 命中
    #[tokio::test]
    async fn injects_auth_when_require_auth_and_token_available() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/auth"))
            .and(wiremock::matchers::header("authorization", "Bearer tok-x"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(json!({"result": "{\"ok\":true}"})),
            )
            .expect(1)
            .mount(&server)
            .await;

        let transport = wrapped_transport(&server.uri(), None, Some("tok-x"), "http://unused");
        let endpoint = ep(&server.uri(), "/auth").with(RequireAuth);
        let v = transport
            .invoke(&endpoint, json!({}))
            .await
            .unwrap()
            .into_result()
            .unwrap();
        assert_json_eq!(v, json!({"ok": true}));
        server.verify().await;
    }

    /// P0：[WecomBackend] 挂 RequireAuth + 无 token → Err(AuthError)，请求不发出
    /// 条件：endpoint 挂 RequireAuth，无 token；mock expect(0)
    /// 断言：invoke 返回 Err(wecom_transport::Error::Other(AuthError))，mock 未被调用
    #[tokio::test]
    async fn rejects_require_auth_without_token() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/auth"))
            .respond_with(ResponseTemplate::new(200))
            .expect(0)
            .mount(&server)
            .await;

        let transport = wrapped_transport(&server.uri(), None, None, "http://unused");
        let endpoint = ep(&server.uri(), "/auth").with(RequireAuth);
        let err = transport.invoke(&endpoint, json!({})).await.unwrap_err();
        match err {
            wecom_transport::Error::Other(e) => {
                let inner = e.downcast_ref::<AuthError>();
                assert!(
                    inner.is_some_and(|e| matches!(e, AuthError::MissingCredentials(_))),
                    "expected AuthError::MissingCredentials, got {inner:?}"
                );
            }
            other => panic!("expected Error::Other(AuthError), got {other:?}"),
        }
        server.verify().await;
    }

    /// P0：[WecomBackend] 未挂 RequireAuth 能力 + 有 token → 仍注入 Authorization 头
    /// 条件：endpoint 不挂 RequireAuth（如 ServiceDiscovery），token=tok-x；
    ///       mock 要求 authorization: Bearer tok-x
    /// 断言：invoke 成功，into_result()=={"ok":true}，mock 命中（证明注入）
    #[tokio::test]
    async fn injects_auth_on_endpoint_without_require_auth() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/open"))
            .and(wiremock::matchers::header("authorization", "Bearer tok-x"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(json!({"result": "{\"ok\":true}"})),
            )
            .expect(1)
            .mount(&server)
            .await;

        let transport = wrapped_transport(&server.uri(), None, Some("tok-x"), "http://unused");
        let endpoint = ep(&server.uri(), "/open");
        let v = transport
            .invoke(&endpoint, json!({}))
            .await
            .unwrap()
            .into_result()
            .unwrap();
        assert_json_eq!(v, json!({"ok": true}));
        server.verify().await;
    }

    /// P0：[WecomBackend] 无 token + 未挂 RequireAuth 门禁（如未登录时的 ServiceDiscovery）
    /// → 不注入 Authorization 头，请求正常发出
    /// 条件：endpoint 不挂 RequireAuth，无 token；mock 要求无 Authorization 头
    /// 断言：invoke 成功，into_result()=={"ok":true}，mock 命中
    #[tokio::test]
    async fn no_token_no_require_auth_omits_auth_header() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/open"))
            .and(NoAuthorization)
            .respond_with(
                ResponseTemplate::new(200).set_body_json(json!({"result": "{\"ok\":true}"})),
            )
            .expect(1)
            .mount(&server)
            .await;

        let transport = wrapped_transport(&server.uri(), None, None, "http://unused");
        let endpoint = ep(&server.uri(), "/open");
        let v = transport
            .invoke(&endpoint, json!({}))
            .await
            .unwrap()
            .into_result()
            .unwrap();
        assert_json_eq!(v, json!({"ok": true}));
        server.verify().await;
    }

    // ── FlatRes（扁平响应）───────────────────────────────

    /// P0：[WecomBackend] FlatRes 引导端点挂 SuppressAuth → 即使有 token 也不注入 Authorization
    /// 条件：endpoint 配 FlatRes envelope + SuppressAuth，有旧 token；
    ///       mock 返回 {errcode:0, token:"t1"} 且要求无 Authorization
    /// 断言：into_result() == {"token":"t1"}；mock 命中（未注入 token）
    #[tokio::test]
    async fn flat_envelope_bootstrap_suppresses_auth() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/bootstrap"))
            .and(NoAuthorization)
            .respond_with(
                ResponseTemplate::new(200).set_body_json(json!({"errcode": 0, "token": "t1"})),
            )
            .expect(1)
            .mount(&server)
            .await;

        let transport = wrapped_transport(&server.uri(), None, Some("old-token"), "http://unused");
        let endpoint = wecom_transport::Endpoint::new().with(
            wecom_transport::HttpEndpoint::new("/bootstrap")
                .with_service(server.uri())
                .with_res_envelope(wecom_auth::FlatRes),
        );
        let endpoint = endpoint.with(SuppressAuth);
        let v = transport
            .invoke(&endpoint, json!({}))
            .await
            .unwrap()
            .into_result()
            .unwrap();
        assert_json_eq!(v, json!({"token": "t1"}));
        server.verify().await;
    }

    // ── 853004 静默刷新（options 来自 execute）─────────────────

    /// P0：[WecomBackend] 853004 刷新复用触发请求的 options（自定义 header），
    /// 引导请求剥离失效的旧 Authorization 头，随后以新 token 重放原请求。
    /// 条件：业务请求带 x-run-scope + 旧 token → mock 返回 853004；
    ///       引导端点断言带 x-run-scope 且无 Authorization → 返回新 token；
    ///       重试断言带新 token + x-run-scope → 成功
    /// 断言：最终 into_result()=={"ok":true}，三个 mock 均命中
    #[tokio::test]
    async fn refresh_reuses_execute_options_without_stale_auth() {
        let server = MockServer::start().await;
        let auth_url = format!("{}/bootstrap", server.uri());

        // 1. 原请求：旧 token + 自定义 header → 853004
        Mock::given(method("POST"))
            .and(path("/api"))
            .and(wiremock::matchers::header(
                "authorization",
                "Bearer tok-old",
            ))
            .and(wiremock::matchers::header("x-run-scope", "run-1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "error": {"code": 853004, "message": "token expired"}
            })))
            .expect(1)
            .mount(&server)
            .await;

        // 2. 引导请求：复用自定义 header，但不带失效的旧 Authorization
        Mock::given(method("POST"))
            .and(path("/bootstrap"))
            .and(wiremock::matchers::header("x-run-scope", "run-1"))
            .and(NoAuthorization)
            .respond_with(
                ResponseTemplate::new(200).set_body_json(json!({"errcode": 0, "token": "tok-new"})),
            )
            .expect(1)
            .mount(&server)
            .await;

        // 3. 重放：新 token + 自定义 header → 成功
        Mock::given(method("POST"))
            .and(path("/api"))
            .and(wiremock::matchers::header(
                "authorization",
                "Bearer tok-new",
            ))
            .and(wiremock::matchers::header("x-run-scope", "run-1"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(json!({"result": "{\"ok\":true}"})),
            )
            .expect(1)
            .mount(&server)
            .await;

        let transport = wrapped_transport(
            &server.uri(),
            Some(BotCredential::new("bot1".into(), "secret1".into())),
            Some("tok-old"),
            &auth_url,
        );
        let endpoint = ep(&server.uri(), "/api").with(RequireAuth);

        let v = transport
            .invoke(&endpoint, json!({}))
            .header("x-run-scope", "run-1")
            .await
            .unwrap()
            .into_result()
            .unwrap();
        assert_json_eq!(v, json!({"ok": true}));
        server.verify().await;
    }
}
