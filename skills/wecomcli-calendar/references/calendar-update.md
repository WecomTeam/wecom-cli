# calendar schedules update — 更新日程

更新已有日程的信息，包括主题、时间、地点、参与人等。**周期日程支持更新**，须通过 `repeat_mod` 指定本次修改的生效范围（见下文「[周期日程更新](#周期日程更新repeat_mod)」）。

> [!CAUTION]
> 这是**写入操作** — 参数就绪后直接执行。

## 命令

```bash
# 修改日程主题和时间
wecom-cli calendar schedules update --json '{
  "schedule_id": "SCHEDULE_ID",
  "subject": "产品评审（更新）",
  "begin_time": "2026-04-08 14:00:00",
  "end_time": "2026-04-08 15:00:00"
}'

# 新增/移除参与人
wecom-cli calendar schedules update --json '{
  "schedule_id": "SCHEDULE_ID",
  "add_attendees": [{"userid": "woxxxc"}],
  "remove_attendees": [{"userid": "woxxxb"}]
}'

# 更换会议室（meeting_room_id 须先经 rooms search 确认新会议室 status=bookable）
wecom-cli calendar schedules update --json '{
  "schedule_id": "SCHEDULE_ID",
  "meeting_room_id": "mrmxxxx"
}'

# 修改周期日程（此场次及将来，repeat_event_time 为目标场次的开始时间）
wecom-cli calendar schedules update --json '{
  "schedule_id": "SCHEDULE_ID",
  "begin_time": "2026-04-08 15:00:00",
  "end_time": "2026-04-08 16:00:00",
  "repeat_mod": {
    "repeat_edit_type": "this_and_future",
    "repeat_event_time": "2026-04-08 14:00:00"
  }
}'
```

## 参数

| 参数 | 类型 | 必填 | 默认值 | 说明 |
|------|------|:----:|--------|------|
| `schedule_id` | string | 是 | — | 日程 ID |
| `subject` | string | 否 | — | 日程主题 |
| `begin_time` | string | 否 | — | 开始时间（格式 `YYYY-MM-DD HH:mm:ss`）。必须晚于当前时刻；与 `end_time` 必须同时传入或同时省略。 |
| `end_time` | string | 否 | — | 结束时间（格式 `YYYY-MM-DD HH:mm:ss`）。必须晚于 `begin_time`（支持跨天 / 多天，无时长上限）；与 `begin_time` 必须同时传入或同时省略。 |
| `location` | string | 否 | — | 日程地点（文本）。用户给的是**会议室**时须走 `meeting_room_id` 改订（见「更换会议室工作流」），不要把会议室名仅写进 `location`；用户给的是**非会议室的普通文本地点**时直接写入 `location` |
| `meeting_room_id` | string | 否 | — | 会议室 ID，传入预定（改订）会议室。用户要更换会议室时，须先经 `rooms search`（见 [calendar-meeting-room](calendar-meeting-room.md)）查询新会议室状态，确认 `status=bookable` 可用后才传入新的 `meeting_room_id`；ID 仅工具链使用，禁止出现在用户回复正文 |
| `description` | string | 否 | — | 日程描述 |
| `allow_self_join` | bool | 否 | — | 是否允许自行加入 |
| `is_all_day` | bool | 否 | — | 是否全天日程 |
| `add_attendees` | object[] | 否 | `[]` | 新增参与人列表，对象数组，格式 `[{"userid": "woxxx"}, {"userid": "woyyy"}]` |
| `remove_attendees` | object[] | 否 | `[]` | 移除参与人列表，对象数组，格式 `[{"userid": "woxxx"}]` |
| `repeat_mod` | object | 否 | — | 重复日程修改设置。**更新周期日程时必传**，用于指定本次修改改哪些场次，子字段见「[周期日程更新](#周期日程更新repeat_mod)」 |

**部分更新（Patch 语义）**：只传要修改的字段，未传字段保持原值；`description` / `location` 传空字符串表示**清空**该字段。

**返回**：`detail` 对象，包含更新后的完整日程详情，字段如下：

| 字段 | 类型 | 说明 |
|------|------|------|
| `detail.schedule_id` | string | 日程 ID |
| `detail.subject` | string | 日程主题 |
| `detail.begin_time` | string | 开始时间（YYYY-MM-DD HH:mm:ss） |
| `detail.end_time` | string | 结束时间（YYYY-MM-DD HH:mm:ss） |
| `detail.attendees` | object[] | 参与人列表，格式 `[{"userid": "USERID", "name": "englishname(name)"}]`，展示时只取 `name`，禁止展示 userid |
| `detail.meeting_room` | object | 会议室信息，含 `meeting_room_id` + `meeting_room_name`（改订会议室后返回，展示用 name） |
| `detail.location` | string | 日程地点 |
| `detail.description` | string | 日程描述 |
| `detail.allow_self_join` | bool | 是否允许自行加入 |
| `detail.is_all_day` | bool | 是否全天日程 |
| `detail.meeting` | object | 关联的腾讯会议信息，含 `meeting_id`/`meeting_code`/`meeting_link` |
| `detail.reminders` | object | 提醒规则：`is_remind`（bool，是否开启提醒）+ `reminder_time`（int[]，提醒时间与事件开始时间的差值秒数，负数为提前提醒） |
| `detail.creator_name` | string | 创建者姓名 |
| `detail.cal_id` | string | 所属日历本 ID |
| `detail.calendar_name` | string | 日历本名称 |
| `detail.is_share_cal` | bool | 所属日历是否共享日历（日历创建者非当前用户） |
| `detail.repeat_rule` | object | 周期规则，见下表 |
| `detail.timezone` | object | 时区信息，含 `timezone_id`（IANA 时区标识，如 `Asia/Shanghai`，优先使用）+ `timezone_offset`（UTC 偏移量秒数，`timezone_id` 为空时使用） |

**`detail.repeat_rule` 子字段：**

| 字段 | 类型 | 说明 |
|------|------|------|
| `is_repeat` | bool | 是否重复日程 |
| `repeat_type` | string | 重复类型：`daily`/`weekly`/`monthly`/`monthly_on_the_nth_day`/`yearly`/`yearly_on_the_nth_day`/`work_day` |
| `repeat_flag` | string[] | 重复标记，数组，可选值：`leap_month`（闰月）、`never_ends`（永不结束） |
| `repeat_time` | int | 重复次数，`0` 表示不限次数 |
| `repeat_interval` | int | 重复间隔 |
| `repeat_until` | string | 重复结束时刻（格式 YYYY-MM-DD HH:mm:ss），`0` 表示永不结束 |
| `repeat_week_of_month` | string[] | 每月第几周，数组，可选值：`first`/`second`/`third`/`fourth`/`last` |
| `repeat_day_of_week` | string[] | 每周周几，数组，可选值：`MO`/`TU`/`WE`/`TH`/`FR`/`SA`/`SU`。仅自定义重复且 `repeat_type=weekly` 时有效 |
| `repeat_month_of_year` | int[] | 每年哪几个月，数组，取值范围：1~12。仅自定义重复且 `repeat_type=yearly` 时有效 |
| `repeat_day_of_month` | int[] | 每月哪几天，数组，取值范围：1~31。仅自定义重复且 `repeat_type` 为 `monthly` / `yearly` 时有效 |
| `is_custom` | bool | 是否自定义重复周期 |
| `exception` | object[] | 例外场次列表（周期日程中被删除或被单独改过时间的单场），子字段见 [calendar-agenda](calendar-agenda.md) 的 `exception[]` 子字段表 |

## 周期日程更新（`repeat_mod`）

目标日程 `repeat_rule.is_repeat=true` 时，本次修改改哪些场次由 `repeat_mod` 决定：

| 字段 | 类型 | 说明 |
|------|------|------|
| `repeat_edit_type` | string | 编辑类型：`this_only` 仅此场次 / `this_and_future` 此场次及将来 / `all` 所有场次 |
| `repeat_event_time` | string | 重复日程事件时间（`YYYY-MM-DD HH:mm:ss`），即要修改的那一场的时间；取定位结果中该场次的 `begin_time` |

**三条约束 [CRITICAL]**

1. **`repeat_edit_type` 必须由用户拍板**：三种范围语义差别很大（改一场 / 改往后所有 / 改整个系列），禁止自行默认。用户未明确时用文字询问：`这是周期日程，本次修改的范围是？（请回复：仅此场次 / 此场次及将来 / 所有场次）`；用户已明确（如"就改这一次""以后都改成这个时间"）时直接映射，无需追问。
2. **`repeat_event_time` 禁止猜测**：必须取自 `search`/`list`/`get` 定位到的目标场次的 `begin_time`，不能用修改后的新时间、也不能凭记忆编造。改时间场景下 `begin_time`（新时间）与 `repeat_event_time`（原场次时间）是两个不同的值，不要混用。
3. **写操作可能改变 `schedule_id`，须重新定位后再操作**：`repeat_mod` 生效后，`update` 返回的 `detail.schedule_id` 未必等于入参的 `schedule_id`（规则见下节）。继续操作同一日程时，禁止复用入参的 id，须以返回的 `detail.schedule_id` 或重新 `list` 定位的结果为准。

### 是否传 `repeat_mod`：以定位结果的 `repeat_rule` 为准 [CRITICAL]

| 定位到的日程 | `schedule_id` | `repeat_mod` |
|---|---|---|
| `repeat_rule.is_repeat=true`（周期日程） | 定位结果中该日程的 `schedule_id` | **必传**，范围由用户拍板 |
| `repeat_rule` 为空 / `is_repeat` 非 `true`（普通单次日程） | 同上 | **不传**，按普通日程更新 |

两种情形的 `schedule_id` 都取自本次定位结果，不要沿用历史上下文中的旧 id。

> **注意**：周期系列中曾被 `this_only` 修改过的那一场，已脱离系列成为**独立单次日程**（`list` 返回的 `repeat_rule` 为空，且 `schedule_id` 与系列不同）。它虽然在用户眼里仍属于那个重复日程，但按上表就是普通单次日程——用它自己的 `schedule_id` 直接更新、不传 `repeat_mod`。

### `schedule_id` 何时会变

判断标准是**本次修改是否把系列切开**：系列内所有场次共享同一份属性，改动只覆盖部分场次时，被改的部分必须裂变为新对象并分配新 `schedule_id`；覆盖全部场次时结构不变、id 不变。

| 情形 | 是否切开系列 | `schedule_id` | 结果 |
|---|---|---|---|
| `all` | 否 | **不变** | 整系列统一更新 |
| `this_and_future`，且目标是系列**首场** | 否（前段为空，实际覆盖全部） | **不变** | 等效于 `all`，服务端自动处理，无需转换写法 |
| `this_and_future`，且目标是**中间场次** | 是（切成两段） | **重新分配** | 该场及之后迁入新系列（保留 `repeat_rule`），原系列 `repeat_until` 自动收敛到前一场 |
| `this_only` | 是（抠出一场） | **重新分配** | 该场脱离系列成为独立单次日程（`repeat_rule` 变空），原系列记一条 `exception` |

> **无需为首场做等价转换**：用户选"此场次及将来"而目标恰为首场时，直接按用户选择传 `this_and_future` 即可，服务端会识别并按整系列处理；不要自行改传 `all`。

## 更新日程流程

> **完整的日程管理工作流**（含查询日程 ID、参数补全策略等）定义在 [SKILL.md](../SKILL.md) 的核心场景中。本文档专注于 `update` 命令的参数和调用细节。

**快速决策参考**：
- 必填参数：`schedule_id`（缺失时需先通过搜索日程获取，见 [calendar-search](calendar-search.md)）
- 仅传入需修改的字段，未传入字段保持不变
- **周期日程须带 `repeat_mod`**：定位到的目标日程若 `repeat_rule.is_repeat=true`，须先确定 `repeat_edit_type`（用户未明确则用文字询问）并带上目标场次的 `repeat_event_time`，再执行 update（见「[周期日程更新](#周期日程更新repeat_mod)」）
- **连续操作同一周期日程时，每次写操作后重新 `list` 定位** [REQUIRED]：`this_only`、以及作用于中间场次的 `this_and_future` 会改变目标场次的 `schedule_id`，禁止复用上一步的 id 直接发起下一次修改；重新定位后按该场次的 `repeat_rule` 决定是否带 `repeat_mod`
- **权限判定交给接口**：不预先按"是否本人创建"拦截——直接执行 `update` 并按返回结果判断（详见注意事项）
- 参数就绪后直接执行，结果展示时人名不暴露 userid

### 参与人变更工作流

涉及 `add_attendees` 或 `remove_attendees` 时，按以下方式获取 userid：上下文中已有合法 userid（`wo` 前缀）则直接使用；用户提供的是姓名时通过 `读取 wecomcli-contact 技能` 将姓名解析为 userid。

```
+-- 参与人变更解析（如有 add_attendees / remove_attendees）
|   +-- 上下文中已有合法 userid → 直接使用，跳过搜索
|   +-- 用户提供的是姓名 → 通过 `读取 wecomcli-contact 技能` 批量搜索所有新增/移除的人名
|   |   +-- 某关键词唯一匹配 → 直接使用，无需确认
|   |   +-- 某关键词多个匹配 → 用文字让用户选择（列出姓名 + 部门）：
|   |   |     文字提问："搜索到多个「{姓名}」，请确认要操作哪一位？"
|   |   |     列出候选（如"张三 - 产品部 - 产品经理 / 张三 - 技术部 - 前端工程师"，最多 4 条，超出取前 4 并提示用户缩小范围）
|   |   +-- 某关键词无结果 → 用文字提示用户确认人名是否正确，停止执行
|   +-- 汇总全部 userid → 组装 add_attendees / remove_attendees（对象数组 `[{"userid": "woxxx"}]`）
+-- 时间/参与人忙闲检查（改时间或加参与人时必做）[REQUIRED]
|   +-- 触发条件：本次修改了 begin_time/end_time，或新增了参与人（add_attendees）
|   +-- 核心原则（避免自冲突误报）[CRITICAL]：对【已在本日程中的人】（自己/创建者 + 已有参与人）
|   |     只查"与本日程当前时段【不重叠】"的时间——本日程已占着原时段，查到的"忙"是它自己造成的误报；
|   |     【新增参与人】才查完整目标时段
|   +-- 据此分三种情况：
|   |   +-- ① 只加人、不改时间 → 仅对【新增参与人 add_attendees】查【日程原时段】；
|   |   |     已有参与人和自己/创建者全部不查（原时段被本日程占满，纳入必然误报；
|   |   |     用户本意就是让别人加入自己这个已定时间的日程）
|   |   +-- ② 改时间且新时段与原时段【不重叠】（平移/改期，如 15:00 改到 17:00）→
|   |   |     对【改后仍需参加的人 + 新增参与人】查【新时段】（新旧无交集，现有参与人查新时段不会撞上本日程）
|   |   +-- ③ 改时间且新时段与原时段【有重叠】（延长/提前等，新时段含部分原时段）→
|   |         · 新增参与人：查【完整新时段】
|   |         · 现有参与人及自己：只查【新时段去掉与原时段重叠后剩下的增量段】
|   |           （如 15:00-16:00 延到 15:00-17:00，现有人只查 16:00-17:00；如 15:00-16:00 提前到 14:00-16:00，只查 14:00-15:00）；
|   |           增量段为空（如仅缩短时间）则现有参与人无需查
|   +-- 按上面裁剪后的查询对象执行；裁剪后查询对象为空、或某人查询时段为空（如仅缩短时间的增量段为空）时才跳过——不要因为"日程只有自己"就跳过（②/③ 里自己在新时段/增量段内仍要查，避免约到自己已占用的时段）
|   +-- 读取 [calendar-freebusy](calendar-freebusy.md)，按上面圈定的查询对象 + 时段调 free list（窗口 ≤ 24h）
|   |   +-- 无冲突 → 继续执行 update
|   |   +-- 有人占线 → 用文字让用户二选一（禁止自行改期）：
|   |   |     文字提问："该时间段{姓名}有冲突，如何处理？（请回复：坚持这个时间 / 换一个时间）"
|   |   +-- 接口失败 → 告知忙闲暂不可用，确认时间后继续，不阻塞
+-- 执行 update
```

> **关键约束**：只要存在多个候选人，必须等用户选择后才能继续，不得自动选取任何一个。

> 边界说明：上面"现有参与人及自己只查增量段、增量段为空则该人不查"，是因为本日程已占着原时段、扣除重叠后这些人在重叠段没有剩余窗口可查（**不是"只有自己就整条跳过"——自己在增量段/新时段内仍要查**）；不要把它套到新建场景——新建时日程尚不存在，自己必须按完整目标时段查（见 [calendar-create](calendar-create.md) 步骤3）。
>
> 查忙闲时 `min_duration_minutes` 设成所查时段时长（或直接传 1），否则被默认 30 分钟过滤掉的短空闲段，会让落在其中的短日程误报为冲突。

### 更换会议室工作流

涉及 `meeting_room_id`（更换 / 改订会议室）时，必须先经会议室查询确认新会议室可用，禁止凭记忆或猜测直接传入 `meeting_room_id`：

```
+-- 用户要更换会议室
|   +-- 确定查询时段：用日程的起止时间；若本次同时改时间，用改后的新 begin_time/end_time
|   +-- 读取 [calendar-meeting-room](calendar-meeting-room.md)，按其编排执行：
|   |   +-- 用户提了楼名 → buildings list 匹配出 building_city/name；没提则跳过（后端按当前所在楼兜底）
|   |   +-- rooms search（带日程时段 + 可选楼 + 可选room_keyword + min_capacity）
|   +-- 按新会议室状态决策：
|   |   +-- 指定会议室 target 中有 bookable 项 → 取该项 target[].room.meeting_room_id 传入 update（多个 bookable 时用文字让用户选）
|   |   +-- 指定会议室 target=[]（查无此名）/命中项均 unavailable（被占）→ 必须先告知用户"未查到/无法预订你指定的『xxx』会议室"，
|   |   |       再用文字让用户决定是否改订其他会议室或换时间；禁止用其他名称会议室静默替代（候选仅 1 个也须用户确认）
|   |   +-- 未指定具体会议室（target=[]）：
|   |   |   +-- recommendations 多个候选 → 用文字让用户选（禁止自动取第一个）
|   |   |   +-- recommendations 仅 1 个    → 可直接使用该候选 meeting_room_id
|   |   |   +-- recommendations = []       → 告知该时段无可用会议室，引导换楼（expand_to_other_buildings）或换时间
|   +-- 拿到用户确认的、可用的 meeting_room_id
|   +-- 判断地点是否需要同步：取原日程 detail.location 与原 detail.meeting_room.meeting_room_name 比对
|   |   +-- 原 location 就是原会议室（与原会议室名/地点一致）→ 把 location 一并改为新会议室对应地点（新会议室名 / rooms search 返回的楼+房间信息），与 meeting_room_id 同次 update 传入
|   |   +-- 原 location 是用户自定义文本（与原会议室无关）/ 原本无会议室 → 不动 location，避免覆盖用户自填内容
|   +-- 执行 update（meeting_room_id，必要时 + location）
```

> **关键约束**：新会议室未经 `rooms search` 确认 `bookable` 之前，禁止传入 `meeting_room_id` 调用 update——否则会改订到不可用或不存在的会议室。会议室查询/选择是本次 update 的前置阻塞项。

> **地点同步**：若原日程已绑定会议室、且 `location` 就是这个原会议室（地点只是在镜像会议室名），更换会议室时要把 `location` 一并改成新会议室对应地点，和 `meeting_room_id` 在同一次 update 传入，避免出现"会议室已换、地点还停在旧会议室"的不一致。若 `location` 是用户自填的、与原会议室无关的文本，则保持不动。

## 典型场景

### 1. 修改日程时间

```
用户：把明天下午3点的评审推迟1小时
→ 调用 search 查询日程 → 获取 schedule_id
→ 组装参数：begin_time="2026-04-08 16:00:00"，end_time="2026-04-08 17:00:00"
→ 调用 update
```

### 2. 添加参与人

**唯一匹配**：
```
用户：把王五加到明天的评审会
→ 通过 wecomcli-contact 技能搜索「王五」→ 唯一匹配，获得 userid woxxxe
→ 调用 search 查询日程 → 获取 schedule_id
→ 调用 update，add_attendees=[{"userid": "woxxxe"}]
```

**多候选情形**：
```
用户：把张三加到明天的评审会
→ 通过 wecomcli-contact 技能搜索「张三」→ 返回 2 个候选
→ 用文字询问：搜索到多个「张三」，请确认要操作哪一位？（列出：张三 - 产品部 - 产品经理 / 张三 - 技术部 - 前端工程师）
→ 用户选择后，获得对应 userid
→ 调用 search 查询日程 → 获取 schedule_id
→ 调用 update，add_attendees=[{"userid": "woxxxf"}]
```

### 3. 移除参与人

```
用户：把李四从明天的评审会里移除
→ 通过 wecomcli-contact 技能搜索「李四」→ 返回 2 个候选
→ 用文字询问：搜索到多个「李四」，请确认要移除哪一位？（列出：李四 - 设计部 - UI设计师 / 李四 - 技术部 - 后端工程师）
→ 用户选择后，获得对应 userid
→ 调用 search 查询日程 → 获取 schedule_id
→ 调用 update，remove_attendees=[{"userid": "woxxxd"}]
```

### 4. 周期日程更新

```
用户：下周一的周会改到下午3点
→ search 拿到日程 → repeat_rule.is_repeat=true（周期日程）
→ 用户未说改哪些场次 → 用文字询问：这是周期日程，本次修改的范围是？（请回复：仅此场次 / 此场次及将来 / 所有场次）
→ 用户答「仅此场次」→ repeat_mod={"repeat_edit_type":"this_only","repeat_event_time":"<该场次原 begin_time>"}
→ 调用 update（begin_time/end_time 为新时间 + repeat_mod）
→ 返回 detail.schedule_id = <新 id>、detail.repeat_rule 为空：该场次已拆为独立日程
```

### 4.1 已拆出的场次再次修改（连续操作同一周期日程）

```
用户（承接上例）：这场的主题也改一下
→ 重新 list 定位该场次（不要复用系列 id）
→ 该场次 repeat_rule 为空 → 属独立单次日程
→ 调用 update：schedule_id=<该场次自身 id>，subject=<新主题>，不传 repeat_mod
```

```
用户：以后的周会都挪到下午3点（此前已对某场做过 this_and_future）
→ 重新 list 定位目标场次 → 该场次 repeat_rule.is_repeat=true，但 schedule_id 已是新系列的 id
→ 用新系列的 schedule_id + repeat_mod 发起修改，不要用最初那个系列 id
```

### 5. 修改非本人创建的日程

不预先按"是否本人创建"拦截，直接执行 update，根据返回结果判断。
```
用户：把明天的评审改到下午3点（该日程创建人是李四）
→ search / get → 找到日程
→ 不因创建人非本人而提前拒绝 → 直接调用 update（begin_time/end_time）
→ 依返回判断：
    · 返回 detail（更新后详情）→ 报告：已改到下午3点
    · 返回权限错误 → 告知：你无权修改该日程，建议联系创建人李四操作
```

### 6. 更换会议室

```
用户：把明天评审会的会议室换到 1608
→ search 拿 schedule_id（及日程起止时间）
→ 读取 calendar-meeting-room，用日程时段 + room_keyword="1608" 调 rooms search
→ target 中有 bookable 项 → 取其 target[].room.meeting_room_id
→ 调用 update，meeting_room_id="mrmxxxx"
→ 展示更新后日程摘要（只露会议室 name）

用户：明天的评审会换个会议室
→ search 拿 schedule_id 与时段 → rooms search（未指定具体会议室，target=[]）
→ recommendations 多个 → 用文字让用户选（展示 name + 楼层 + 容量）
→ 用户选定后取其 meeting_room_id → update
```

## 注意事项

- **权限判定交给接口**：不预先按"是否本人创建"限制修改——直接执行 `update`，根据返回结果判断：返回 `detail`（更新后完整详情）即修改成功；返回权限类错误则说明当前用户无权修改该日程，告知用户并建议联系创建人操作。
- **含会议链接的日程不在本技能改时间**：目标日程 `meeting` 非空（含在线会议链接，`search`/`list` 结果即可判定，无需补 `get`）时，`calendar update` 改不动其背后的在线会议，须改用 `读取 wecomcli-meeting 技能` 把 `meeting_id` 传入 `meeting update`。本技能 `update` 只处理纯日程（`meeting` 为空）。
- **`schedule_id` 获取**：如用户未提供，需先通过 [calendar-search](calendar-search.md) 查询。
- **部分更新**：只需传入要修改的字段，未传字段服务端保持原值不变。
- **更换会议室**：用户要换会议室时必须先经 [calendar-meeting-room](calendar-meeting-room.md) 的 `rooms search` 查询新会议室、确认 `status=bookable` 可用后，再把新会议室的 `meeting_room_id` 传入 update。禁止跳过查询、凭记忆/猜测直接传 `meeting_room_id`，禁止把会议室名仅写进 `location`（那样不会真正占用会议室）。同时改时间又改会议室时，用改后的新时段查询会议室。若原 `location` 本就是原会议室（地点镜像会议室名），换会议室时把 `location` 一并改为新会议室对应地点同次传入；`location` 是用户自填的无关文本则不动。
- **时间字段成对传入**：修改时间时 `begin_time` 与 `end_time` 必须同时传入；只传其一会与原值组合，可能立即违反"晚于当前时刻"约束而失败。
- **时间合法性**：`begin_time` 必须晚于当前真实时刻、`end_time` 晚于 `begin_time`（支持跨天 / 多天，无时长上限）。任何不满足都先用文字询问引导用户修正，禁止直接传错时间试错。
- **周期日程更新须带 `repeat_mod`**：目标日程 `repeat_rule.is_repeat=true` 时，须传 `repeat_mod`（`repeat_edit_type` + 目标场次的 `repeat_event_time`）指定修改范围，其中 `repeat_edit_type` 必须由用户拍板、不得自行默认；禁止改为取消重建（详见「[周期日程更新](#周期日程更新repeat_mod)」）。
- **写操作后 `schedule_id` 可能变化，须重新定位 [CRITICAL]**：修改只覆盖系列部分场次时会发生裂变并分配新 `schedule_id`——`this_only` 把目标场次拆为独立单次日程、`this_and_future`（目标为中间场次）把该场及之后迁入新系列；而 `all`、以及目标为首场的 `this_and_future` 覆盖了全部场次，id 不变（规则见「[`schedule_id` 何时会变](#schedule_id-何时会变)」）。继续操作同一日程时，须以返回的 `detail.schedule_id` 或重新 `list` 定位的结果为准；其中已拆为独立日程的场次（`repeat_rule` 为空）按普通日程更新、**不再传 `repeat_mod`**。
- **改时间/加参与人需查忙闲 [REQUIRED]**：本次修改了 `begin_time`/`end_time` 或新增了参与人（`add_attendees`）时，执行 update 前必须先读取 [calendar-freebusy](calendar-freebusy.md) 查忙闲；占线时用文字让用户在「坚持这个时间 / 换一个时间」二选一，禁止自行改期。
  - **查询对象须排除"因本日程占用而必然忙碌"的人 [CRITICAL]**：核心原则是【已在本日程中的人】（自己/创建者 + 已有参与人）只查"与本日程当前时段【不重叠】"的时间，【新增参与人】查完整目标时段。分三种情况：①只加人、不改时间 → 只对新增参与人查日程原时段，自己和已有参与人全部不查；②改时间且新旧时段不重叠（平移/改期）→ 对"改后仍需参加的人 + 新增参与人"查新时段；③改时间且新旧时段有重叠（延长/提前等）→ 新增参与人查完整新时段，现有参与人及自己只查"新时段去掉与原时段重叠后的增量段"（如 15:00-16:00 延到 15:00-17:00 只查 16:00-17:00），增量段为空（如仅缩短时间）则不查。    裁剪后查询对象为空、或某人查询时段为空时才跳过——不要因为"日程只有自己"就跳过（②/③ 中自己在新时段/增量段内仍要查）。
- **禁止暴露 userid**：结果展示中只显示人名。
- **直接执行**：参数补全后直接调用更新接口，无需展示摘要或等待确认。
- **时区标注**：`detail.timezone.timezone_offset != 28800`（非东八区）时，结果摘要按 [SKILL.md 输出格式规范](../SKILL.md) 的时区标注规则在时间后带上时区。传入的 `begin_time` / `end_time` 按日程时区解释，禁止自行换算。

## 参考

- [wecomcli-calendar](../SKILL.md) — 日程技能主文档
- [calendar-search](calendar-search.md) — 搜索日程（获取 schedule_id）
- [calendar-freebusy](calendar-freebusy.md) — 查询参与人共同空闲（改时间/加参与人时查忙闲）
- [calendar-create](calendar-create.md) — 创建日程
- [calendar-cancel](calendar-cancel.md) — 取消日程
- [calendar-meeting-room](calendar-meeting-room.md) — 会议室查询（更换会议室时确认新会议室 `status=bookable`）
