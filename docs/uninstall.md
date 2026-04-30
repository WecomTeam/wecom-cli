# 卸载 wecom-cli

想要彻底移除 wecom-cli，请分三步清理：CLI 包、Skill、以及本地配置/缓存。

## 1. 卸载 CLI 二进制

```
npm uninstall -g @wecom/cli
```

命令会移除全局的 `wecom-cli` 入口脚本，npm 会自动删除依赖的 `@wecom/cli-<platform>` 可选包。遇到安装失败请加上 `--force`。

## 2. 移除 Skill

安装时使用了 `npx skills add WeComTeam/wecom-cli -y -g`，卸载时执行：

```
npx skills remove WeComTeam/wecom-cli -g
```

如在 `skills` 命令中没有 `remove` 子命令，请先运行 `npx skills help` 确认。如果你在其他路径下各自安装了 Skill，请改为对应的 `skills remove <owner/repo>`。

## 3. 清理配置和密钥

wecom-cli 将配置和密钥保存至 `~/.config/wecom`（可通过 `WECOM_CLI_CONFIG_DIR` 指定自定义路径）。卸载时删除该目录：

```
rm -rf ~/.config/wecom
```

该目录内包含 `bot.enc`、`mcp_config.enc`、`.encryption_key` 等缓存，删除即可让后续重新安装重新初始化。定制过 `WECOM_CLI_CONFIG_DIR`、`WECOM_CLI_TMP_DIR` 或代理相关环境变量，也请同步清理。

## 额外清理（可选）

- 删除 `npm cache` 中残留：`npm cache clean --force`
- 删除 `skills` 相关缓存：`rm -rf ~/.skills/wecom-cli`
- 清理自定义临时目录：`rm -rf "$WECOM_CLI_TMP_DIR"`（默认为 `$(mktemp -d)/wecom`）

完成上述步骤即可还原到未安装状态。
