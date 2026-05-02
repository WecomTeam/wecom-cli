# 卸载 wecom-cli

## 步骤 1：移除 CLI

```bash
npm uninstall -g @wecom/cli
```

## 步骤 2：移除 CLI Skill

```bash
npx skills remove WeComTeam/wecom-cli -g
```

如果提示 `skills` 命令不存在，先安装：

```bash
npm install -g @anthropic-ai/skills
```

## 步骤 3：清理本地配置和数据

```bash
# 删除配置目录（包含 Bot 凭证、MCP 配置、缓存）
rm -rf ~/.config/wecom/

# 删除临时媒体文件
rm -rf "${TMPDIR:-/tmp}/wecom/"
```

> 注意：WECOM_CLI_CONFIG_DIR 或 WECOM_CLI_TMP_DIR 环境变量会覆盖上述默认路径，请按照你实际配置的路径清理。
