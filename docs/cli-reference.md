# CLI 使用约定

这页只记录已经从仓库实现中核对过的 CLI 事实，避免把营销文案、Skill 工作流和运行时行为混在一起。

## 命令总览

顶层命令：

```bash
wecom-cli --help
```

当前内置命令与品类：

| 命令 | 说明 |
| --- | --- |
| `init` | 初始化机器人凭证，或刷新 MCP 配置 |
| `contact` | 通讯录成员查询和搜索 |
| `doc` | 文档和智能表格创建/管理 |
| `meeting` | 视频会议创建、管理和查询 |
| `msg` | 聊天列表、消息收发和媒体下载 |
| `schedule` | 日程增删改查和闲忙查询 |
| `todo` | 待办创建、查询和编辑 |

## 调用格式

通用格式：

```bash
wecom-cli <category> <method> '<json_args>'
```

例如：

```bash
wecom-cli contact get_userlist '{}'
wecom-cli meeting get_meeting_info '{"meetingid": "MEETING_ID"}'
wecom-cli doc create_doc '{"doc_type": 3, "doc_name": "项目周报"}'
```

## 已验证行为

| 场景 | 行为 |
| --- | --- |
| `wecom-cli --help` | 直接输出本地顶层帮助，不依赖初始化 |
| `wecom-cli init --help` | 直接输出 `init` 参数说明 |
| `wecom-cli <category>` | 远程拉取该品类的工具列表；如果尚未 `init`，会失败 |
| `wecom-cli <category> --help` | 与上面一样，会拉取远程工具列表，因此同样依赖已完成初始化 |
| `wecom-cli <category> <method>` | 输出该工具的 schema/help，不会真正调用 |
| `wecom-cli <category> <method> '<json_args>'` | 真正执行 JSON-RPC 工具调用 |
| `wecom-cli msg get_msg_media ...` | 会把媒体文件下载到本地临时目录，并在结果里返回 `local_path` |

补充说明：

- 分类工具列表和工具 schema 都来自 MCP 接口，因此“看帮助”本身也依赖凭证与网络。
- 普通工具调用默认超时为 30 秒；`get_msg_media` 使用 120 秒超时。

## 初始化与帮助建议

推荐顺序：

```bash
wecom-cli --help
wecom-cli init
wecom-cli contact --help
wecom-cli contact get_userlist '{}'
```

如果你只执行下面这条：

```bash
wecom-cli contact get_userlist
```

当前行为是显示 `get_userlist` 的输入 schema，而不是实际返回通讯录数据。

## 运行时路径

| 项目 | 默认位置 | 备注 |
| --- | --- | --- |
| 配置目录 | `~/.config/wecom` | 可由 `WECOM_CLI_CONFIG_DIR` 覆盖 |
| 机器人凭证 | `<config_dir>/bot.enc` | 初始化时创建 |
| MCP 配置缓存 | `<config_dir>/mcp_config.enc` | 初始化或刷新后更新 |
| 媒体临时目录 | `<system_tmp>/wecom/media` | 可由 `WECOM_CLI_TMP_DIR` 覆盖根目录 |

## 环境变量

| 变量 | 作用 |
| --- | --- |
| `WECOM_CLI_CONFIG_DIR` | 覆盖默认配置目录 |
| `WECOM_CLI_TMP_DIR` | 覆盖媒体临时目录的根目录 |
| `WECOM_CLI_LOG_LEVEL` | 打开 stderr 日志并设置过滤级别 |
| `WECOM_CLI_LOG_FILE` | 打开 JSON 日志输出，按天写入 `ww.log` |
| `WECOM_CLI_MCP_CONFIG_ENDPOINT` | 覆盖默认 MCP 配置接口地址 |

## 详细业务能力

具体参数结构、工作流示例和补充参考不再复制到这页，统一从 [`docs/skills.md`](skills.md) 进入各个 Skill 的 `SKILL.md`。
