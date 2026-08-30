//! [`WecomClient`]：面向第三方应用的门面客户端。
//!
//! 组合 [`wecom::Client`]（动态服务发现 + schema 驱动调用 + 长任务轮询）
//! 与带鉴权的 [`Transport`](wecom_transport::Transport)，第三方应用无需
//! 关心端点目录、网关协议与 token 注入细节。

use wecom_transport::Transport;

/// WeCom 客户端：认证、服务发现与方法调用的统一入口。
///
/// 经 [`WecomClientBuilder`](crate::WecomClientBuilder) 构建后即可直接使用：
///
/// ```ignore
/// let client = WecomClientBuilder::new()
///     .token_provider(Arc::new(provider))
///     .build()
///     .await?;
///
/// // 程序化调用（服务/方法名由 discovery 下发）
/// let svc = client.service("hr").await?;
/// let result = svc.method(&["users", "list"])?.invoke(json!({})).await?;
/// ```
pub struct WecomClient {
    inner: wecom::Client,
}

impl WecomClient {
    /// 用已构建的 transport 组装客户端（自动挂网关协议端点目录）。
    ///
    /// 适合需要完全自定义 transport 的调用方；常规路径请用
    /// [`WecomClientBuilder`](crate::WecomClientBuilder)。
    ///
    /// # Errors
    ///
    /// [`wecom::Client`] 构建失败时返回 [`wecom::Error`]。
    pub fn from_transport(transport: Transport) -> Result<Self, wecom::Error> {
        Ok(Self {
            inner: wecom::Client::builder()
                .transport(transport)
                .endpoint_catalog(crate::endpoint_catalog())
                .build()?,
        })
    }

    /// 包裹一个已配置好的 [`wecom::Client`]（不改动其任何配置）。
    #[must_use]
    pub fn from_client(inner: wecom::Client) -> Self {
        Self { inner }
    }

    /// 底层 [`wecom::Client`]（需要沙箱 FS、helper、扩展命令等高级配置时使用）。
    #[must_use]
    pub fn inner(&self) -> &wecom::Client {
        &self.inner
    }

    /// 消耗自身，返回底层 [`wecom::Client`]。
    #[must_use]
    pub fn into_inner(self) -> wecom::Client {
        self.inner
    }

    /// 带鉴权的 transport（可注入默认 header、extension 等）。
    #[must_use]
    pub fn transport(&self) -> &Transport {
        self.inner.transport()
    }

    /// 列出 discovery 下发的服务目录。
    ///
    /// # Errors
    ///
    /// 服务发现失败时返回 [`wecom::Error`]。
    pub async fn list_services(&self) -> Result<Vec<wecom::ServiceInfo>, wecom::Error> {
        self.inner.list_services().await
    }

    /// 获取指定服务的句柄（服务名由 discovery 下发）。
    ///
    /// # Errors
    ///
    /// 服务不存在或发现失败时返回 [`wecom::Error`]。
    pub async fn service(&self, name: &str) -> Result<wecom::ServiceHandle<'_>, wecom::Error> {
        self.inner.service(name).await
    }

    /// 按路径（服务名 + 资源段 + 方法名）直接获取方法句柄。
    ///
    /// # Errors
    ///
    /// 方法不存在或发现失败时返回 [`wecom::Error`]。
    pub async fn method(&self, path: &[&str]) -> Result<wecom::MethodHandle<'_>, wecom::Error> {
        self.inner.method(path).await
    }

    /// CLI 风格的 argv 调度入口（与 `wecom-cli` 同一套命令模型）。
    ///
    /// 返回 [`wecom::CliRun`]（`IntoFuture`），`.await` 即执行：
    ///
    /// ```ignore
    /// client.run(vec!["wecom".into(), "hr".into(), "users".into(), "list".into()])
    ///     .await?;
    /// ```
    #[must_use]
    pub fn run(&self, argv: Vec<String>) -> wecom::CliRun<'_> {
        self.inner.run(argv)
    }
}
