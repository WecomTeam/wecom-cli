//! 扫码登录的网络流程：创建二维码会话 → 轮询扫码结果。
//!
//! 本模块不含任何终端/浏览器表现层逻辑；渲染二维码、打开浏览器、进度提示
//! 由调用方（如 wecom-cli）在拿到 [`QrSession`] 后自行处理。

use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::bot::BotCredential;
use crate::error::AuthError;

const SOURCE: &str = "wecom_cli_external";
const QR_GENERATE_URL: &str = "https://work.weixin.qq.com/ai/qc/generate";
const QR_QUERY_URL: &str = "https://work.weixin.qq.com/ai/qc/query_result";
const QR_CODE_PAGE: &str = "https://work.weixin.qq.com/ai/qc/gen";

/// 轮询间隔 3 秒
const POLL_INTERVAL: Duration = Duration::from_secs(3);
/// 超时 5 分钟
const POLL_TIMEOUT: Duration = Duration::from_secs(300);

/// 一次扫码登录会话。
#[derive(Debug, Clone)]
pub struct QrSession {
    /// 轮询凭据（服务端会话码）。
    pub scode: String,
    /// 二维码承载的 URL（供调用方渲染成二维码）。
    pub auth_url: String,
    /// 二维码页面链接（供调用方在浏览器中打开 / 展示）。
    pub page_url: String,
}

/// 扫码直连请求的网络/解码失败 → transport 层 Network 错误（复用 E_NETWORK 语义与诊断链）。
fn qr_network_error(message: &str, endpoint: &str, source: reqwest::Error) -> AuthError {
    wecom_transport::Error::Network {
        message: message.to_string(),
        endpoint: endpoint.to_string(),
        source,
    }
    .into()
}

impl QrSession {
    /// 创建扫码会话：向服务端申请二维码。
    pub async fn create() -> Result<Self, AuthError> {
        let url = format!(
            "{}?source={}&plat={}",
            QR_GENERATE_URL,
            SOURCE,
            get_plat_code()
        );

        let response: GenerateResponse = reqwest::Client::new()
            .get(&url)
            .send()
            .await
            .map_err(|e| qr_network_error("获取二维码失败", &url, e))?
            .json()
            .await
            .map_err(|e| qr_network_error("获取二维码失败，响应格式异常", &url, e))?;

        let Some(data) = response.data else {
            return Err(protocol_error(
                "获取二维码失败，响应格式异常",
                &url,
                serde_json::to_value(response).unwrap_or_default(),
            ));
        };

        let (Some(scode), Some(auth_url)) = (&data.scode, &data.auth_url) else {
            return Err(protocol_error(
                "获取二维码失败，响应格式异常",
                &url,
                serde_json::to_value(data).unwrap_or_default(),
            ));
        };

        let page_url = format!("{}?source={}&scode={}", QR_CODE_PAGE, SOURCE, scode);
        tracing::debug!("qr session created");
        Ok(Self {
            scode: scode.to_string(),
            auth_url: auth_url.to_string(),
            page_url,
        })
    }

    /// 轮询扫码结果，直到用户扫码成功或超时（5 分钟）。
    pub async fn poll(&self) -> Result<BotCredential, AuthError> {
        let url = format!("{}?scode={}", QR_QUERY_URL, self.scode);
        let client = reqwest::Client::new();

        let start = std::time::Instant::now();
        tracing::debug!(
            timeout_secs = POLL_TIMEOUT.as_secs(),
            "polling qr scan status"
        );

        loop {
            if start.elapsed() >= POLL_TIMEOUT {
                tracing::debug!("qr scan polling timed out");
                return Err(AuthError::QrTimeout);
            }

            let response: QueryResponse = client
                .get(&url)
                .send()
                .await
                .map_err(|e| qr_network_error("查询扫码结果失败", &url, e))?
                .json()
                .await
                .map_err(|e| qr_network_error("查询扫码结果失败，响应格式异常", &url, e))?;

            if let Some(data) = &response.data
                && data.status.as_deref() == Some("success")
            {
                let Some(bot_info) = &data.bot_info else {
                    return Err(protocol_error(
                        "扫码成功但未获取到 Bot 信息",
                        &url,
                        serde_json::Value::Null,
                    ));
                };
                let (Some(botid), Some(secret)) = (&bot_info.botid, &bot_info.secret) else {
                    return Err(protocol_error(
                        "扫码成功但未获取到 Bot 信息",
                        &url,
                        serde_json::Value::Null,
                    ));
                };

                tracing::info!("qr code scanned, bot bound");
                return Ok(BotCredential::new(botid.to_string(), secret.to_string()));
            }

            tokio::time::sleep(POLL_INTERVAL).await;
        }
    }
}

/// 后台响应协议异常（缺字段/格式不符）→ transport 层
/// [`wecom_transport::Error::Parse`]。
fn protocol_error(message: &str, endpoint: &str, body: serde_json::Value) -> AuthError {
    wecom_transport::Error::Parse {
        message: message.to_string(),
        endpoint: endpoint.to_string(),
        body: Box::new(body),
        source: None,
    }
    .into()
}

fn get_plat_code() -> u8 {
    if cfg!(target_os = "macos") {
        1
    } else if cfg!(target_os = "windows") {
        2
    } else if cfg!(target_os = "linux") {
        3
    } else {
        0
    }
}

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize)]
struct GenerateResponse {
    data: Option<GenerateData>,
}

#[derive(Serialize, Deserialize)]
struct GenerateData {
    scode: Option<String>,
    auth_url: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct QueryResponse {
    data: Option<QueryData>,
}

#[derive(Serialize, Deserialize)]
struct QueryData {
    status: Option<String>,
    bot_info: Option<BotInfoPayload>,
}

#[derive(Serialize, Deserialize)]
struct BotInfoPayload {
    botid: Option<String>,
    secret: Option<String>,
}
