# Skills 导航

仓库当前内置的 Agent Skills 位于 `skills/` 目录下。本文档提供分类、能力概览和入口；每个 Skill 的触发条件、完整工作流、参数示例与补充参考以各自的 `SKILL.md` 为准。

## Agent Skills

当前共内置 14 个 Agent Skills：

| Skill | 品类 | 说明 |
| ----- | ---- | ---- |
| [`wecomcli-shared`](../skills/wecomcli-shared/SKILL.md) | 公共前置检查 | 供所有 `wecomcli-*` 业务 Skill 共用；在执行 CLI 命令前检查安装、最低版本和授权状态，必要时完成安装、升级或初始化；提供获取机器人操作者和授权人身份工具的说明。 |
| [`wecomcli-contact`](../skills/wecomcli-contact/SKILL.md) | 通讯录 | 按姓名、拼音、英文名或别名搜索联系人，查询匹配人员的部门和职务等信息 |
| [`wecomcli-calendar`](../skills/wecomcli-calendar/SKILL.md) | 日程 | 创建、查看、搜索、更新和取消不含在线会议链接的日程，并查询参与人忙闲状态 |
| [`wecomcli-meeting`](../skills/wecomcli-meeting/SKILL.md) | 会议 | 管理含会议号或入会链接的在线会议，包括创建、查询、搜索、更新、取消，以及获取纪要、待办和转写原文 |
| [`wecomcli-todo`](../skills/wecomcli-todo/SKILL.md) | 待办 | 创建、查询、筛选、更新、完成、删除或退出待办，并管理参与人状态和截止时间 |
| [`wecomcli-email`](../skills/wecomcli-email/SKILL.md) | 邮件 | 搜索邮件并读取正文、附件和内嵌图片；仅支持浏览与查询，不支持发送、回复、转发、标记或删除 |
| [`wecomcli-disk`](../skills/wecomcli-disk/SKILL.md) | 微盘 | 列出、搜索、上传、下载和重命名微盘文件，读取文件元信息，以及新建文件夹 |
| [`wecomcli-media`](../skills/wecomcli-media/SKILL.md) | 媒体文件 | 在本地文件与 `media_id` 之间执行上传或下载；只负责文件搬运，不解析文件内容 |
| [`wecomcli-message`](../skills/wecomcli-message/SKILL.md) | 消息 | 先查询最近会话，再使用列表返回的会话 ID 发送 Markdown、图片、文件、语音或视频 |
| [`wecomcli-doc-manage`](../skills/wecomcli-doc-manage/SKILL.md) | 文档公共管理 | 搜索各类企业微信文档、查看最近浏览或创建的文档、修改名称、管理成员权限和文档加入规则 |
| [`wecomcli-doc`](../skills/wecomcli-doc/SKILL.md) | 在线文档 | 新建或导入明确指定为 doc、docx、Word 或在线文档的文件，并读取、追加、局部替换或覆盖正文内容 |
| [`wecomcli-sheet`](../skills/wecomcli-sheet/SKILL.md) | 在线表格 | 新建或导入在线表格，读取、修改和追加表格数据，以及管理子表 |
| [`wecomcli-smartsheet`](../skills/wecomcli-smartsheet/SKILL.md) | 智能表格 | 读取和管理智能表格的数据、结构与样式，包括子表、字段、记录、视图和图表 |
| [`wecomcli-smartpage`](../skills/wecomcli-smartpage/SKILL.md) | 智能文档 | 创建、导入、读取和修改智能文档，调整页面树结构，并获取内置智能表格信息；未指定类型的文档创建、写作或整理请求默认由该 Skill 承接 |
