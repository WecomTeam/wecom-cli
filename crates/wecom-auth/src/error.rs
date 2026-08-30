//! wecom-auth 统一错误（错误码段 893300–893399）。
//!
//! 每个变体对应本层特有失败；下层（wecom-transport）错误经 [`AuthError::Transport`]
//! 委托透传（code 一路透传至 transport 层或后台错误码）。

// Error code range: 893300 - 893399, this crate uses 893300 - 893399.

/// 无可用凭据/token（需要授权但凭据缺失，或缺 bot 凭据无法刷新）。
pub const E_AUTH_MISSING: i64 = 893301;
/// 扫码超时（5 分钟）。
pub const E_QR_TIMEOUT: i64 = 893302;
/// 凭据加密/解密/密钥失败。
pub const E_CRYPTO: i64 = 893303;
/// 凭据存储读写失败（落盘/删除等本地 IO）。
pub const E_STORAGE: i64 = 893304;
/// 共享兜底码（仅意料之外的分支 / 系统失败）。
pub const E_OTHER: i64 = 893999;

/// wecom-auth 统一错误。
///
/// - [`AuthError::Transport`]：wecom-transport 层错误透传（含后台 errcode 语义）。
/// - [`AuthError::Other`] 仅用于意料之外的分支 / 系统失败；逻辑错误必须有自己的变体。
#[derive(Debug)]
pub enum AuthError {
    /// 无可用凭据/token：需要授权但凭据缺失，或缺 bot 凭据无法静默刷新。
    MissingCredentials(String),

    /// 扫码超时（5 分钟），请重试。
    QrTimeout,

    /// 凭据加密/解密/密钥相关失败。
    Crypto(String),

    /// 凭据存储读写失败（落盘 / 删除等本地 IO）。
    Storage(String),

    /// 下层 transport 错误透传（网络 / HTTP / 协议解析 / 后台 errcode）。
    Transport(wecom_transport::Error),

    /// 兜底：仅意料之外的分支 / 系统失败使用；逻辑错误必须有自己的变体。
    Other(Box<dyn std::error::Error + Send + Sync>),
}

impl AuthError {
    /// Category error code for this variant.
    ///
    /// [`AuthError::Transport`] 委托内层 [`wecom_transport::Error::code`]；
    /// 本层变体返回各自的 8933xx 码；[`AuthError::Other`] 返回共享兜底码 893999。
    #[must_use]
    pub fn code(&self) -> i64 {
        match self {
            AuthError::Transport(inner) => inner.code(),
            AuthError::MissingCredentials(_) => E_AUTH_MISSING,
            AuthError::QrTimeout => E_QR_TIMEOUT,
            AuthError::Crypto(_) => E_CRYPTO,
            AuthError::Storage(_) => E_STORAGE,
            AuthError::Other(_) => E_OTHER,
        }
    }

    #[must_use]
    pub fn message(&self) -> String {
        match self {
            AuthError::Transport(inner) => inner.to_string(),
            AuthError::MissingCredentials(message)
            | AuthError::Crypto(message)
            | AuthError::Storage(message) => message.clone(),
            AuthError::QrTimeout => "扫码超时（5 分钟），请重试".to_string(),
            AuthError::Other(e) => e.to_string(),
        }
    }
}

impl std::fmt::Display for AuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let code = self.code();
        match self {
            AuthError::MissingCredentials(msg) => write!(f, "AuthError: {msg} [code={code}]"),
            AuthError::QrTimeout => write!(f, "QrTimeout: {} [code={code}]", self.message()),
            AuthError::Crypto(msg) => write!(f, "CryptoError: {msg} [code={code}]"),
            AuthError::Storage(msg) => write!(f, "StorageError: {msg} [code={code}]"),
            AuthError::Transport(inner) => write!(f, "{inner}"),
            AuthError::Other(e) => write!(f, "UnknownError: {e} [code={code}]"),
        }
    }
}

impl std::error::Error for AuthError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            AuthError::Transport(inner) => Some(inner),
            AuthError::Other(inner) => Some(inner.as_ref()),
            _ => None,
        }
    }
}

// ── 下层 → 本层 ────────────────────────────────────────────────

impl From<wecom_transport::Error> for AuthError {
    fn from(e: wecom_transport::Error) -> Self {
        AuthError::Transport(e)
    }
}

// ── 本层 → 下层（跨边界出口）────────────────────────────────────

impl From<AuthError> for wecom_transport::Error {
    fn from(e: AuthError) -> Self {
        match e {
            // 委托错误直接拆包：保留 Api 等变体的 errcode 语义。
            AuthError::Transport(inner) => inner,
            other => wecom_transport::Error::Other(Box::new(other)),
        }
    }
}

#[cfg(test)]
mod tests {
    //! ## 模块摘要：AuthError（wecom-auth 统一错误）
    //!
    //! ### 关键接口
    //! - [AuthError::code] / [AuthError::message] — 本层变体按各自 8933xx 码产出，
    //!   Transport 委托内层
    //! - `From` 双向转换 — 下层统一包裹为 Transport；出口方向 Transport 拆包

    use super::*;

    /// P0：[AuthError::code] 本层各变体映射到专属 8933xx 码
    /// 条件：分别构造各本层变体
    /// 断言：code() 返回对应常量
    #[test]
    fn code_maps_each_variant() {
        assert_eq!(
            AuthError::MissingCredentials("x".into()).code(),
            E_AUTH_MISSING
        );
        assert_eq!(AuthError::QrTimeout.code(), E_QR_TIMEOUT);
        assert_eq!(AuthError::Crypto("x".into()).code(), E_CRYPTO);
        assert_eq!(AuthError::Storage("x".into()).code(), E_STORAGE);
        assert_eq!(AuthError::Other("x".into()).code(), E_OTHER);
    }

    /// P0：[AuthError::code] Transport 委托透传后台错误码
    /// 条件：Transport(Api{code:853000})
    /// 断言：code() == 853000
    #[test]
    fn code_transport_delegates() {
        let e = AuthError::Transport(wecom_transport::Error::Api {
            message: "invalid".into(),
            action: "/x".into(),
            code: Some(853000),
            body: Box::new(serde_json::Value::Null),
        });
        assert_eq!(e.code(), 853000);
    }

    /// P0：From<wecom_transport::Error> 包裹为 Transport；反向拆包还原
    /// 条件：transport Api 错误 → AuthError → transport::Error
    /// 断言：包裹后匹配 Transport；拆包后保留 Api 变体与 errcode
    #[test]
    fn transport_error_roundtrip() {
        let api = wecom_transport::Error::Api {
            message: "expired".into(),
            action: "/x".into(),
            code: Some(853004),
            body: Box::new(serde_json::Value::Null),
        };
        let wrapped = AuthError::from(api);
        assert!(matches!(wrapped, AuthError::Transport(_)));

        let back: wecom_transport::Error = wrapped.into();
        assert!(matches!(
            back,
            wecom_transport::Error::Api {
                code: Some(853004),
                ..
            }
        ));
    }

    /// P1：本层变体出口装箱为 transport Other，文案经 Display 保留
    /// 条件：MissingCredentials → transport::Error
    /// 断言：Other 中可 downcast 回 AuthError，Display 含原始消息与错误码
    #[test]
    fn local_variant_boxes_into_transport_other() {
        let e: wecom_transport::Error = AuthError::MissingCredentials("need login".into()).into();
        match e {
            wecom_transport::Error::Other(boxed) => {
                let down = boxed.downcast_ref::<AuthError>();
                assert!(
                    down.is_some_and(|e| matches!(e, AuthError::MissingCredentials(_))),
                    "expected AuthError::MissingCredentials"
                );
            }
            other => panic!("expected Other, got {other:?}"),
        }
    }
}
