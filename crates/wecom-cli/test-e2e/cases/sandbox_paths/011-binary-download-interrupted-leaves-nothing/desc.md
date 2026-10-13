# 沙箱路径：二进制下载流中断不留半成品

- **场景**：二进制下载在流中途断开时，绝不保留截断文件——`--output` 分支由预留方（`execute_and_output` 单一回退点）删除，默认落盘分支由 `handle_binary_output` 本地删除
- **Transport**：HTTP（mock discovery + `hr department list` 返回 octet-stream，响应体经 chunked 流写入部分数据后中断）
- **Feature**：`custom-endpoint`
- **前置条件**：cwd=临时目录 A、`WECOM_CLI_CONFIG_DIR`=临时目录 B（预置加密凭据）、`WECOM_CLI_BASE_URL`=mock server；方法 mock 命中 2 次
- **命令与断言**：
  1. `wecom hr department list --id root --output ./trunc.bin` → 非零退出，输出包含错误；`cwd/trunc.bin` 不存在
  2. `wecom hr department list --id root`（默认落盘 cwd，文件名取自 Content-Disposition）→ 非零退出；`cwd/trunc.bin` 不存在
- **断言 — HTTP Endpoint**：`POST /department/list` 恰好 2 次
- **断言 — FS**：两次中断都不产生文件（无截断半成品）
- **关键上下文**：`crates/wecom/src/service/output.rs` 的 `finalize_download` 在流失败时返回 Err，未保留的预留文件由写端 `discard()` 撤回；无 `--output` 分支在本地 `create_file_unique` 后失败即本地撤回
- **来源场景**：二进制下载产物完整性（不完整即删）的二进制级回归
