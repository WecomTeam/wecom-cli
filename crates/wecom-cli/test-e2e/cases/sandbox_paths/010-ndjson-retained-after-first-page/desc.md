# 沙箱路径：分页 NDJSON 首页写入后失败仍保留

- **场景**：分页模式写 `--output` 时，首页 NDJSON 写入成功即视为有效部分结果；次页请求失败时文件保留（不因后续失败整份回退）
- **Transport**：HTTP（mock discovery + 两页 `hr department list`：首页 200 带 `has_more`/`next_cursor`，次页 500）
- **Feature**：`custom-endpoint`
- **前置条件**：cwd=临时目录 A、`WECOM_CLI_CONFIG_DIR`=临时目录 B（预置加密凭据）、`WECOM_CLI_BASE_URL`=mock server；服务方法请求体是 `{"payload":"<json字符串>"}` 封装——首页 mock 不带 body matcher，次页 mock 用 `Matcher::Regex("cursor_1")` 只匹配带游标的请求
- **命令与断言**：
  1. `wecom hr department list --id root --page-count 2 --output ./out.ndjson` → 非零退出（次页 500）
  2. `cwd/out.ndjson` 存在且仅含 1 行 NDJSON，首行 `departments[0].id` = "1"
- **断言 — HTTP Endpoint**：首页、次页 mock 各命中 1 次
- **断言 — FS**：首页 NDJSON 行保留在磁盘上
- **关键上下文**：`crates/wecom/src/service/execute.rs` 在首页 `write_page` 成功后调用 `OutputFileInfo::keep()`，拆除失败回退；次页失败时上层回退点因 `is_kept()` 为 true 而不删除文件
- **来源场景**：`--output` 预留文件生命周期（预留—保留—回退）的二进制级回归
