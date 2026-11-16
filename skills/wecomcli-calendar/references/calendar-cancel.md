# calendar schedules cancel — 取消日程

取消用户发起的日程。**周期日程支持取消**，须通过 `repeat_mod` 指定本次取消的生效范围（见下文「[周期日程取消](#周期日程取消repeat_mod)」）。

> [!CAUTION]
> 这是**写入操作** — 参数就绪后直接执行。

## 命令

```bash
# 取消普通日程
wecom-cli calendar schedules cancel --json '{"schedule_id": "<schedule_id>"}'

# 取消周期日程的某一场（repeat_event_time 为该场次的开始时间）
wecom-cli calendar schedules cancel --json '{
  "schedule_id": "<schedule_id>",
  "repeat_mod": {
    "repeat_edit_type": "this_only",
    "repeat_event_time": "2026-04-08 14:00:00"
  }
}'
```

## 参数

| 参数 | 类型 | 必填 | 说明 |
|------|------|:----:|------|
| `schedule_id` | string | 是 | 要取消的日程 ID（由 search/list 返回，格式不固定，直接透传即可） |
| `repeat_mod` | object | 否 | 重复日程修改设置。**取消周期日程时必传**，用于指定取消哪些场次，子字段见「[周期日程取消](#周期日程取消repeat_mod)」 |

**返回**：成功时返回空对象 `{}`，这是正常结果，不代表失败。收到空对象即可告知用户取消成功，**不需要再查询核对**。

> `cancel` + `all` 取消整个系列时，此前经 `this_only` 拆出的独立场次会一并取消。

## 周期日程取消（`repeat_mod`）

**是否传 `repeat_mod`，一律以定位结果的 `repeat_rule.is_repeat` 为准 [CRITICAL]**：

| 定位到的日程 | `schedule_id` | `repeat_mod` |
|---|---|---|
| `repeat_rule.is_repeat=true`（周期日程） | 定位结果中该日程的 `schedule_id` | **必传** |
| `repeat_rule` 为空 / `is_repeat` 非 `true`（普通单次日程） | 同上 | **不传** |

两种情形的 `schedule_id` 都取自本次定位结果，不要沿用历史上下文中的旧 id。

> **注意**：周期系列中曾被 `this_only` 修改过的那一场，已脱离系列成为**独立单次日程**（`list` 返回的 `repeat_rule` 为空，且 `schedule_id` 与系列不同）。它虽然在用户眼里仍属于那个重复日程，但按上表就是普通单次日程——用它自己的 `schedule_id` 直接取消、不传 `repeat_mod`。

`repeat_mod` 的子字段：

| 字段 | 类型 | 说明 |
|------|------|------|
| `repeat_edit_type` | string | 编辑类型：`this_only` 仅此场次 / `this_and_future` 此场次及将来 / `all` 所有场次 |
| `repeat_event_time` | string | 重复日程事件时间（`YYYY-MM-DD HH:mm:ss`），即要取消的那一场的时间；取定位结果中该场次的 `begin_time` |

**两条约束 [CRITICAL]**

1. **`repeat_edit_type` 必须由用户拍板**：取消是不可逆写操作，且三种范围差别极大（退掉一场 / 退掉往后所有 / 整个系列全退），禁止自行默认。用户未明确时用文字询问：`这是周期日程，本次取消的范围是？（请回复：仅此场次 / 此场次及将来 / 所有场次）`；用户已明确（如"就这周不开""以后都不开了"）时直接映射，无需追问。
2. **`repeat_event_time` 禁止猜测**：必须取自 `search`/`list`/`get` 定位到的目标场次的 `begin_time`，不能凭记忆编造——传错场次会取消掉用户没想取消的那一场。

## 定位目标时的跨载体消歧（模糊取消）[REQUIRED]

用户说"取消那个会 / 取消 xx 会 / 把那个会取消掉"等模糊表述、未明确是日程还是在线会议时，**不要只在日程里找**——「会」可能是一条纯日程，也可能是含在线会议链接的会议，只查一边会漏定位：

- **明确是日程 / 安排**（说的是"日程 / 安排 / 我的日历"且不带在线会议特征）→ 只在本技能 `search`/`list` 定位，走 `schedule cancel`。
- **明确是在线会议**（提到入会链接 / 会议号 / 视频会议 / 腾讯会议 / 远程参会等专属特征）→ 改用 `读取 wecomcli-meeting 技能` 在会议里定位并 `meeting cancel`。
- **模糊无法判定** → 日程和会议**两边都查**：本技能 `search`/`list` + `读取 wecomcli-meeting 技能` 用同样关键词 / 时间查会议，合并候选、按"主题 + 时间"去重（同一场两边都命中只保留一条），再用文字让用户**选定要取消的唯一一条**；选定后按其归属路由——是纯日程 → `schedule cancel`；是会议（或两边都命中的同一场）→ 改用 `读取 wecomcli-meeting 技能` 走 `meeting cancel`（会连带取消关联日程，禁止再对该日程调用 `schedule cancel`）。

> **与查询消歧的区别**：查询时可以两边都查、都展示；但取消是**写操作，绝不能两边都直接取消**，模糊时必须先让用户确认唯一目标，再执行对应的 cancel。

## 取消日程工作流

```
用户发起取消意图
    |
    +-- 搜索目标日程
    |   +-- 有关键词 → search（不追问时间）
    |   +-- 有时间信息 → list 按时间范围查询
    |   +-- 都没有 → 用文字询问引导用户补全缺失的参数
    |
    +-- 匹配结果处理
    |   +-- 唯一匹配 → 继续
    |   +-- 多条匹配 → 用文字让用户选择目标日程：
    |   |     文字提问："找到多个匹配日程，请选择要取消的一个："
    |   |     列出候选（如"项目评审 - 4月8日 14:00 / 项目评审 - 4月15日 14:00"，最多 4 条）
    |   +-- 无匹配 → 扩大搜索 / 提示换关键词
    |
    +-- 判定会议关联与周期性：meeting 与 repeat_rule 在 search/list 结果中已直接返回，直接判定，无需补 get
    |   +-- meeting.meeting_code 非空（含在线会议链接）
    |   |     +-- 改期意图（改约/挪到/顺延，即使带"取消"）→ 改用 `读取 wecomcli-meeting 技能`，把 meeting_id 传入 meeting update 改时间
    |   |     +-- 纯取消（不办了/不要了）→ 改用 `读取 wecomcli-meeting 技能` 走 meeting cancel（会连带取消关联日程，禁止在此 schedule cancel）
    |   +-- meeting 为空（纯日程）
    |         +-- 普通日程 → 直接执行 cancel
    |         +-- 周期日程（repeat_rule.is_repeat=true）→ 先确定取消范围：用户未明确则用文字询问
    |               （仅此场次 / 此场次及将来 / 所有场次），再带 repeat_mod 执行 cancel
    |               （repeat_event_time 取该场次的 begin_time）
    |
    +-- 执行 cancel（不论日程由谁创建，都直接执行，不提前拒绝）→ 依返回结果判断：
          +-- 返回空对象 {} → 取消成功，报告结果
          +-- 返回权限类错误 → 说明当前用户无权取消该日程，告知用户并建议联系日程创建人（creator_name）操作
```

## 典型场景

### 1. 取消普通日程

```
用户：帮我取消明天的项目评审
→ 调用 search（keywords=["项目评审"]，明天）
→ 找到 1 条
→ 调用 cancel（schedule_id）
→ 报告：已取消
```

### 2. 取消周期日程

```
用户：帮我取消这周五的周会
→ 搜索 → 找到"团队周会"（周期日程，repeat_rule.is_repeat=true）
→ 用户未说取消哪些场次 → 用文字询问：这是周期日程，本次取消的范围是？（请回复：仅此场次 / 此场次及将来 / 所有场次）
→ 用户答「仅此场次」→ repeat_mod={"repeat_edit_type":"this_only","repeat_event_time":"<该场次 begin_time>"}
→ 调用 cancel → 返回 {} → 报告：已取消
```

### 3. 取消非本人创建的日程

不预先按"是否本人创建"拦截，直接执行 cancel，根据返回结果判断。
```
用户：帮我取消明天张三约的评审
→ 搜索 / get → 找到日程（创建人是张三）
→ 不提前拒绝 → 直接调用 cancel（schedule_id）
→ 依返回判断：
    · 返回 {} → 报告：已取消
    · 返回权限错误 → 告知：你无权取消该日程，建议联系创建人张三操作
```

## 注意事项

- **权限判定交给接口**：不预先按"是否本人创建"限制取消——直接执行 `cancel`，根据返回结果判断：返回空对象 `{}` 即取消成功；返回权限类错误则说明当前用户无权取消该日程，告知用户并建议联系创建人操作。
- **直接执行**：参数就绪后直接调用取消接口，无需展示摘要或等待确认。
- **周期日程取消须带 `repeat_mod`**：检测到目标日程 `repeat_rule.is_repeat=true` 时，须传 `repeat_mod`（`repeat_edit_type` + 目标场次的 `repeat_event_time`）指定取消范围，其中 `repeat_edit_type` 必须由用户拍板、不得自行默认（详见「[周期日程取消](#周期日程取消repeat_mod)」）。
- **"取消……改约到……"是改期、不是取消**：同时出现"取消"和"改到/改约/挪到/顺延"时本质是改期，按 SKILL.md「改约 / 重建日程前必须先识别会议关联」走更新流程，禁止拆成 cancel + create（目标 `meeting` 非空时，cancel + create 会丢失会议链接）。仅用户明确"不办了/不要了/直接取消"且无改期诉求时才执行 cancel。
- **关联在线会议的日程不在此取消**：若目标 `meeting` 非空（`search`/`list` 结果即可判定，无需补 `get`；或同一场在会议和日程两边都命中），改用 `读取 wecomcli-meeting 技能` 走 `meeting cancel`，会议取消后该日程会被一并取消，禁止在此对其调用 `schedule cancel`。
- **禁止暴露 userid**：结果展示中参与人只显示人名。

## 参考

- [wecomcli-calendar](../SKILL.md) — 日程技能主文档
- [calendar-search](calendar-search.md) — 搜索日程（用于定位目标日程）
- [calendar-agenda](calendar-agenda.md) — 查看日程安排
