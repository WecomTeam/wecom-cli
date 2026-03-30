# Skills 导航

仓库当前内置 12 个 Agent Skills，位于 `skills/` 目录下。这里负责给出分类和入口；每个 Skill 的具体工作流、参数示例和补充参考仍以各自的 `SKILL.md` 为准。

通用 CLI 行为以 [`docs/cli-reference.md`](cli-reference.md) 为准；如果某个 Skill 页面对“帮助行为”或“是否需要传 `'{}'`”没有展开说明，请优先遵循 CLI 参考页。

## 按领域查看

| Skill | 领域 | 用途 |
| --- | --- | --- |
| [`wecomcli-lookup-contact`](../skills/wecomcli-lookup-contact/SKILL.md) | `contact` | 获取可见成员列表，按姓名或别名匹配 |
| [`wecomcli-get-todo-list`](../skills/wecomcli-get-todo-list/SKILL.md) | `todo` | 查询待办列表与分页信息 |
| [`wecomcli-get-todo-detail`](../skills/wecomcli-get-todo-detail/SKILL.md) | `todo` | 批量获取待办完整详情 |
| [`wecomcli-edit-todo`](../skills/wecomcli-edit-todo/SKILL.md) | `todo` | 创建、更新、删除待办和变更状态 |
| [`wecomcli-create-meeting`](../skills/wecomcli-create-meeting/SKILL.md) | `meeting` | 创建预约会议 |
| [`wecomcli-edit-meeting`](../skills/wecomcli-edit-meeting/SKILL.md) | `meeting` | 取消会议和更新受邀成员 |
| [`wecomcli-get-meeting`](../skills/wecomcli-get-meeting/SKILL.md) | `meeting` | 查询会议列表和会议详情 |
| [`wecomcli-get-msg`](../skills/wecomcli-get-msg/SKILL.md) | `msg` | 查看聊天、拉取消息、下载媒体、发送文本 |
| [`wecomcli-manage-schedule`](../skills/wecomcli-manage-schedule/SKILL.md) | `schedule` | 管理日程、参与人和闲忙 |
| [`wecomcli-manage-doc`](../skills/wecomcli-manage-doc/SKILL.md) | `doc` | 创建、导出和编辑企业微信文档 |
| [`wecomcli-manage-smartsheet-schema`](../skills/wecomcli-manage-smartsheet-schema/SKILL.md) | `doc` | 管理智能表格的子表与字段结构 |
| [`wecomcli-manage-smartsheet-data`](../skills/wecomcli-manage-smartsheet-data/SKILL.md) | `doc` | 读取、写入、更新和删除智能表格记录 |

## 维护边界

- 需要跨 Skill 的通用规则时，优先写到 `docs/cli-reference.md`。
- 需要单个业务域的输入输出细节时，写到对应 Skill 目录。
- 需要补充接口示例、字段枚举或格式说明时，优先放到该 Skill 的 `references/` 子目录。
