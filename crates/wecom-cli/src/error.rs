//! wecom-cli（bin 层）统一错误类型。
//!
//! 三层嵌套错误模型，每层只定义本层特有的错误变体，下层错误经委托变体逐层透传：
//!
//! ```text
//! wecom_cli::Error::Wecom(wecom::Error::Transport(wecom_transport::Error::Xxx))
//! ```
//!
//! 错误码段（总段 893000–893999）：
//! - wecom：893000–893099；wecom-transport：893100–893199；wecom-cli（本层）：893200–893299；
//! - wecom-auth（鉴权库）：893300–893399；
//! - 893999：各层共享兜底码（仅意料之外的分支 / 系统失败；逻辑错误必须有专属变体与错误码）。
//!
//! 跨边界转换：
//! - bin → wecom（扩展命令出口）/ bin → transport（[`WecomBackend`] 出口）：
//!   `Wecom` 委托变体拆包（不套娃），本层变体装箱为下层的 `Other`
//!   （Display 已携带完整文案与错误码，回程不做 downcast 还原）；
//! - wecom-auth 的 [`AuthError`](wecom_auth::AuthError) 按变体映射为本层
//!   对应变体（MissingCredentials → `Auth`、QrTimeout → `QrTimeout`、
//!   Crypto → `Crypto`、Transport → 拆包透传）。
//!
//! [`WecomBackend`]: wecom_runtime::WecomBackend

use serde_json::{Value, json};

// Error code range: 893000 - 893999, this crate uses 893200 - 893299.

/// Authorization error code (missing credentials/token, or no bot credentials
/// for silent token refresh).
pub const E_AUTH: i64 = 893201;
/// QR-code scan timeout error code.
pub const E_QR_TIMEOUT: i64 = 893202;
/// Credential crypto error code (encrypt/decrypt/key failures).
pub const E_CRYPTO: i64 = 893203;
/// Catch-all error code (unexpected branches / system failures only).
pub const E_OTHER: i64 = 893999;

/// wecom-cli（bin 层）统一错误。
///
/// - [`Error::Wecom`]：wecom 库层错误（含其透传的 transport 错误），访问器委托内层。
/// - 后台响应协议异常（缺字段/格式不符）不单独设变体，统一经
///   [`Error::protocol`] 构造为 transport 层 [`wecom_transport::Error::Parse`]。
/// - [`Error::Other`] 仅用于意料之外的分支 / 系统失败；逻辑错误必须有自己的变体。
#[derive(Debug)]
pub enum Error {
    /// wecom 库层错误（含其透传的 transport 错误）。
    Wecom(wecom::Error),

    /// 鉴权错误：需要授权但无可用凭据/token，或缺 bot 凭据无法静默刷新。
    Auth(String),

    /// 扫码超时（5 分钟），请重试。
    QrTimeout,

    /// 凭据加密/解密/密钥相关失败。
    Crypto(String),

    /// 兜底：仅意料之外的分支 / 系统失败使用；逻辑错误必须有自己的变体。
    Other(Box<dyn std::error::Error + Send + Sync>),
}

impl Error {
    /// 后台响应协议异常（缺字段/格式不符）→ transport 层
    /// [`wecom_transport::Error::Parse`]（code = E_PARSE 893103）。
    pub fn protocol(
        message: impl Into<String>,
        endpoint: impl Into<String>,
        body: serde_json::Value,
    ) -> Self {
        wecom_transport::Error::Parse {
            message: message.into(),
            endpoint: endpoint.into(),
            body: Box::new(body),
            source: None,
        }
        .into()
    }

    /// Category error code for this variant.
    ///
    /// [`Error::Wecom`] delegates to [`wecom::Error::code`]（一路透传至
    /// transport 层或后台错误码）；本层变体返回各自的 8932xx 码；
    /// [`Error::Other`] 返回共享兜底码 893999。
    #[must_use]
    pub fn code(&self) -> i64 {
        match self {
            Error::Wecom(inner) => inner.code(),
            Error::Auth(_) => E_AUTH,
            Error::QrTimeout => E_QR_TIMEOUT,
            Error::Crypto(_) => E_CRYPTO,
            Error::Other(_) => E_OTHER,
        }
    }

    #[must_use]
    pub fn message(&self) -> String {
        match self {
            Error::Wecom(inner) => inner.message(),
            Error::Auth(message) | Error::Crypto(message) => message.clone(),
            Error::QrTimeout => "扫码超时（5 分钟），请重试".to_string(),
            Error::Other(e) => e.to_string(),
        }
    }

    /// Convert this error into a structured JSON [`Value`].
    ///
    /// [`Error::Wecom`] delegates to [`wecom::Error::to_json`]；本层变体输出
    /// `{"error": {"type", "code", "message"}}` 结构。
    #[must_use]
    pub fn to_json(&self) -> Value {
        let ty = match self {
            Error::Wecom(inner) => return inner.to_json(),
            Error::Auth(_) => "AuthError",
            Error::QrTimeout => "QrTimeout",
            Error::Crypto(_) => "CryptoError",
            Error::Other(_) => "UnknownError",
        };
        json!({
            "error": {
                "type": ty,
                "code": self.code(),
                "message": self.message(),
            },
        })
    }

    /// Render the error as a ready-to-display string.
    ///
    /// [`Error::Wecom`] delegates to [`wecom::Error::render`]；本层变体输出
    /// pretty-printed JSON。
    #[must_use]
    pub fn render(&self) -> String {
        if let Error::Wecom(inner) = self {
            return inner.render();
        }
        serde_json::to_string_pretty(&self.to_json()).unwrap_or_else(|_| self.to_string())
    }

    /// Suggested process exit code.
    ///
    /// [`Error::Wecom`] delegates to [`wecom::Error::exit_code`]（如 `CliOutput`
    /// 的 0/2）；本层变体统一为 1。
    #[must_use]
    pub fn exit_code(&self) -> i32 {
        match self {
            Error::Wecom(inner) => inner.exit_code(),
            _ => 1,
        }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let code = self.code();
        match self {
            Error::Wecom(inner) => write!(f, "{inner}"),
            Error::Auth(msg) => write!(f, "AuthError: {msg} [code={code}]"),
            Error::QrTimeout => write!(f, "QrTimeout: {} [code={code}]", self.message()),
            Error::Crypto(msg) => write!(f, "CryptoError: {msg} [code={code}]"),
            Error::Other(e) => write!(f, "UnknownError: {e} [code={code}]"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Wecom(inner) => Some(inner),
            Error::Other(inner) => Some(inner.as_ref()),
            _ => None,
        }
    }
}

// ── 下层 → 本层 ────────────────────────────────────────────────

impl From<wecom::Error> for Error {
    /// 统一裹进 [`Error::Wecom`]，不做特判。
    fn from(e: wecom::Error) -> Self {
        Error::Wecom(e)
    }
}

impl From<wecom_transport::Error> for Error {
    fn from(e: wecom_transport::Error) -> Self {
        Error::Wecom(e.into())
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Wecom(e.into())
    }
}

impl From<wecom_auth::AuthError> for Error {
    /// wecom-auth 错误按变体映射为本层变体；Transport 委托错误拆包透传，
    /// 保留 Api 等变体的 errcode 语义。
    fn from(e: wecom_auth::AuthError) -> Self {
        match e {
            wecom_auth::AuthError::Transport(inner) => inner.into(),
            wecom_auth::AuthError::MissingCredentials(message) => Error::Auth(message),
            wecom_auth::AuthError::QrTimeout => Error::QrTimeout,
            wecom_auth::AuthError::Crypto(message) => Error::Crypto(message),
            wecom_auth::AuthError::Storage(message) => Error::Other(message.into()),
            wecom_auth::AuthError::Other(inner) => Error::Other(inner),
        }
    }
}

impl From<clap::Error> for Error {
    fn from(e: clap::Error) -> Self {
        // 扩展命令二次解析失败视为用法错误，复用 wecom 层 CliOutput（exit code 2）。
        Error::Wecom(wecom::Error::CliOutput {
            code: 2,
            message: e.to_string(),
            source: Some(e),
        })
    }
}

// ── 本层 → 下层（跨边界出口）────────────────────────────────────

impl From<Error> for wecom::Error {
    fn from(e: Error) -> Self {
        match e {
            // 已委托下层的错误直接拆包，避免 Other 套娃。
            Error::Wecom(inner) => inner,
            other => wecom::Error::Other(Box::new(other)),
        }
    }
}

impl From<Error> for wecom_transport::Error {
    fn from(e: Error) -> Self {
        match e {
            // Transport 委托错误直接拆包：保留 Api 等变体的 errcode 语义
            // （否则 Other 套娃后上层无法再匹配后台错误码）。
            Error::Wecom(wecom::Error::Transport(inner)) => inner,
            other => wecom_transport::Error::Other(Box::new(other)),
        }
    }
}

#[cfg(test)]
mod tests {
    //! ## 模块摘要：Error（wecom-cli bin 层统一错误类型）
    //!
    //! ### 关键接口
    //! - [Error::code] / [Error::message] / [Error::to_json] / [Error::render] /
    //!   [Error::exit_code] — Wecom 变体委托内层，本层变体按各自 8932xx 码产出
    //! - [Error::protocol] — 构造 transport 层 Parse 变体的协议异常
    //! - `From` 双向转换 — 下层统一包裹为 Wecom；出口方向 Wecom 委托变体拆包
    //!
    //! ### 关键分支与异常路径
    //! - 本层仅 Auth / QrTimeout / Crypto / Other 四个变体；协议异常复用 transport Parse
    //! - 本层变体跨边界装箱为下层 Other（Display 携带完整文案与错误码，不做还原）

    use assert_json_diff::assert_json_eq;

    use super::*;

    // ── code() / message() ──

    /// P0：[Error::code] 本层各变体映射到专属 8932xx 码
    /// 条件：分别构造 Auth / QrTimeout / Crypto / Other
    /// 断言：code() 返回对应常量
    #[test]
    fn code_maps_each_variant() {
        assert_eq!(Error::Auth("x".into()).code(), E_AUTH);
        assert_eq!(Error::QrTimeout.code(), E_QR_TIMEOUT);
        assert_eq!(Error::Crypto("x".into()).code(), E_CRYPTO);
        assert_eq!(Error::Other("x".into()).code(), E_OTHER);
    }

    /// P0：[Error::code] Wecom 变体委托内层 code
    /// 条件：构造 Wecom(wecom::Error::Validation)
    /// 断言：code() == 893001（wecom 层 E_VALIDATION）
    #[test]
    fn code_wecom_delegates() {
        let e = Error::Wecom(wecom::Error::Validation("x".into()));
        assert_eq!(e.code(), 893001);
    }

    /// P0：[Error::message] QrTimeout 返回固定指引文案
    /// 条件：构造 QrTimeout
    /// 断言：message 含「扫码超时」
    #[test]
    fn message_qr_timeout() {
        assert!(Error::QrTimeout.message().contains("扫码超时"));
    }

    // ── to_json() / render() ──

    /// P0：[Error::to_json] 本层变体输出 type/code/message 结构
    /// 条件：构造 Auth("need login")
    /// 断言：JSON 为 {"error":{"type":"AuthError","code":893201,"message":"need login"}}
    #[test]
    fn to_json_auth() {
        let e = Error::Auth("need login".into());
        assert_json_eq!(
            e.to_json(),
            json!({
                "error": {
                    "type": "AuthError",
                    "code": E_AUTH,
                    "message": "need login",
                }
            })
        );
    }

    /// P1：[Error::to_json] 各变体的 JSON type 命名
    /// 条件：分别构造各变体
    /// 断言：type 字段符合约定命名
    #[test]
    fn to_json_type_names() {
        let cases: Vec<(Error, &str)> = vec![
            (Error::Auth("x".into()), "AuthError"),
            (Error::QrTimeout, "QrTimeout"),
            (Error::Crypto("x".into()), "CryptoError"),
            (Error::Other("x".into()), "UnknownError"),
        ];
        for (e, ty) in cases {
            assert_eq!(e.to_json()["error"]["type"], json!(ty), "type for {e:?}");
        }
    }

    /// P0：[Error::render] Wecom 变体委托内层 render
    /// 条件：构造 Wecom(wecom::Error::Validation("field required"))
    /// 断言：render 输出与内层 render 一致（含 ValidationError）
    #[test]
    fn render_wecom_delegates() {
        let inner = wecom::Error::Validation("field required".into());
        let expected = inner.render();
        let e = Error::Wecom(inner);
        assert_eq!(e.render(), expected);
    }

    /// P1：[Error::render] 本层变体输出可解析 JSON
    /// 条件：构造 Crypto("bad key")
    /// 断言：render 反序列化后 type/code/message 正确
    #[test]
    fn render_crypto() {
        let e = Error::Crypto("bad key".into());
        let v: Value = serde_json::from_str(&e.render()).unwrap();
        assert_json_eq!(
            v,
            json!({
                "error": {
                    "type": "CryptoError",
                    "code": E_CRYPTO,
                    "message": "bad key",
                }
            })
        );
    }

    // ── exit_code() ──

    /// P0：[Error::exit_code] Wecom 委托内层；本层变体为 1
    /// 条件：Wecom(CliOutput code=2) 与 QrTimeout
    /// 断言：exit_code 分别为 2 与 1
    #[test]
    fn exit_code_delegates_wecom() {
        let e = Error::Wecom(wecom::Error::CliOutput {
            code: 2,
            message: "usage".into(),
            source: None,
        });
        assert_eq!(e.exit_code(), 2);
        assert_eq!(Error::QrTimeout.exit_code(), 1);
    }

    // ── Display ──

    /// P0：[Error::Display] 本层变体含类型名、消息与错误码
    /// 条件：构造 Crypto("bad ciphertext")
    /// 断言：Display 含 "CryptoError"、"bad ciphertext"、code=893203
    #[test]
    fn display_crypto() {
        let e = Error::Crypto("bad ciphertext".into());
        let s = format!("{e}");
        assert!(s.contains("CryptoError"));
        assert!(s.contains("bad ciphertext"));
        assert!(s.contains("code=893203"));
    }

    /// P1：[Error::Display] Wecom 变体委托内层 Display
    /// 条件：构造 Wecom(wecom::Error::Config("bad cfg"))
    /// 断言：Display 含 "ConfigError" 与 "bad cfg"
    #[test]
    fn display_wecom_delegates() {
        let e = Error::Wecom(wecom::Error::Config("bad cfg".into()));
        let s = format!("{e}");
        assert!(s.contains("ConfigError"));
        assert!(s.contains("bad cfg"));
    }

    // ── protocol() ──

    /// P0：[Error::protocol] 构造为三层嵌套的 transport Parse 变体
    /// 条件：Error::protocol("missing token", "/auth", Value::Null)
    /// 断言：匹配 Wecom(Transport(Parse))，code == E_PARSE(893103)
    #[test]
    fn protocol_constructs_transport_parse() {
        let e = Error::protocol("missing token", "/auth", Value::Null);
        assert!(matches!(
            e,
            Error::Wecom(wecom::Error::Transport(
                wecom_transport::Error::Parse { .. }
            ))
        ));
        assert_eq!(e.code(), 893103);
        assert_eq!(e.message(), "missing token");
    }

    // ── From：下层 → 本层 ──

    /// P0：[From<wecom::Error>] 统一包裹为 Wecom，不做特判
    /// 条件：wecom::Error::Other("plain")
    /// 断言：直接匹配 Wecom(Other)，code 为共享兜底 893999
    #[test]
    fn from_wecom_wraps_uniformly() {
        let e = Error::from(wecom::Error::Other("plain".into()));
        assert!(matches!(e, Error::Wecom(wecom::Error::Other(_))));
        assert_eq!(e.code(), E_OTHER);
    }

    /// P1：[From<wecom_transport::Error>] 转换为 Wecom(Transport) 嵌套
    /// 条件：wecom_transport::Error::Http
    /// 断言：三层嵌套 Wecom(Transport(Http))，code 委托 E_HTTP(893102)
    #[test]
    fn from_transport_error_nests() {
        let e = Error::from(wecom_transport::Error::Http {
            message: "not found".into(),
            endpoint: "/x".into(),
            status: 404,
        });
        assert!(matches!(
            e,
            Error::Wecom(wecom::Error::Transport(wecom_transport::Error::Http { .. }))
        ));
        assert_eq!(e.code(), 893102);
    }

    /// P1：[From<std::io::Error>] 转换为 Wecom(Io)
    /// 条件：io::Error NotFound
    /// 断言：匹配 Wecom(Io)，code 为 wecom 层 E_IO(893003)
    #[test]
    fn from_io_error_nests_wecom_io() {
        let e = Error::from(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "no such file",
        ));
        assert!(matches!(e, Error::Wecom(wecom::Error::Io { .. })));
        assert_eq!(e.code(), 893003);
    }

    /// P1：[From<clap::Error>] 转换为 Wecom(CliOutput)，exit code 2
    /// 条件：clap::Error::raw(InvalidValue)
    /// 断言：匹配 Wecom(CliOutput)，exit_code() == 2
    #[test]
    fn from_clap_error_becomes_cli_output() {
        let e = Error::from(clap::Error::raw(
            clap::error::ErrorKind::InvalidValue,
            "bad value",
        ));
        assert!(matches!(e, Error::Wecom(wecom::Error::CliOutput { .. })));
        assert_eq!(e.exit_code(), 2);
    }

    // ── From：本层 → 下层（跨边界出口）──

    /// P0：[From<Error> for wecom::Error] Wecom 变体拆包不套娃
    /// 条件：Error::Wecom(Validation)
    /// 断言：直接得到 wecom::Error::Validation（非 Other 装箱）
    #[test]
    fn into_wecom_unwraps_wecom_variant() {
        let e: wecom::Error = Error::Wecom(wecom::Error::Validation("x".into())).into();
        assert!(matches!(e, wecom::Error::Validation(_)));
    }

    /// P0：[From<Error> for wecom::Error] 本层变体装箱为 Other
    /// 条件：Error::Auth → wecom::Error
    /// 断言：装箱为 Other，且可 downcast 回 bin 层错误
    #[test]
    fn into_wecom_boxes_cli_variant() {
        let e: wecom::Error = Error::Auth("need login".into()).into();
        match e {
            wecom::Error::Other(boxed) => {
                assert!(
                    boxed
                        .downcast_ref::<Error>()
                        .is_some_and(|e| { matches!(e, Error::Auth(_)) })
                );
            }
            other => panic!("expected Other, got {other:?}"),
        }
    }

    /// P0：[From<Error> for wecom_transport::Error] Wecom(Transport) 拆包保留 Api 语义
    /// 条件：Error::Wecom(Transport(Api{code:853004}))
    /// 断言：直接得到 transport Api 变体，errcode 可匹配
    #[test]
    fn into_transport_unwraps_transport_variant() {
        let e: wecom_transport::Error =
            Error::Wecom(wecom::Error::Transport(wecom_transport::Error::Api {
                message: "expired".into(),
                action: "/x".into(),
                code: Some(853004),
                body: Box::new(Value::Null),
            }))
            .into();
        assert!(matches!(
            e,
            wecom_transport::Error::Api {
                code: Some(853004),
                ..
            }
        ));
    }

    /// P1：[From<Error> for wecom_transport::Error] 本层变体装箱为 Other
    /// 条件：Error::Crypto → wecom_transport::Error
    /// 断言：Other 中可 downcast 回 bin 层错误
    #[test]
    fn into_transport_boxes_cli_variant() {
        let e: wecom_transport::Error = Error::Crypto("x".into()).into();
        match e {
            wecom_transport::Error::Other(boxed) => {
                assert!(
                    boxed
                        .downcast_ref::<Error>()
                        .is_some_and(|e| { matches!(e, Error::Crypto(_)) })
                );
            }
            other => panic!("expected Other, got {other:?}"),
        }
    }

    // ── 跨边界装箱行为 ──

    /// P1：本层变体经扩展命令出口装箱后，文案经 Display 保留（不做还原）
    /// 条件：Error::Auth → wecom::Error（Other 装箱）→ Error（Wecom 包裹）
    /// 断言：code 为共享兜底 893999，message 为内层 Display（含原始错误码文本）
    #[test]
    fn boxed_cli_error_keeps_display_message() {
        let wecom_err: wecom::Error = Error::Auth("need login".into()).into();
        let back = Error::from(wecom_err);
        assert!(matches!(back, Error::Wecom(wecom::Error::Other(_))));
        assert_eq!(back.code(), E_OTHER);
        let msg = back.message();
        assert!(msg.contains("AuthError"));
        assert!(msg.contains("need login"));
    }

    /// P1：[Error::code] Wecom 委托透传后台错误码
    /// 条件：Wecom(Transport(Api{code:40001}))
    /// 断言：code() 透传后台错误码 40001，to_json 为原始 body
    #[test]
    fn wecom_transport_api_code_delegates() {
        let body = json!({"errcode": 40001, "errmsg": "invalid credential"});
        let e = Error::Wecom(wecom::Error::Transport(wecom_transport::Error::Api {
            message: "invalid credential".into(),
            action: "/x".into(),
            code: Some(40001),
            body: Box::new(body.clone()),
        }));
        assert_eq!(e.code(), 40001);
        assert_eq!(e.to_json(), body);
    }

    /// P1：[std::error::Error::source] Wecom / Other 暴露内层 source
    /// 条件：构造 Wecom(Validation) 与 Other(io)
    /// 断言：source() 均非 None；QrTimeout 为 None
    #[test]
    fn source_chain() {
        use std::error::Error as _;
        let e = Error::Wecom(wecom::Error::Validation("x".into()));
        assert!(e.source().is_some());
        let e = Error::Other(std::io::Error::other("io").into());
        assert!(e.source().is_some());
        assert!(Error::QrTimeout.source().is_none());
    }
}
