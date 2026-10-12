# PPT 编辑：命令索引

> 参数**必须用 `--schema` 现查**，不得凭记忆拼参：
> ```bash
> wecom-cli slide pages addjsx --schema
> ```

命令形态为 `wecom-cli slide <资源> <动作>`；坐标单位磅(pt)；页码 / 形状 / 表格行列索引全部 0-based；插入形状须设 fill / border 颜色。文档标识统一用 `docid`。调用规约见 [pptx-api.md](pptx-api.md)。

> 🚨 **`wecom-cli slide designs update` 是写类前置门**：设计契约未持久化时，下表所有写命令一律被服务端拒绝。

> 🚨 **没有原生撤销**：改坏了只能整页重做（`slide pages delete` + `slide pages addjsx`），所以生成阶段保留的每页 JSX 源码副本是唯一回退依据。

> 🚨 **对非本 session 生成、本地无 JSX 源码副本的已有页面，禁止用 `slide pages delete` + `slide pages addjsx` 整页重做**——原页内容不可恢复。此类文档的编辑仅限 `--schema` 探测到的原位编辑命令；探测不到就告知用户当前版本不支持该编辑，不做破坏性兜底。

## skill 固化的编辑命令

本文件只固化以下 **5 条**编辑命令。**固化的是命令名，不是入参**——调用前同样先 `wecom-cli slide <命令> --schema` 现查参数再传，只读的 `slide get` / `slide pages get` 也不例外：

| 命令 | 说明 |
|---|---|
| `slide pages add` | 在演示文稿中插入一张新幻灯片 |
| `slide pages delete` | 删除指定位置的幻灯片 |
| `slide pages addjsx` | 通过 JSX 代码生成一页幻灯片并插入到指定位置 |
| `slide get` | 获取演示文稿元数据：幻灯片总数、有序 slide_ids、幻灯片尺寸（「列出所有页」走这里，**无 `pages list` 命令**） |
| `slide pages get` | 获取指定幻灯片上所有形状的摘要信息和动画列表 |

## 其余编辑接口：动态下发，不在 skill 固化

改文字 / 图表 / 表格 / 形状样式与位置 / 备注 / 批注 / 动画 / 节 / 母版主题等其他编辑接口**由服务端动态下发，不在 skill 中固化**。需要时：

```bash
wecom-cli slide --schema                # 探测当前下发的命令全集（随版本变化）
wecom-cli slide <资源> <动作> --schema   # 取单条命令的参数 schema
```

探测到目标命令 → 按其 schema 传参使用；探测不到 → 当前版本未开放，**换已固化的命令组合兜底**（典型：整页重做）。探测到批量变体（如 `batchadd` 系列）时**优先用批量**，减少往返与编辑冲突。
