# 沙箱路径：首页请求失败回退 `--output` 预留文件

- **场景**：`--output` 指定的文件在首页请求前就已预留（`create_file` 独占创建），首页请求失败后该文件必须被回退删除，不留 0 字节空壳
- **Transport**：HTTP（mock discovery + `hr department list` 首页 500）
- **Feature**：`custom-endpoint`
- **前置条件**：cwd=临时目录 A、`WECOM_CLI_CONFIG_DIR`=临时目录 B（预置加密凭据）、`WECOM_CLI_BASE_URL`=mock server；方法 mock 返回 500 且命中 1 次
- **命令与断言**：
  1. `wecom hr department list --id root --output ./out.json` → 非零退出，输出包含错误
  2. `cwd/out.json` 不存在（预留文件已回退，不留空壳）
- **断言 — HTTP Endpoint**：`POST /department/list` 恰好 1 次
- **断言 — FS**：`out.json` 未产生
- **关键上下文**：`crates/wecom/src/service/execute.rs` 在首页请求前预留文件（提前暴露写权限问题），失败且文件未保留时由单一回退点消费写端 `discard()` 撤回；保留点在首页内容写入成功之后
- **来源场景**：`--output` 预留文件生命周期（预留—保留—回退）的二进制级回归
