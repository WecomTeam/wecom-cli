# 文档中心

本目录是 `wecom-cli` 的长期维护文档集，用来承载安装、使用约定、Skills 导航和开发说明。根 `README.md` 保持为项目首页，不再承担所有细节参考。

## 从这里开始

- 新用户安装和第一次调用：[`docs/getting-started.md`](getting-started.md)
- 查 CLI 行为、运行时路径和环境变量：[`docs/cli-reference.md`](cli-reference.md)
- 查 12 个内置 Skills 的分工和入口：[`docs/skills.md`](skills.md)
- 本地开发、调试和仓库结构：[`docs/development.md`](development.md)

## Source Of Truth

| 位置 | 负责内容 |
| --- | --- |
| `README.md` | 项目介绍、价值主张、最短上手路径 |
| `docs/getting-started.md` | 安装、初始化、首个成功调用 |
| `docs/cli-reference.md` | 已验证的 CLI 约定、帮助行为、运行时路径、环境变量 |
| `docs/skills.md` | Skills 索引、分类和跳转入口 |
| `skills/*/SKILL.md` | 单个 Skill 的工作流、参数示例、补充参考资料 |
| `packages/*/README.md` | 平台二进制包的最小安装说明 |

## 维护约定

- 优先把持续维护的说明写进 `docs/`，避免再次把根 `README.md` 写成长篇参考手册。
- 同一主题只保留一个主入口；其他页面通过链接复用，不复制大段相同内容。
- 写工具示例时，无入参工具也显式传 `'{}'`，避免和“显示 schema/help”行为混淆。
- 若命令、路径、环境变量无法从仓库确认，就不要写入文档。
