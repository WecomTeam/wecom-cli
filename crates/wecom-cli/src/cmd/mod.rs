// skip-auth-check 构建屏蔽 auth 命令(无本地凭据流程)：模块整体不参与编译，注册点（main.rs）同步条件化。
#[cfg(not(feature = "skip-auth-check"))]
pub mod auth;
