//! transport 模块测试。

use assert_json_diff::assert_json_eq;
use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Match, Mock, MockServer, Request, ResponseTemplate};

use wecom_transport::{HttpTransportBackend, RequestOptions, ResponseEnvelope, Transport};

use super::backend::{TOKEN_EXPIRED_ERRCODE, is_token_expired, set_bearer_token};
use super::capability::{RequireAuth, SuppressAuth};
use super::envelope::{FlatRes, NestedRes};
use super::*;
use crate::auth;
use crate::error::Error as CliError;

/// 测试用鉴权引导端点（固定值，避免依赖 env/config）。
const TEST_AUTH_ENDPOINT: &str = "https://qyapi.weixin.qq.com/cgi-bin/aibot/cli/get_cli_config";

/// 匹配器：请求不含 Authorization 头。
struct NoAuthorization;
impl Match for NoAuthorization {
    fn matches(&self, request: &Request) -> bool {
        request.headers.get("authorization").is_none()
    }
}

/// 构造装饰了 [WecomBackend] 的 Transport（内层为真实 HttpTransportBackend）。
fn wrapped_transport(
    base_url: &str,
    bot: Option<auth::Bot>,
    token: Option<&str>,
    auth_endpoint: &str,
) -> Transport {
    HttpTransportBackend::builder()
        .base_url(base_url)
        .build()
        .expect("valid")
        .wrap_backend(|backend| {
            Arc::new(WecomBackend::new(
                backend,
                bot,
                token.map(str::to_owned),
                auth_ep(auth_endpoint),
            ))
        })
}

/// 构造带 base_url / envelope 的 HTTP endpoint（鉴权能力由各用例自行挂载）。
fn ep(base: &str, path: &str) -> wecom_transport::Endpoint {
    wecom_transport::Endpoint::new()
        .with(wecom_transport::HttpEndpoint::new(path).with_service(base))
}

/// 装配鉴权引导端点（与 auth 侧引导端点等价：FlatRes 信封 + SuppressAuth）。
fn auth_ep(url: &str) -> wecom_transport::Endpoint {
    wecom_transport::Endpoint::new()
        .with(wecom_transport::HttpEndpoint::from_url(url).with_res_envelope(FlatRes))
        .with(SuppressAuth)
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
    assert!(!is_token_expired(&wecom_transport::Error::other(
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
        Arc::new(WecomBackend::new(
            backend,
            Some(auth::Bot::new("bot1".into(), "secret1".into())),
            Some("tok-1".into()),
            auth_ep(TEST_AUTH_ENDPOINT),
        ))
    });
    assert_eq!(transport.name(), "http");
}

/// P1：[WecomBackend::Debug] 不泄露 bot secret 与缓存 token
/// 条件：构造含 "super-secret" / "cached-token" 的 WecomBackend 并格式化
/// 断言：Debug 输出不含这两个敏感值
#[test]
fn debug_does_not_leak_secrets() {
    let backend = WecomBackend::new(
        Arc::new(HttpTransportBackend::default()),
        Some(auth::Bot::new("bot1".into(), "super-secret".into())),
        Some("cached-token".into()),
        auth_ep(TEST_AUTH_ENDPOINT),
    );
    let dbg = format!("{backend:?}");
    assert!(!dbg.contains("super-secret"), "secret 泄露: {dbg}");
    assert!(!dbg.contains("cached-token"), "token 泄露: {dbg}");
}

/// P1：[WecomBackend] 无 bot 凭据时 bot 为 None，token 缓存保持正常
/// 条件：构造 bot=None、token=Some("cached-token") 的 WecomBackend
/// 断言：bot 为 None；cached_token() == Some("cached-token")
#[test]
fn no_bot_credentials_token_cached() {
    let backend = WecomBackend::new(
        Arc::new(HttpTransportBackend::default()),
        None,
        Some("cached-token".into()),
        auth_ep(TEST_AUTH_ENDPOINT),
    );
    assert!(backend.bot.is_none());
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
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"result": "{\"ok\":true}"})))
        .expect(1)
        .mount(&server)
        .await;

    let transport = wrapped_transport(&server.uri(), None, Some("tok-x"), TEST_AUTH_ENDPOINT);
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

/// P0：[WecomBackend] 挂 RequireAuth + 无 token → Err(Error::Auth)，请求不发出
/// 条件：endpoint 挂 RequireAuth，无 token；mock expect(0)
/// 断言：invoke 返回 Err(Wrapped(OtherError(CliError::Auth)))，mock 未被调用
#[tokio::test]
async fn rejects_require_auth_without_token() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/auth"))
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&server)
        .await;

    let transport = wrapped_transport(&server.uri(), None, None, TEST_AUTH_ENDPOINT);
    let endpoint = ep(&server.uri(), "/auth").with(RequireAuth);
    let err = transport.invoke(&endpoint, json!({})).await.unwrap_err();
    match err {
        wecom_transport::Error::Wrapped(w) => {
            let inner = w
                .as_any()
                .downcast_ref::<wecom_error::OtherError>()
                .and_then(|o| o.0.downcast_ref::<CliError>());
            assert!(
                inner.is_some_and(|e| matches!(e, CliError::Auth(_))),
                "expected CliError::Auth, got {inner:?}"
            );
        }
        other => panic!("expected Wrapped(CliError::Auth), got {other:?}"),
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
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"result": "{\"ok\":true}"})))
        .expect(1)
        .mount(&server)
        .await;

    let transport = wrapped_transport(&server.uri(), None, Some("tok-x"), TEST_AUTH_ENDPOINT);
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
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"result": "{\"ok\":true}"})))
        .expect(1)
        .mount(&server)
        .await;

    let transport = wrapped_transport(&server.uri(), None, None, TEST_AUTH_ENDPOINT);
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

// ── NestedRes（网关扁平协议，由产品层定义）──────────────────

/// P0：[NestedRes] 扁平协议：errcode 校验 → results_json 脱壳
/// 条件：body 为 {errcode:0, results_json: "{result:...}"}
/// 断言：decode 返回 ApiResponse，result 为脱壳后的字符串
#[test]
fn results_json_res_decodes_flat_protocol() {
    let body = json!({
        "errcode": 0,
        "errmsg": "ok",
        "results_json": r#"{"result":"{\"ok\":true}"}"#,
    });
    let res = NestedRes
        .decode("https://api.example.com/x", body)
        .expect("flat protocol should decode");
    assert_eq!(res.result.as_deref(), Some(r#"{"ok":true}"#));
}

/// P0：[NestedRes] 扁平协议：errcode != 0 → Api 错误
/// 条件：body 为 {errcode: 40001, errmsg: "bad request"}
/// 断言：decode 返回 Err(Error::Api)，code=40001，message 为 errmsg
#[test]
fn results_json_res_err_code_is_api_error() {
    let body = json!({
        "errcode": 40001,
        "errmsg": "invalid credential",
        "results_json": r#"{"result":"{}"}"#,
    });
    let err = NestedRes
        .decode("https://api.example.com/x", body)
        .expect_err("errcode != 0 should be an error");
    match err {
        wecom_transport::Error::Api { code, message, .. } => {
            assert_eq!(code, Some(40001));
            assert_eq!(message, "invalid credential");
        }
        other => panic!("expected Error::Api, got {other:?}"),
    }
}

/// P1：[NestedRes] 扁平协议：缺少 results_json → 协议异常
/// 条件：body 仅含 errcode/errmsg
/// 断言：decode 返回 Err(Error::Parse)，message 含 missing `results_json`
#[test]
fn results_json_res_missing_results_json_is_parse_error() {
    let body = json!({ "errcode": 0, "errmsg": "ok" });
    let err = NestedRes
        .decode("https://api.example.com/x", body)
        .expect_err("missing results_json should be an error");
    assert!(
        matches!(err, wecom_transport::Error::Parse { .. }),
        "expected Error::Parse, got {err:?}"
    );
}

/// P1：[NestedRes] 扁平协议：results_json 内层 error.code 校验透传
/// 条件：results_json 内层为 {error:{code:40001}}
/// 断言：decode 返回 Err(Error::Api)，code=40001
#[test]
fn results_json_res_inner_api_error_is_validated() {
    let body = json!({
        "errcode": 0,
        "errmsg": "ok",
        "results_json": r#"{"result":null,"error":{"code":40001,"message":"inner err"}}"#,
    });
    let err = NestedRes
        .decode("https://api.example.com/x", body)
        .expect_err("inner error.code should be an error");
    match err {
        wecom_transport::Error::Api { code, .. } => assert_eq!(code, Some(40001)),
        other => panic!("expected Error::Api, got {other:?}"),
    }
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

    let transport = wrapped_transport(&server.uri(), None, Some("old-token"), TEST_AUTH_ENDPOINT);
    let endpoint = wecom_transport::Endpoint::new().with(
        wecom_transport::HttpEndpoint::new("/bootstrap")
            .with_service(server.uri())
            .with_res_envelope(FlatRes),
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
    // 隔离凭据目录：避免命中本机真实 credentials.enc 使双检直接复用磁盘 token。
    // 使用进程级共享锁，与 auth::credentials 等修改 WECOM_CLI_CONFIG_DIR 的测试互斥。
    let _guard = crate::env::TEST_ENV_LOCK.lock().await;
    let dir = tempfile::tempdir().unwrap();
    unsafe {
        std::env::set_var(crate::env::CONFIG_DIR, dir.path());
    }
    async {
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
            Some(auth::Bot::new("bot1".into(), "secret1".into())),
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
    .await;
    unsafe {
        std::env::remove_var(crate::env::CONFIG_DIR);
    }
}

// ── WECOM_CLI_ACCESS_TOKEN 覆盖 ─────────────────────────────

/// P0：[resolve_access_token] WECOM_CLI_ACCESS_TOKEN 覆盖 auth 提供的 token
/// 条件：设置 WECOM_CLI_ACCESS_TOKEN=env-tok，隔离凭据目录（auth::load_token 返回 None）
/// 断言：resolve_access_token() == Some("env-tok")
#[cfg(any(feature = "custom-endpoint", feature = "managed-auth"))]
#[tokio::test]
async fn access_token_env_overrides_auth_token() {
    let _guard = crate::env::TEST_ENV_LOCK.lock().await;
    let dir = tempfile::tempdir().unwrap();
    unsafe {
        std::env::set_var(crate::env::CONFIG_DIR, dir.path());
        std::env::set_var(crate::env::ACCESS_TOKEN, "env-tok");
    }
    let r = resolve_access_token();
    unsafe {
        std::env::remove_var(crate::env::ACCESS_TOKEN);
        std::env::remove_var(crate::env::CONFIG_DIR);
    }
    assert_eq!(r.as_deref(), Some("env-tok"));
}

/// P1：[resolve_access_token] 环境变量为空时回退 auth 提供的 token
/// 条件：WECOM_CLI_ACCESS_TOKEN=""，隔离凭据目录（无凭据 → load_token 返回 None）
/// 断言：resolve_access_token() == None（空环境变量不生效，走回退路径）
#[cfg(any(feature = "custom-endpoint", feature = "managed-auth"))]
#[tokio::test]
async fn access_token_env_empty_falls_back_to_auth() {
    let _guard = crate::env::TEST_ENV_LOCK.lock().await;
    let dir = tempfile::tempdir().unwrap();
    unsafe {
        std::env::set_var(crate::env::CONFIG_DIR, dir.path());
        std::env::set_var(crate::env::ACCESS_TOKEN, "");
    }
    let r = resolve_access_token();
    unsafe {
        std::env::remove_var(crate::env::ACCESS_TOKEN);
        std::env::remove_var(crate::env::CONFIG_DIR);
    }
    assert_eq!(r, None);
}
