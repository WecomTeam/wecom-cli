//! AI Bot CLI 网关协议：扁平响应信封与鉴权能力标记。
//!
//! 真实网关协议为顶层 `{errcode, errmsg, results_json}`，由 [`NestedRes`]
//! 完成 errcode 校验 + results_json 内层脱壳；鉴权引导等不套网关信封的接口
//! 使用 [`FlatRes`]——业务数据平铺在顶层，经 [`FlatApiResponse::extra`] 透传，
//! errcode 校验与 [`NestedRes`] 共用 [`validate_flat_api_response`]。
//!
//! 鉴权语义：
//! - [`RequireAuth`] 作为**门禁**标记挂在 [`Endpoint`](wecom_transport::Endpoint)
//!   能力袋上：挂载该标记的端点若无可用的 token，请求直接报
//!   [`AuthError::MissingCredentials`](crate::error::AuthError) 且不发出。
//! - [`SuppressAuth`] 作为**抑制注入**标记：携带该标记的端点（如换取 token
//!   的鉴权引导接口）即使持有 token 也不注入 `Authorization` 头。
//! - 默认行为（不挂任何标记）：只要持有 token 就注入
//!   `Authorization: Bearer <token>`，没有 token 则忽略（不报错）。

use indexmap::IndexMap;
use wecom_transport::{
    HttpEndpoint, ResponseEnvelope,
    backend::protocol::{ApiResponse, validate_api_response},
};

/// 端点调用前的 token 门禁标记（存在即生效）。
///
/// 挂进 [`Endpoint`](wecom_transport::Endpoint) 能力袋——鉴权门禁按 endpoint
/// 单独声明：挂载后调用前必须已有可用 token，无 token 时报
/// [`AuthError::MissingCredentials`](crate::error::AuthError)，请求不发出。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RequireAuth;

/// 抑制 `Authorization` 注入的标记（换取 token 的引导端点专用）。
///
/// 默认所有端点「有 token 就携带、无 token 则忽略」；仅鉴权引导等换取 token
/// 的接口挂此标记，保证引导请求绝不携带失效 token，避免 853004 刷新自死锁。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SuppressAuth;

/// 鉴权引导端点默认 URL（botid+secret 签名调用换取 Bearer token，product/正式环境）。
pub const DEFAULT_AUTH_ENDPOINT: &str =
    "https://qyapi.weixin.qq.com/cgi-bin/aibot/cli/get_cli_config";

/// 网关扁平协议响应体：顶层只有 `errcode` / `errmsg` / `results_json`。
///
/// `results_json` 为字符串，内层直接复用 [`ApiResponse`]。
#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct FlatApiResponse {
    pub errcode: Option<i64>,
    pub errmsg: Option<String>,
    pub results_json: Option<String>,
    #[serde(flatten)]
    pub extra: IndexMap<String, serde_json::Value>,
}

/// 网关扁平协议响应信封。
///
/// 顶层 `{errcode, errmsg, results_json}`：`errcode` 校验 →
/// `results_json` 脱壳为 [`ApiResponse`]（含 `error.code` 校验）。除鉴权引导
/// （[`FlatRes`]，扁平整体响应）外，所有端点均走此协议。
#[derive(Debug, Clone, Copy, Default)]
pub struct NestedRes;

impl ResponseEnvelope for NestedRes {
    fn decode(
        &self,
        url: &str,
        body: serde_json::Value,
    ) -> std::result::Result<ApiResponse, wecom_transport::Error> {
        // 扁平协议顶层解析 + errcode 校验。
        let flat: FlatApiResponse =
            serde_json::from_value(body).map_err(|e| wecom_transport::Error::Parse {
                message: format!("Parse FlatApiResponse failed for {url}: {e:#}"),
                endpoint: url.to_string(),
                body: Box::new(serde_json::Value::Null),
                source: Some(e),
            })?;
        let flat = validate_flat_api_response(url, flat)?;

        // 解析 results_json 内层（复用 ApiResponse，含 error.code 校验）。
        // 网关扁平响应必须携带 results_json，缺失视为协议异常。
        let results_json =
            flat.results_json
                .as_deref()
                .ok_or_else(|| wecom_transport::Error::Parse {
                    message: "API response missing `results_json` field".to_string(),
                    endpoint: url.to_string(),
                    body: Box::new(serde_json::to_value(&flat).unwrap_or_default()),
                    source: None,
                })?;

        let inner: ApiResponse =
            serde_json::from_str(results_json).map_err(|e| wecom_transport::Error::Parse {
                message: format!("Parse `results_json` JSON failed: {e:#}"),
                endpoint: url.to_string(),
                body: Box::new(serde_json::Value::String(results_json.to_string())),
                source: Some(e),
            })?;

        validate_api_response(url, inner)
    }

    fn name(&self) -> &'static str {
        "nested"
    }
}

/// 扁平响应信封（鉴权引导等「不套网关 `results_json` 信封」的接口使用）。
///
/// 与 [`NestedRes`] 同为网关扁平协议（[`FlatApiResponse`]），区别仅在
/// 业务数据的位置：`results_json` 字符串 vs 顶层平铺字段（`extra`）。
/// `errcode` 校验复用 [`validate_flat_api_response`]，`extra` 即业务结果。
/// 引导端点须显式挂 [`SuppressAuth`]
/// 抑制 Authorization 注入（换取 token 的请求不得携带 token）。
#[derive(Debug, Clone, Copy, Default)]
pub struct FlatRes;

impl ResponseEnvelope for FlatRes {
    fn decode(
        &self,
        url: &str,
        body: serde_json::Value,
    ) -> std::result::Result<ApiResponse, wecom_transport::Error> {
        let flat: FlatApiResponse =
            serde_json::from_value(body).map_err(|e| wecom_transport::Error::Parse {
                message: format!("Parse FlatApiResponse failed for {url}: {e:#}"),
                endpoint: url.to_string(),
                body: Box::new(serde_json::Value::Null),
                source: Some(e),
            })?;

        let flat = validate_flat_api_response(url, flat)?;

        // 业务数据平铺在顶层，经 extra 透传。
        Ok(ApiResponse {
            result: Some(serde_json::to_string(&flat.extra).unwrap_or_default()),
            error: None,
            taskid: None,
            poll_mode: None,
            long_task_poll: None,
            extra: Default::default(),
        })
    }

    fn name(&self) -> &'static str {
        "flat"
    }
}

/// 校验已反序列化的扁平响应（[`FlatApiResponse`]）的业务错误码。
///
/// `errcode != 0` → [`wecom_transport::Error::Api`]（errmsg 为消息）；缺失视为 0。
fn validate_flat_api_response(
    url: &str,
    data: FlatApiResponse,
) -> std::result::Result<FlatApiResponse, wecom_transport::Error> {
    let code = data.errcode.unwrap_or(0);
    if code != 0 {
        return Err(wecom_transport::Error::Api {
            message: data
                .errmsg
                .clone()
                .unwrap_or_else(|| "Unknown error".to_string()),
            action: url.to_string(),
            code: Some(code),
            body: Box::new(serde_json::to_value(&data).unwrap_or_default()),
        })
        .inspect_err(|e| tracing::error!(error = %e, "API error response"));
    }
    Ok(data)
}

/// 按 URL 装配鉴权引导端点（换取 Bearer token 的专用 Endpoint）——引导端点
/// 的唯一装配原语。
///
/// 使用 [`FlatRes`] 扁平响应信封（整体 JSON body 即业务结果
/// `{errcode, errmsg, token}`），并挂 [`SuppressAuth`] 抑制标记——即使持有
/// token 也不携带 Authorization 头（换取 token 的引导请求不得带失效 token，
/// 否则 853004 刷新会自死锁）。
pub fn auth_endpoint(url: &str) -> wecom_transport::Endpoint {
    wecom_transport::Endpoint::new()
        .with(HttpEndpoint::from_url(url).with_res_envelope(FlatRes))
        .with(SuppressAuth)
}

#[cfg(test)]
mod tests {
    //! ## 模块摘要：gateway（AI Bot CLI 网关协议：NestedRes / FlatRes / 鉴权标记）
    //!
    //! ### 关键接口
    //! - [FlatRes::decode] — 复用 [FlatApiResponse] 解析 +
    //!   [validate_flat_api_response] 校验 errcode；业务数据在顶层平铺字段（extra）中
    //!
    //! ### 关键分支与异常路径
    //! - errcode != 0 → `wecom_transport::Error::Api`（message 取后台 errmsg，
    //!   body 含 errcode/errmsg，保留后台 errcode）；errcode 缺失视为 0

    use serde_json::json;

    use super::*;

    /// P0：[FlatRes::decode] errcode=0 时顶层平铺字段（extra）作为 result 透传
    /// 条件：body 为 {"errcode":0,"errmsg":"ok","token":"t1"}
    /// 断言：decode 成功，result 为 {"token":"t1"} 的 JSON 字符串
    #[test]
    fn flat_res_returns_extra_on_success() {
        let body = json!({"errcode": 0, "errmsg": "ok", "token": "t1"});
        let resp = FlatRes.decode("/auth", body).unwrap();
        let result: serde_json::Value =
            serde_json::from_str(resp.result.as_deref().unwrap()).unwrap();
        assert_eq!(result, json!({"token": "t1"}));
    }

    /// P1：[FlatRes::decode] errcode 缺失视为 0
    /// 条件：body 无 errcode 字段
    /// 断言：decode 成功，result 为顶层平铺字段
    #[test]
    fn flat_res_missing_errcode_is_ok() {
        let resp = FlatRes.decode("/auth", json!({"token": "t1"})).unwrap();
        let result: serde_json::Value =
            serde_json::from_str(resp.result.as_deref().unwrap()).unwrap();
        assert_eq!(result, json!({"token": "t1"}));
    }

    /// P0：[FlatRes::decode] errcode!=0 → Api 错误取后台 errmsg
    /// 条件：body 为 {"errcode":853000,"errmsg":"invalid credential"}
    /// 断言：Api 错误 message=errmsg、code=853000、action=url、body 含 errcode/errmsg
    #[test]
    fn flat_res_errcode_is_api_error() {
        let err = FlatRes
            .decode(
                "/auth",
                json!({"errcode": 853000, "errmsg": "invalid credential"}),
            )
            .unwrap_err();
        match err {
            wecom_transport::Error::Api {
                message,
                action,
                code,
                body,
            } => {
                assert_eq!(message, "invalid credential");
                assert_eq!(action, "/auth");
                assert_eq!(code, Some(853000));
                assert_eq!(body["errcode"], json!(853000));
                assert_eq!(body["errmsg"], json!("invalid credential"));
            }
            other => panic!("expected Api error, got {other:?}"),
        }
    }

    /// P1：[FlatRes::decode] errmsg 缺失时回退默认文案，code 保留
    /// 条件：errcode=853004 且无 errmsg
    /// 断言：Api 错误 message 为 "Unknown error"，code=853004
    #[test]
    fn flat_res_missing_errmsg_falls_back_to_default() {
        let err = FlatRes
            .decode("/auth", json!({"errcode": 853004}))
            .unwrap_err();
        match err {
            wecom_transport::Error::Api { message, code, .. } => {
                assert_eq!(message, "Unknown error");
                assert_eq!(code, Some(853004));
            }
            other => panic!("expected Api error, got {other:?}"),
        }
    }
}
