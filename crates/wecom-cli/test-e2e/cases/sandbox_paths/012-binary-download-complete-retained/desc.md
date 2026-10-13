# 沙箱路径：完整二进制下载落盘保留

- **场景**：二进制下载完整结束时，`--output` 预留文件被标记保留并落盘，大小与响应体一致；回退只在失败路径触发
- **Transport**：HTTP（mock discovery + `hr department list` 返回 4 KiB octet-stream + Content-Disposition）
- **Feature**：`custom-endpoint`
- **前置条件**：cwd=临时目录 A、`WECOM_CLI_CONFIG_DIR`=临时目录 B（预置加密凭据）、`WECOM_CLI_BASE_URL`=mock server；方法 mock 命中 1 次
- **命令与断言**：
  1. `wecom hr department list --id root --output ./saved.bin` → 退出码 0
  2. stdout 的下载结果：`content_type` = "application/octet-stream"、`size` = 4096
  3. `cwd/saved.bin` 内容与响应体逐字节一致
- **断言 — HTTP Endpoint**：`POST /department/list` 恰好 1 次
- **断言 — FS**：`saved.bin` 存在、大小为 4096 字节
- **关键上下文**：`crates/wecom/src/service/output.rs` 的 `finalize_download` 在流结束并 `sync_all` 后调用 `OutputFileInfo::keep()` 拆除回退
- **来源场景**：二进制下载产物完整性（完成即保留）的二进制级回归
