//! 内置 endpoint 目录覆写（协议差异由产品层定义）。

use wecom::{EndpointCatalog, EndpointKey, PayloadStringReq};
use wecom_auth::{NestedRes, RequireAuth};
use wecom_transport::HttpEndpoint;

/// 内置 endpoint 配置目录：在 wecom crate 内建默认之上，为全部内置
/// endpoint 挂上网关扁平协议响应信封（[`NestedRes`]），并为媒体上传 /
/// schema 方法挂上 [`RequireAuth`] **门禁**（`ServiceDiscovery` 免门禁——
/// 未登录时也需可引导）。Authorization 注入不依赖本能力：持有 token 即注入。
///
/// 协议差异由产品层定义：网关扁平协议实现位于 [`wecom_auth::gateway`]。
pub fn endpoint_catalog() -> EndpointCatalog {
    EndpointCatalog::default().map_all(|key, ep| {
        let ep = ep.map::<HttpEndpoint>(|h| {
            h.with_req_envelope(PayloadStringReq)
                .with_res_envelope(NestedRes)
        });
        if key == EndpointKey::ServiceDiscovery {
            ep
        } else {
            ep.with(RequireAuth)
        }
    })
}
