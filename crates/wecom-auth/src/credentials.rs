//! 凭据存储抽象：[`CredentialStore`] trait 与内存实现。
//!
//! [`Credentials`] 为凭据总账：bot 信息与 Bearer token 共存，保证原子更新。
//! 服务端 / 容器部署可经 [`CredentialStore`] 接入 KMS、Vault 或共享 Secret Store。

use std::sync::{Arc, RwLock};

use serde::{Deserialize, Serialize};

use crate::bot::BotCredential;
use crate::error::AuthError;

/// 本地凭据总账：bot 信息与 Bearer token 共存，保证原子更新。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Credentials {
    pub bot: Option<BotCredential>,
    pub token: Option<String>,
}

/// 凭据存储抽象。
///
/// 实现方按自身介质决定持久化方式（加密文件 / 内存 / KMS / Redis 等）；
/// `save` 必须整体覆盖写入（bot 与 token 原子更新）。
pub trait CredentialStore: Send + Sync {
    /// 读取凭据；无凭据时返回 `None`。
    ///
    /// # Errors
    /// 存储介质故障（IO / 远端不可达）时返回 [`AuthError::Storage`]；
    /// 凭据缺失或解密失败不算错误（返回 `None`，与「未授权」表现一致）。
    fn load(&self) -> Result<Option<Credentials>, AuthError>;

    /// 整体覆盖写入凭据。
    ///
    /// # Errors
    /// 写入失败时返回 [`AuthError::Storage`]。
    fn save(&self, credentials: &Credentials) -> Result<(), AuthError>;

    /// 清除凭据（无凭据时应为 no-op）。
    ///
    /// # Errors
    /// 删除失败时返回 [`AuthError::Storage`]。
    fn clear(&self) -> Result<(), AuthError>;
}

/// 内存凭据存储（进程内共享）。
///
/// 适用于：单实例部署从环境变量 / 挂载 Secret 注入凭据，或测试。
#[derive(Debug, Clone, Default)]
pub struct MemoryCredentialStore {
    inner: Arc<RwLock<Option<Credentials>>>,
}

impl MemoryCredentialStore {
    /// 创建空存储；`initial` 非空时预置凭据。
    pub fn new(initial: Credentials) -> Self {
        Self {
            inner: Arc::new(RwLock::new(
                Some(initial).filter(|c| c.bot.is_some() || c.token.is_some()),
            )),
        }
    }

    /// 进程内共享实例。
    pub fn shared(self) -> Arc<Self> {
        Arc::new(self)
    }
}

impl CredentialStore for MemoryCredentialStore {
    fn load(&self) -> Result<Option<Credentials>, AuthError> {
        Ok(self.inner.read().unwrap_or_else(|e| e.into_inner()).clone())
    }

    fn save(&self, credentials: &Credentials) -> Result<(), AuthError> {
        let value = Some(credentials.clone()).filter(|c| c.bot.is_some() || c.token.is_some());
        *self.inner.write().unwrap_or_else(|e| e.into_inner()) = value;
        Ok(())
    }

    fn clear(&self) -> Result<(), AuthError> {
        *self.inner.write().unwrap_or_else(|e| e.into_inner()) = None;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    //! ## 模块摘要：credentials（凭据总账与存储抽象）
    //!
    //! ### 关键接口
    //! - [Credentials] — bot 信息与 Bearer token 共存的联合结构
    //! - [CredentialStore] — 凭据存储抽象（load / save / clear）
    //! - [MemoryCredentialStore] — 内存实现（进程内共享）
    //!
    //! ### 关键分支与异常路径
    //! - bot 与 token 均空 → save 等价于 clear
    //! - bot 与 token 可独立更新互不影响

    use super::*;

    fn bot(id: &str) -> BotCredential {
        BotCredential::new(id.to_string(), "secret".into())
    }

    /// P0：bot + token 保存后可完整读回
    #[test]
    fn memory_store_roundtrip() {
        let store = MemoryCredentialStore::new(Credentials::default());
        assert!(store.load().unwrap().is_none());

        let creds = Credentials {
            bot: Some(bot("bot1")),
            token: Some("tok-1".into()),
        };
        store.save(&creds).unwrap();
        let loaded = store.load().unwrap().unwrap();
        assert_eq!(loaded.bot.as_ref().map(|b| b.id.as_str()), Some("bot1"));
        assert_eq!(loaded.token.as_deref(), Some("tok-1"));
    }

    /// P0：bot 与 token 均空时 save 等价 clear
    #[test]
    fn memory_store_save_empty_clears() {
        let store = MemoryCredentialStore::new(Credentials {
            bot: Some(bot("bot1")),
            token: None,
        });
        store.save(&Credentials::default()).unwrap();
        assert!(store.load().unwrap().is_none());
    }

    /// P0：clear 清空凭据；空存储 clear 为 no-op
    #[test]
    fn memory_store_clear() {
        let store = MemoryCredentialStore::new(Credentials {
            bot: Some(bot("bot1")),
            token: Some("tok-1".into()),
        });
        store.clear().unwrap();
        assert!(store.load().unwrap().is_none());
        store.clear().unwrap();
    }
}
