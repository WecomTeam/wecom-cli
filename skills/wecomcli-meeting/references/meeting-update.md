# 操作参考：更新会议

更新已创建会议的信息，包括主题、时间、参会人、地点等。**写操作**，参数就绪后直接执行。不预先按"是否本人创建"拦截，能否修改由接口返回结果判断。**支持更新周期会议**，见下文「周期会议更新」。

## 命令

```bash
wecom-cli meeting update --json '{...}'
```

## 请求参数

| 字段 | 类型 | 必填 | 说明 |
| ---- | ---- | ---- | ---- |
| `meeting_id` | string | 是 | 会议 ID（来自 `list`/`search` 返回的 `meeting_id` 字段，长字符串，非 9 位会议号） |
| `subject` | string | 否 | 新的会议主题 |
| `begin_time` | string | 否 | 新的开始时间（格式 YYYY-MM-DD HH:mm:ss） |
| `end_time` | string | 否 | 新的结束时间（格式 YYYY-MM-DD HH:mm:ss） |
| `add_attendees` | array | 否 | 新增参会人列表，对象数组，格式 `[{"userid": "woxxx"}]` |
| `remove_attendees` | array | 否 | 移除参会人列表，对象数组，格式 `[{"userid": "woxxx"}]` |
| `location` | string | 否 | 新的会议地点（文本）。用户给的是**会议室**时须走 `meeting_room_id` 改订（见工作流「会议室变更解析」），不要把会议室名仅写进 `location`；用户给的是**非会议室的普通文本地点**时直接写入 `location` |
| `meeting_room_id` | string | 否 | 会议室 ID，传入预定（改订）会议室。用户要更换会议室时，须先经 `rooms search`（会议室查询接口定义在 `读取 wecomcli-calendar 技能` 的 [会议室查询参考](../../wecomcli-calendar/references/calendar-meeting-room.md)）查询新会议室状态，确认`status=bookable` 可用后才传入新的 `meeting_room_id`；ID 仅工具链使用，禁止出现在用户回复正文 |
| `description` | string | 否 | 新的会议备注 |
| `sub_meeting_id` | string | 否 | 子会议 ID，标记要修改周期会议中的哪一场。**周期会议必传**，取值来自 `list`/`search` 返回值，禁止编造；非周期会议不传 |
| `repeat_edit_type` | string | 否 | 周期会议修改类型，**周期会议场景下必传**，非周期会议不传：`this_only` 仅修改此场次子会议 / `this_and_future` 修改此场次及将来所有场次 |

> **只传要改的字段**：未传入的字段保持原值不变。其中 `location` / `description` 传**空字符串**是"清空"语义，与"不传"（保持原值）不同——用户没提到地点 / 备注时一律不传该字段，只有用户明确要"去掉地点 / 清掉备注"时才传空字符串。

## 周期会议更新

周期会议（`is_repeat_meeting=true`）可以更新，但除 `sub_meeting_id`（标记改哪一场）外，还必须传 `repeat_edit_type` 标记**修改范围**，否则接口无法确定要改的是一场还是整个系列：

| 取值 | 含义 | 适用表达 |
| ---- | ---- | ---- |
| `this_only` | 仅修改此场次子会议 | "只改这一次""就这周的挪一下""下周那场单独改" |
| `this_and_future` | 修改此场次及将来所有场次 | "以后都改成""这个点固定下来""后面每次都改""整个系列都改" |

> **修改范围必须由用户拍板 [CRITICAL]**：用户表达里没有明确"只改这一次"还是"以后都改"时，**必须用文字追问，禁止自行默认任一取值**——两者影响范围差别极大（改错会波及整个系列或只动一场），且改完无法一键回滚。文字提问：`这是周期会议，本次修改的范围是？（请回复：仅这一场 / 这一场及以后所有场次）`
>
> **非周期会议禁止传这两个字段**：非周期的普通会议不传 `sub_meeting_id` 与 `repeat_edit_type`。

## 返回字段

| 字段 | 说明 |
| ---- | ---- |
| `meeting_id` | 会议 ID |
| `sub_meeting_id` | 子会议 ID，唯一标识一场周期会议下的子会议；**周期会议修改的场景才返回**，不传则不涉及子会议 |
| `subject` | 更新后的会议主题 |
| `begin_time` | 更新后的开始时间 |
| `end_time` | 更新后的结束时间 |
| `attendees` | 更新后的完整参会人列表，扁平对象数组，每项含 `userid` / `name` / `is_external`；内部成员与外部联系人统一用 `userid`，由 `is_external` 区分（`true` 为外部参会人）。`name` 为框架按 userid 注入的展示名（只读）。展示取 `name`，禁止展示 userid |
| `attendees_count` | `attendees` 数组元素数量 |
| `location` | 更新后的会议地点 |
| `description` | 更新后的会议备注 |

## 约束

- **不预先按"是否本人创建"拦截修改**，直接执行 `update`，能否修改由接口返回结果判断：返回更新后的字段即成功；返回权限类错误则说明当前用户无权修改，告知用户并建议联系会议发起人
- 只需传入要修改的字段，未传入字段保持原值不变
- **周期会议支持更新**，但须带 `sub_meeting_id` + `repeat_edit_type`（见「周期会议更新」）。禁止对周期会议逐场 `update` 拼凑整体修改，也禁止改为取消重建
- 修改时间时 `end_time` 必须晚于 `begin_time`
- **更换会议室**：用户要换会议室时，`meeting_room_id` 必须先经 `rooms search` 查询、确认新会议室 `status=bookable` 可用后才传入；禁止跳过查询凭记忆/猜测直接传，禁止把会议室名仅写进 `location`（那样不会真正占用会议室）。会议室查询接口须 `读取 wecomcli-calendar 技能` 的 [会议室查询参考](../../wecomcli-calendar/references/calendar-meeting-room.md)
- userid（前缀为 `wo`）不接受姓名直接传入；用户提供的是姓名时通过 `读取 wecomcli-contact 技能` 解析为 userid，禁止把姓名当 userid 拼接，禁止凭记忆或猜测编造

## 工作流

```
用户发起更新意图
    |
    +-- 定位目标会议
    |   +-- 有关键词 → meeting search（不追问时间）
    |   +-- 有时间信息 → meeting list 按时间范围查询
    |   +-- 都没有 → 用文字询问引导用户补全信息
    |
    +-- 匹配结果处理
    |   +-- 唯一匹配 → 继续
    |   +-- 多条匹配 → 用文字让用户选择：
    |   |     文字提问："找到多个匹配会议，请选择要修改的一个："
    |   |     列出候选（如"项目评审 - 4月8日 14:00 / 项目评审 - 4月15日 14:00"，最多 4 条）
    |   +-- 无匹配 → 建议修改关键词或扩大时间范围重试
    |
    +-- 判断是否周期会议（用定位阶段 search/list 已返回的 is_repeat_meeting，无需额外调 get）
    |   +-- 非周期会议 → 直接收集修改内容，执行更新（不传 sub_meeting_id / repeat_edit_type）
    |   +-- 周期会议 → 补齐两个必传字段后再更新：
    |   |   +-- sub_meeting_id：取定位阶段 list/search 返回的该系列最近一场的 sub_meeting_id；
    |   |   |     用户指定的是中间某一场（非最近一场）→ 不执行，如实告知只能修改最近一场（给出该场日期）
    |   |   |     或最近一场及以后所有场次，让用户选择；禁止拿最近一场顶替，禁止编造 sub_meeting_id
    |   |   +-- repeat_edit_type：用户已明确"只改这一次" → this_only；明确"以后都改 / 整个系列" → this_and_future；
    |   |         未明确 → 必须用文字追问，禁止自行默认：
    |   |         文字提问："这是周期会议，本次修改的范围是？（请回复：仅这一场 / 这一场及以后所有场次）"
    |   > **关键约束**：两个字段缺任一时禁止调用 update；修改范围未经用户拍板前禁止执行。
    |
    +-- 参会人变更解析（如有）
|   +-- 上下文中已有合法 userid（`wo` 前缀）→ 直接使用，跳过搜索
|   +-- 用户提供的是姓名 → 通过 `读取 wecomcli-contact 技能` 批量搜索所有新增/移除的人名
    |   |   +-- 某关键词唯一匹配 → 直接使用，无需确认
    |   |   +-- 某关键词多个匹配 → 用文字让用户选择（列出姓名 + 部门）：
    |   |   |     文字提问："搜索到多个「{姓名}」，请确认要操作哪一位？"
    |   |   |     列出候选（如"张三 - 产品部 - 产品经理 / 张三 - 技术部 - 前端工程师"，最多 4 条；超出取前 4 条并提示用户可进一步缩小范围）
    |   |   +-- 某关键词无结果 → 用文字提示用户确认人名是否正确，停止执行
    |   +-- 汇总全部 userid → 组装 add_attendees / remove_attendees（对象数组 [{"userid": "woxxx"}]）
    |
    |   > **关键约束**：只要存在多个候选人，必须等用户选择后才能继续，不得自动选取任何一个。
    |
    +-- 会议室变更解析（如用户要换会议室）
    |   +-- 确定查询时段：用会议起止时间；若本次同时改时间，用改后的新时段
    |   +-- 读取 wecomcli-calendar 技能的 [会议室查询参考](../../wecomcli-calendar/references/calendar-meeting-room.md) → rooms search 查新会议室状态
    |   |   +-- target 中有 bookable 项 → 取该项 target[].room.meeting_room_id（多个 bookable 时用文字让用户选）
    |   |   +-- 指定会议室 target=[]（查无此名）/ 命中项均 unavailable（被占）→ 必须先告知用户"未查到/无法预订你指定的『xxx』会议室"，
    |   |   |       再用文字让用户决定是否改订其他会议室或换时间；禁止用其他名称会议室静默替代（候选仅 1 个也须用户确认）
    |   |   +-- 未指定具体会议室（target=[]）：
    |   |   |   +-- recommendations 多个 → 用文字让用户选（禁止自动取第一个）
    |   |   |   +-- recommendations 仅 1 个 → 可直接使用
    |   |   |   +-- recommendations = [] → 告知无可用会议室，引导换楼或换时间
    |   +-- 拿到用户确认的、可用的 meeting_room_id → 传入 update
    |   > **关键约束**：新会议室未经 rooms search 确认 bookable 之前，禁止传 meeting_room_id 调 update。
    |
    +-- 时间/参会人忙闲检查（改时间或加参会人时必做）[REQUIRED]
    |   +-- 触发条件：本次修改了 begin_time/end_time，或新增了参会人（add_attendees）
    |   +-- 查询对象与时段：核心原则是排除"因本会议占用而必然忙碌"的时段，避免自冲突误报 [CRITICAL]
    |   |     ——【已在本会议中的人】（自己/创建者 + 已有参会人）只查"与本会议当前时段【不重叠】"的时间，【新增参会人】才查完整目标时段。分三种情况：
    |   |   +-- ① 只加人、不改时间 → 仅对【新增参会人 add_attendees 中的内部成员】查【会议原时段】；
    |   |   |     绝不把当前用户（自己/创建者）及已有参会人纳入——他们正被本会议占用、必然显示"忙"，是误报
    |   |   |     （用户本意就是让别人加入自己这个已定时间的会议）
    |   |   +-- ② 改时间且新旧时段【不重叠】（平移/改期，如 15:00→17:00）→ 对【改后仍需参加的内部成员（含自己）+ 新增内部参会人】查【完整新时段】
    |   |   |     （新旧无交集，现有参会人查新时段不会撞上本会议原时段，可正常纳入自己/已有参会人）
    |   |   +-- ③ 改时间且新旧时段【有重叠】（延长/提前等，新时段含部分原时段）→ 分两类查：
    |   |   |     · 新增内部参会人：查【完整新时段】
    |   |   |     · 现有内部参会人及自己：只查【新时段去掉与原时段重叠后剩下的增量段】
    |   |   |       （如 15:00-16:00 延到 15:00-17:00 只查 16:00-17:00；15:00-16:00 提前到 14:00-16:00 只查 14:00-15:00）；
    |   |   |       增量段为空（如仅缩短时间）则现有参会人及自己无需查
    |   +-- 按上面裁剪后的查询对象执行；裁剪后查询对象为空、或仅剩外部联系人（wm，忙闲不可查）时才跳过——不要因为"只有自己"就跳过（②/③ 里自己在新时段/增量段内仍要查，避免约到自己已占用的时段）
    |   +-- 忙闲接口不在本技能 → 读取 wecomcli-calendar 技能的 [忙闲查询参考](../../wecomcli-calendar/references/calendar-freebusy.md)，按上面圈定的查询对象 + 时段调 free list（窗口 ≤ 24h）
    |   |   +-- 无冲突 → 继续执行 update
    |   |   +-- 有人占线 → 用文字让用户二选一（禁止自行改期）：
    |   |   |     文字提问："该时间段{姓名}有冲突，如何处理？（请回复：坚持这个时间 / 换一个时间）"
    |   |   +-- 接口失败 → 告知忙闲暂不可用，确认时间后继续，不阻塞
    |
    +-- 执行 update（不论会议由谁创建，都直接执行，不提前拒绝）→ 依返回结果判断：
          +-- 返回更新后的字段 → 修改成功，展示更新后的会议摘要
          +-- 返回权限类错误 → 说明当前用户无权修改该会议，告知用户并建议联系会议发起人
```

### 异常路径

| 异常情况 | 处理方式 |
|---------|---------|
| 接口返回无权修改（非发起人） | 直接执行 update 后依返回判断；返回权限错误时告知用户无权操作，建议联系会议发起人 |
| 用户要修改周期系列中间的某一场 | 拿不到该场 `sub_meeting_id`，如实告知只能修改最近一场（给出日期）或最近一场及以后所有场次，让用户选择；或引导其在企业微信客户端操作。禁止拿最近一场顶替，禁止编造 `sub_meeting_id` |
| 周期会议修改范围未明确 | 用文字追问：`这是周期会议，本次修改的范围是？（请回复：仅这一场 / 这一场及以后所有场次）`，据此传 `this_only` / `this_and_future`，禁止自行默认 |
| 修改时间冲突（end ≤ begin） | 提示用户结束时间必须晚于开始时间，请重新输入 |
| wecomcli-contact 技能搜索无结果 | 提示用户确认人名是否正确，或尝试其他搜索词 |
| wecomcli-contact 技能返回多个候选人 | 用文字询问用户（列出候选姓名 + 部门），等待用户选择后汇总继续 |
| 换会议室时新会议室不可用 | `rooms search` 返回 `unavailable`/`not_found`：用文字让用户从 `recommendations` 候选中选，或引导换楼（`expand_to_other_buildings`）/换时间；禁止传不可用的 `meeting_room_id` 调 update |
| 更新接口返回错误 | 检查参数格式，重新阅读本文档确认用法 |

## 示例请求

**修改普通会议时间和主题**：
```json
{
  "meeting_id": "<meeting_id>",
  "subject": "产品需求评审（更新）",
  "begin_time": "2026-04-08 15:00:00",
  "end_time": "2026-04-08 16:00:00"
}
```

**新增/移除参会人**：
```json
{
  "meeting_id": "<meeting_id>",
  "add_attendees": [{"userid": "woxxxc"}],
  "remove_attendees": [{"userid": "woxxxb"}]
}
```

**更换会议室**（`meeting_room_id` 须先经 `rooms search` 确认新会议室 `status=bookable`）：
```json
{
  "meeting_id": "<meeting_id>",
  "meeting_room_id": "mrmxxxx"
}
```

**修改周期会议的某一场**（`sub_meeting_id` + `repeat_edit_type` 必传）：
```json
{
  "meeting_id": "<meeting_id>",
  "sub_meeting_id": "<sub_meeting_id>",
  "repeat_edit_type": "this_only",
  "begin_time": "2026-09-08 15:00:00",
  "end_time": "2026-09-08 16:00:00"
}
```

**修改周期会议的此场及将来所有场次**：
```json
{
  "meeting_id": "<meeting_id>",
  "sub_meeting_id": "<sub_meeting_id>",
  "repeat_edit_type": "this_and_future",
  "begin_time": "2026-09-08 16:00:00",
  "end_time": "2026-09-08 17:00:00"
}
```

## 典型场景

### 1. 修改会议时间

```
用户：把明天下午3点的评审会推迟1小时
→ 调用 meeting search（keywords=["评审"]）→ 获取 meeting_id
→ 调用 meeting get → 判断非周期会议（不做是否本人创建的前置拦截）
→ 组装参数：begin_time="2026-04-08 16:00:00"，end_time="2026-04-08 17:00:00"
→ 调用 update → 展示更新结果
```

### 2. 添加参会人

```
用户：把王五加到明天的评审会
→ 调用 meeting search → 获取 meeting_id
→ 通过 wecomcli-contact 技能搜索「王五」→ 返回 2 个候选
→ 用文字询问：搜索到多个「王五」，请确认要邀请哪一位？（列出：王五 - 市场部 - 市场专员 / 王五 - 技术部 - 前端工程师）
→ 用户选择后，获得对应 userid
→ 调用 update，add_attendees=[{"userid": "woxxxe"}]
→ 展示更新后完整参会人列表
```

### 3. 修改周期会议

```
用户：下周一的周会改到下午3点，只改这一次（下周一即最近一场）
→ 调用 meeting search（keywords=["周会"]）→ 找到周期会议（is_repeat_meeting=true），拿到 meeting_id 与最近一场的 sub_meeting_id
→ 用户已明确"只改这一次" → repeat_edit_type="this_only"（无需追问）
→ 忙闲检查（改时间且新旧时段不重叠 → 查完整新时段）
→ 调用 update：meeting_id + sub_meeting_id + repeat_edit_type="this_only"
   + begin_time="<下周一日期> 15:00:00" + end_time="<下周一日期> 16:00:00"
→ 展示更新结果

用户：周会以后都改到下午3点
→ 定位到周期会议，取最近一场的 sub_meeting_id（this_and_future 从最近一场起生效）
→ 用户已明确"以后都改" → repeat_edit_type="this_and_future"
→ 调用 update → 展示更新结果

用户：周会改到下午3点
→ 定位到周期会议 → 修改范围未明确
→ 必须用文字追问：这是周期会议，本次修改的范围是？（请回复：仅这一场 / 这一场及以后所有场次）
→ 用户拍板后才调用 update，禁止自行默认任一取值
```

### 4. 更换会议室

```
用户：把明天评审会的会议室换到 1608
→ meeting search/list 拿 meeting_id（及会议起止时间）→ get 拿会议详情（不做是否本人创建的前置拦截）
→ 读取 wecomcli-calendar 技能的会议室查询参考，用会议时段 + room_keyword="1608" 调 rooms search
→ target 中有 bookable 项 → 取其 target[].room.meeting_room_id
→ 调用 update，meeting_room_id="mrmxxxx"
→ 展示更新后会议摘要（只露会议室 name）

用户：明天的评审会换个会议室
→ 拿 meeting_id 与时段 → rooms search（未指定具体会议室，target=[]）
→ recommendations 多个 → 用文字让用户选（展示 name + 楼层 + 容量）
→ 用户选定后取其 meeting_room_id → update
```

## 参考

- [wecomcli-meeting](../SKILL.md) — 会议技能主文档
- [meeting-search](meeting-search.md) — 搜索会议（获取 meeting_id）
- [meeting-list](meeting-list.md) — 查看会议列表
- [meeting-cancel](meeting-cancel.md) — 取消会议
- `读取 wecomcli-calendar 技能` 的 [忙闲查询参考](../../wecomcli-calendar/references/calendar-freebusy.md) — 改时间/加参会人时查共同空闲
- `读取 wecomcli-calendar 技能` 的 [会议室查询参考](../../wecomcli-calendar/references/calendar-meeting-room.md) — 更换会议室时确认新会议室 `status=bookable`
