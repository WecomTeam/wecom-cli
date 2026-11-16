# 操作参考：创建会议

## 命令

```bash
wecom-cli meeting create --json '{...}'
```

## 请求参数

| 字段                         | 类型    | 必填 | 说明                                                   |
| ---------------------------- | ------- | ---- | ------------------------------------------------------ |
| `subject`         | string  | 是   | 会议主题                                               |
| `begin_time`      | string  | 是   | 开始时间, 格式 `YYYY-MM-DD HH:mm:ss`, 必须晚于当前时间 |
| `end_time`        | string  | 是   | 结束时间, 格式 `YYYY-MM-DD HH:mm:ss`, 必须晚于 `begin_time`, 且与 `begin_time` 间隔不超过 24 小时。**用户未给出时长时默认为 `begin_time` 的 1 小时后（不追问）** |
| `attendees`       | array   | 否   | 参会人，对象数组，格式 `[{"userid": "woxxx"}, {"userid": "woyyy"}]`（`wo` 前缀），最多 300 个。用户提供的是姓名时通过 `读取 wecomcli-contact 技能` 解析为 userid |
| `location`        | string| 否| 会议地点（文本）。**用户给的地点是某会议室时**：须先经 `rooms search` 预订（见步骤 4），预订成功后**只传 `meeting_room_id`、不再写 `location`**（会议室名由后端关联返回，无需在 `location` 里重复填充）。**用户给的地点不是会议室时**（如"星巴克""客户现场"）：直接写入 `location`，不涉及 `meeting_room_id`。禁止把会议室名仅写进 `location` 却不订房——那样不会真正占用会议室 |
| `meeting_room_id` | string  | 否   | 会议室 ID，传入即触发后端"建会议 + 占会议室"原子操作（**周期会议同样支持预定会议室**：与 `recurrence` 在同一次 create 中一并传入，会议室按首场时段查询、由服务端占用整个系列）。查询会议室拿此 ID 的能力不在本技能，须 `读取 wecomcli-calendar 技能` 的 [会议室查询参考](../../wecomcli-calendar/references/calendar-meeting-room.md)（`buildings list` + `rooms search`）。订会议室时只传本字段即可，**不需要再把会议室名重复填进 `location`**。**该 ID 仅工具链使用，禁止出现在用户回复正文** |
| `description`     | string  | 否   | 会议备注/描述                                          |
| `timezone`        | object  | 否   | 时区设置，用户指定时区时传入，未指定则不传。格式：`{"timezone_id": "Asia/Shanghai", "timezone_offset": 28800}`，`timezone_id` 为 IANA 时区标识（**优先使用**），`timezone_offset` 为 UTC 偏移量（秒，`timezone_id` 为空时才使用） |
| `cal_id`          | string  | 否   | 所属日历本 ID。仅当上下文已有明确的日历本 ID 时传入，禁止凭记忆 / 猜测编造；用户未指定则省略 |
| `mark_optional_attendees` | array | 否 | 标记为「可选参与人」的成员，**平铺字符串数组**，格式 `["woxxx", "woyyy"]`（**注意与 `attendees` 的对象数组格式不同，不要写成 `[{"userid": "woxxx"}]`**）。取值须是已出现在 `attendees` 中的 userid；仅当用户明确说了某人"可选 / 可不来 / 有空再来"时才传，未提及则省略 |
| `recurrence`      | object  | 否   | 周期规则。**不传、或传 `is_repeat=false` 即创建单次会议**；用户要"每天 / 每周 / 每月 / 每个工作日"重复时传入本对象，子字段见下方「[周期规则（recurrence）](#周期规则recurrence)」 |

> **会议室预订能力在 wecomcli-calendar 技能 [CRITICAL]**：本技能不直接定义会议室查询接口——用户提出"订会议室 / 在 1605 开 / 找个会议室 / 某栋楼的会议室"等诉求时，须 `读取 wecomcli-calendar 技能` 的 [会议室查询参考](../../wecomcli-calendar/references/calendar-meeting-room.md)（`buildings list` + `rooms search`）查到真实会议室，拿 `meeting_room_id` 传入本创建（见步骤 4）。禁止把会议室名仅写进 `location`（那样不会真正占用会议室），也禁止凭记忆 / 猜测编造 `meeting_room_id`。仅当用户给的是非会议室的普通文本地点时，才只写 `location`。

## 周期规则（recurrence）

`recurrence` 为选填对象：**不传、或 `is_repeat=false` 即创建单次会议**。传入即在同一次 `meeting create` 中由服务端一次性展开为整个系列，禁止批量调用 create 创建多条单次会议来模拟周期。

| 字段 | 类型 | 说明 |
| ---- | ---- | ---- |
| `is_repeat` | bool | 是否为周期会议。`true` 才按周期展开 |
| `repeat_type` | string | 重复类型：`daily` 每天 / `weekly` 每周 / `monthly` 每月 / `work_day` 每个工作日。**不支持 `monthly_on_the_nth_day`（每月第 N 个星期几）、`yearly`（每年）、`yearly_on_the_nth_day`（每年第 N 个）** |
| `repeat_time` | int | 结束次数（展开后的总场次数，**含首场**）：1~200；`repeat_type=monthly` 及"每两周"（`weekly` + `repeat_interval=2`）时上限 50。与 `repeat_until` 二选一即可，都传时以 `repeat_until` 为准 |
| `repeat_interval` | int | 重复间隔：每 N（天/周/月）重复一次，**需配合 `is_custom=true` 生效**；上限：`daily` 99、`weekly` 12、`monthly` 12 |
| `repeat_until` | string | 重复结束时刻，格式 `YYYY-MM-DD HH:mm:ss`；必须晚于会议开始时间，且不能超过当前时间起约 49 年。与 `repeat_time` 二选一即可，只传 `repeat_time` 时服务端自动换算成本字段 |
| `repeat_day_of_week` | array | 每周周几重复（`repeat_type=weekly` 时生效）：`MO` 周一 ~ `SU` 周日，最多 7 个。**服务端会自动并入会议开始时间当天，首场必在周期内** |
| `repeat_day_of_month` | array | 每月哪几天重复（`repeat_type=monthly` 时生效）：取值 1~31，最多 31 个。**服务端会自动并入会议开始时间当天，首场必在周期内** |
| `is_custom` | bool | 是否自定义重复周期：`true` 时 `repeat_interval` 才生效（如每 3 天 / 每 2 周） |

> **首场自动并入 [CRITICAL]**：`repeat_day_of_week` / `repeat_day_of_month` 会被服务端自动并入 `begin_time` 当天，所以组装参数时须先把 `begin_time` 对齐到用户指定的第一个星期几 / 日期，否则会凭空多出一场（如用户要"每周二、四"但 `begin_time` 落在周一，实际会变成每周一/二/四）。
>
> **`repeat_interval` 必须配 `is_custom=true`**：只传 `repeat_interval` 而不传 `is_custom=true` 时间隔不生效。"每两周一次" = `repeat_type="weekly"` + `repeat_interval=2` + `is_custom=true`。
>
> **结束条件二选一**：`repeat_time`（次数）与 `repeat_until`（截止时刻）只需给一个，都传时以 `repeat_until` 为准。

## 返回字段

| 字段           | 说明                     |
| -------------- | ------------------------ |
| `meeting_id`   | 会议唯一标识             |
| `meeting_link` | 入会链接, 可分享给参会人 |
| `meeting_code` | 9 位会议号               |

## 约束

- 开始时间必须晚于当前时间, 否则创建失败
- 结束时间 `end_time` 必须晚于 `begin_time`, 且与 `begin_time` 间隔不超过 24 小时（86400 秒）。用户要建超过 24h 的单场会议时，直接告知不支持并拒绝，禁止自行拆分成多场会议或用其他方式变通绕过；如确需多天安排，由用户明确拆分要求后再分别创建
- 参会人总数不超过 300 人
- **支持创建周期 / 重复会议**：通过 `recurrence` 字段在同一次 create 中一次性创建整个系列（见「[周期规则（recurrence）](#周期规则recurrence)」）。禁止批量调用 create 接口创建多条单次会议来模拟周期效果——那样在企微客户端中不会被识别为周期会议，反而造成多条独立会议难以批量管理。
- **`repeat_type` 仅支持 `daily` / `weekly` / `monthly` / `work_day` 四种**：用户要"每月第 N 个星期几"（`monthly_on_the_nth_day`）、"每年"（`yearly` / `yearly_on_the_nth_day`）等重复方式时，**直接告知用户不支持该重复方式，并引导用户在企业微信客户端手动预订**；禁止用 `monthly` 等其他类型近似替代，禁止批量创建多条单次会议模拟。
- **周期会议创建后可改、可取消，但操作粒度受限**：`meeting update` 支持修改（须带 `sub_meeting_id` + `repeat_edit_type`，见 [meeting-update](meeting-update.md)）；`meeting cancel` 支持取消（不传 `sub_meeting_id` 取消整个系列、传则仅取消该场次，见 [meeting-cancel](meeting-cancel.md)）。但 `list` / `search` 对一个周期系列只返回最近要开的一场，**系列中间的某一场拿不到 `sub_meeting_id`，无法单独操作**，此类需求只能引导用户在企业微信客户端调整。
- `mark_optional_attendees` 是平铺字符串数组（`["woxxx"]`），与 `attendees` 的对象数组格式不同，且其成员须已包含在 `attendees` 中
- `cal_id` 仅在上下文已有明确日历本 ID 时传入，禁止编造
- userid（前缀为 `wo`）不接受姓名直接传入；用户提供的是姓名时通过 `读取 wecomcli-contact 技能` 解析为 userid，禁止把姓名当 userid 拼接，禁止凭记忆或猜测编造
- 时区字段（`timezone`）仅在用户明确指定时区时传入，否则省略

## 工作流

### 正常路径

0. **日程 / 会议消歧（仅当用户说"会议/会/开会/约会/xx会"且未明确时）**：用户未明确是日程还是会议（会议含在线会议链接、可远程/视频参会）时，必须先用文字追问，禁止默认直接创建会议。已明确是会议的场景（如"发个入会链接""要会议号""视频会议""远程参会"）时才跳过本步骤。

   > **注意 1**：用户只说"会议/会/开会"等泛称，本身不构成"明确"——这些词没有表明是日程还是会议，**禁止仅因 query 里有"会议"二字就默认走会议创建、跳过本步骤**，必须先用文字追问。
   >
   > **注意 2**：仅给出地点/会议室号的表述（如"在 1605 开会""到 A 座会议室碰一下""订个会议室开会"）不构成"明确是会议"——会议室里同样可能只是纯线下安排，是日程还是会议仍未知。此类"只有地点"的表述仍需先用文字询问消歧，不要因为带了地点就跳过本步骤。
   > **问题与选项固定 [CRITICAL]**：消歧确认时，问题与可选项都必须原文照用、严格禁止修改任何内容——问题固定为 `"需要创建日程还是会议？"`，可选项固定为 `日程` / `会议`；不得改写问题措辞、增减或改写选项、翻译，或自行设计其他表述（如"在线会议 / 线上会议 / 视频会议 / 线下会议"等）。

   用文字向用户提问：`需要创建日程还是会议？（请回复：日程 / 会议）`

   - 用户答「会议」→ 留在本技能，继续步骤 1。
   - 用户答「日程」→ 停止本工作流，改用 `读取 wecomcli-calendar 技能` 创建日程。
   - 若用户同时要线下与远程参会，因含在线会议链接，留在本技能创建即可（创建会议会同时生成对应日程，无需再去 wecomcli-calendar 技能另建日程）。
1. **参数补全**：从对话中提取主题、时间、时长、参会人信息。
   - `subject` 缺失时，用文字询问会议主题。仅描述参会方式或动作的词（如「视频会议 / 开个会 / 会面 / 远程接入」）不构成有效 `subject`，一律按缺失处理走文字询问，禁止把它们当主题直接创建——否则用户事后补主题需额外调一次修改会议工具，效率非常低。文字提问如：`会议主题是什么？`（可举例"项目同步 / 需求评审 / 周例会 / 一对一沟通"供参考，最多举 4 个）
   - `begin_time` 缺失时，用文字一步询问，直接列出具体的"日期+时刻"候选项让用户选（结合当前时间动态推断，所有候选项必须晚于当前时刻，禁止使用"上午/下午/傍晚"等模糊表述，最多列 4 个）。文字提问如：`会议什么时候开始？`（候选按当前时刻动态生成、均须晚于现在，例如当前 19:40 可列 "明天 09:00 / 明天 14:00 / 明天 16:00 / 后天 09:00"）
   - `end_time` / 时长缺失时，**默认时长 60 分钟（1 小时），不追问**，按 `end_time = begin_time + 60 分钟` 换算为结束时间。仅当用户明确说了时长（如"开半小时""聊俩小时"）时按其值换算。
   - `attendees` 缺失时，用文字追问参会人，禁止默认创建无参会人的会议或自行猜测；地点、会议室等非必填参数用户未明确指定时不追问，也不传该字段。文字提问如：`需要邀请哪些人参会？`（可列出"仅自己"及根据对话语境补充的 1-3 个候选人名供参考）
2. **参会人解析**：上下文中已有合法 userid（`wo` 前缀）则直接使用，跳过本步骤；用户提供的是人名时，通过 `读取 wecomcli-contact 技能` 将所有姓名批量搜索，逐个关键词独立处理结果：
   - 某关键词唯一匹配 → 直接使用，无需确认
   - 某关键词多个匹配 → 用文字让用户选择：`搜索到多个「{姓名}」，请确认要邀请哪一位？` 并列出候选（来自 wecomcli-contact 技能搜索结果，如"张三 - 产品部 - 产品经理 / 张三 - 技术部 - 前端工程师"，最多 4 条，超出取前 4 并提示用户缩小范围）
   - 某关键词无结果 → 用文字提示用户重新输入：`未找到「{姓名}」，请确认姓名是否正确`
   - 所有姓名确认完毕后，汇总 userid 组装为对象数组一并传入 `attendees`
3. **参会人忙闲检查（新建一律必做）[REQUIRED]**：新建会议的查询对象必含当前用户自己（`wo` 前缀），故创建前必须先查忙闲，避免约到冲突时间（**含只有自己的会议——避免约到自己已占用的时段**）；**本步骤是步骤 5（调用创建接口）的前置阻塞项——未完成忙闲检查、或检测到冲突但未经用户拍板，一律禁止进入创建（仅接口失败的降级例外，见下）**；外部联系人（`wm` 前缀，忙闲不可查）不纳入查询对象、但**不因此跳过**整体检查。忙闲接口不在本技能 —— 须 `读取 wecomcli-calendar 技能` 的 [忙闲查询参考](../../wecomcli-calendar/references/calendar-freebusy.md)，调 `free list`（窗口 ≤ 24h，跨天需分段；**查询对象 = 自己 + 其他内部参会人，新建会议时自己也要纳入，避免约到自己已占用的时段**）：
   - **推荐时段长度 ≠ 会议时长（精确 / 范围时间均适用）**：忙闲返回的推荐时段只用于确定会议**开始时间**，其长度不代表会议时长；用户选定时段后，会议时长仍以用户明确指定的为准，用户未明确时长时一律默认 1 小时（`begin_time + 1h`），禁止把推荐时段的长度直接当作会议时长。
   - **精确时间**（用户已给定具体开始时间）：用 `[begin_time, end_time]` 查忙闲。有人占线时，必须用文字让用户在「坚持这个时间 / 换一个时间」二选一，禁止自行改期或劝阻：`该时间段{姓名}有冲突，如何处理？（请回复：坚持这个时间 / 换一个时间）`
   - **范围时间**（如"明天下午"）：调 `free list` 拿共同空闲 `slots`，按 `slots[0].available_count` 与 `total_count` 判断全员空闲 / 降级 / 全忙（详见忙闲查询参考），挑前几个时段让用户选定后再继续。
   - **接口失败**：告知"忙闲查询暂时不可用"，确认时间后继续创建，不阻塞。
4. **会议室预订（仅当用户有订房意图时触发）**：用户提到"订会议室 / 在 1605 / 找个会议室 / 某栋楼的会议室"等意图时才走本步骤，没提则跳过。会议室的查询接口定义不在本技能 —— 须 `读取 wecomcli-calendar 技能` 的 [会议室查询参考](../../wecomcli-calendar/references/calendar-meeting-room.md)，按其编排执行。**五条硬性规则不可跳过**：① **先查询、后推荐、后创建**——`meeting_room_id` 必须来自 `rooms search` 的真实返回值，禁止跳过查询直接 create，禁止凭记忆 / 猜测编造；且在成功调用 `rooms search` 之前，禁止凭记忆 / 上下文 / 想象向用户罗列或推荐任何具体会议室（含用文字给出的候选、正文里的房间名 / 号 / 楼层 / 容量），要让用户选会议室必须先查到真实候选再组装选项；② **存在多个会议室必须让用户选**——命中多个候选时必须用文字让用户选择或指定具体会议室，禁止自动替用户挑选；③ **会议室必须订房、且只传 `meeting_room_id`**——只要用户给的地点是会议室，就必须经 `rooms search` 查到真实会议室并通过 `meeting_room_id` 传入，严禁把会议室名 / 房间号仅塞进 `location` 字段就创建（那样不会真正占用会议室）；预订成功后创建时**只传 `meeting_room_id`**（占用），**不需要再把会议室名重复填进 `location`**（会议室名由后端关联返回），仅当用户给的是非会议室的普通地点时才只写 `location`、不走订房；④ **优先先订房、后建会**——用户在创建时就提到会议室的，应先把会议室敲定（拿到用户确认的 `meeting_room_id`）再进入步骤 5 创建会议，本步骤是步骤 5 的前置阻塞项，避免创建后会议室被抢占。若会议室查询 / 选择尚未完成（如等待用户在候选中选择），必须停在本步骤等待，不得提前调用 create；⑤ **指定会议室查无/不可用必须先告知、禁止静默替换**——用户指定的会议室 `not_found`（查无此名）或 `unavailable`（被占）时，先告知用户"未查到 / 无法预订你指定的『xxx』会议室"，再让用户决定改订其他会议室或换时间，禁止静默替代（候选仅 1 个也须用户确认）。若创建时漏订或事后要换会议室，可走 [meeting-update](meeting-update.md) 传入新 `meeting_room_id` 改订（须先经 `rooms search` 确认 `status=bookable`），不必取消重建。
   - 用户提了楼名 → `buildings list` + LLM 匹配得到楼；没提楼则跳过（后端用当前所在楼兜底）
   - `rooms search`（带已定 `begin_time`/`end_time` + 可选楼 + 可选 `room_keyword` + `min_capacity = len(attendees) + 1`）
   - 按结果决策：用户**指定了具体会议室**（传了 `room_keyword`）且 `target` 中有 `bookable` 项 → 取该项 `meeting_room_id`（仅 1 个直接用，多个则用文字让用户选）；指定的会议室 `target=[]`（查无此名）或命中项均 `unavailable`（被占）时——先告知用户"未查到 / 无法预订你指定的『xxx』会议室"，再用文字让用户决定改订其他会议室或换时间，禁止静默替代（候选仅 1 个也须用户确认）；用户**未指定具体会议室**（`target` 为 `[]`）时——`recommendations` 有**多个**候选则必须用文字让用户选择，只有 **1 个**候选可直接使用，**为空**则问是否跨楼或换时间
   - 选定后将用户确认的 `meeting_room_id` 带入下一步的 create
5. **调用创建接口**：参数就绪后直接执行 `wecom-cli meeting create --json '{...}'`。
   > **创建前置门禁（CRITICAL）**：
   > ① **忙闲门禁**——新建会议查询对象必含当前用户自己（`wo` 前缀），故调用 create 前**必须已完成步骤 3 的忙闲检查**；若检测到冲突，必须已通过文字询问让用户在「坚持这个时间 / 换一个时间」中拍板。禁止在"未查忙闲"或"冲突未经用户决定"的情况下直接 create（仅忙闲接口失败时按降级继续，不阻塞）。
   > ② **会议室门禁**——若用户提到过会议室相关内容，则调用 create 前**必须已持有一个来自 `rooms search`、并经用户确认的真实 `meeting_room_id`**，且该 ID 已写入创建参数。只要"提到会议室但 `meeting_room_id` 仍为空"，就禁止调用 create——先回到步骤 4 完成查询 / 选择拿到 ID。严禁以"先把会议建起来、忙闲/会议室随后补"的方式跳过本门禁。
6. **查询详情并展示**：使用返回的 `meeting_id` 调用 `meeting get`（见 [meeting-list](meeting-list.md)）获取 `subject`、`begin_time`/`end_time`、`attendees[].name`。**创建成功后输出内容只包含三部分：主题、时间、参会人**，禁止输出其他任何内容和额外语句（不展示地点、会议室、会议号、入会链接、meeting_id 等字段，也不附加说明、建议或寒暄）；参会人原样取接口返回的 `attendees[].name` 展示（完全与接口返回的格式保持一致，如返回 `zhangsan(张三)` 就展示 `zhangsan(张三)`），禁止暴露 userid。

### 异常路径

| 异常情况 | 处理方式 |
|---------|---------|
| `begin_time` 早于当前时间 | 提示用户时间已过, 请重新选择未来时间 |
| `end_time` 与 `begin_time` 间隔超过 24 小时 | 提示用户会议时长不可超过 24 小时, 请调整；禁止自行拆分成多场会议 |
| 参会人数超过 300 | 提示用户参会人数已达上限, 需减少人数 |
| 用户要"每月第 N 个星期几 / 每年"重复 | `repeat_type` 不支持该重复方式，告知用户并引导其在企业微信客户端手动预订；禁止用 `monthly` 等近似替代或批量创建单次会议 |
| `repeat_time` 超上限（>200，或 `monthly`/每两周 >50） | 告知实际上限，请用户确认调整后的场次或改用 `repeat_until`，禁止静默截断 |
| `repeat_interval` 超上限（daily>99 / weekly>12 / monthly>12） | 告知实际上限并请用户调整间隔 |
| `repeat_until` 早于会议开始时间或超过约 49 年 | 提示结束时刻必须晚于会议开始时间且不能过远，请重新给定 |
| wecomcli-contact 技能搜索无结果 | 用文字提示用户重新输入：`未找到「{姓名}」，请确认姓名是否正确` |
| wecomcli-contact 技能返回多个候选人 | 用文字询问用户（列出候选姓名 + 部门），等待用户选择后汇总继续 |
| 创建接口返回错误 | 检查参数格式, 重新阅读本文档确认用法 |
| `meeting_room_taken`（会议室被抢占） | 查询通过后、create 前被他人占走。用文字告知"{会议室名} 刚被占用"，让用户在「换会议室 / 换时间」二选一；换会议室则重走步骤 4 的 `rooms search`，禁止静默重试同一会议室 |
| `meeting_room_not_found`（会议室无效） | `meeting_room_id` 不存在或上下文过期，重新走步骤 4 的会议室查询 |
| 换会议室需求 | 走 [meeting-update](meeting-update.md) 传入新 `meeting_room_id` 改订（须先经 `rooms search` 确认新会议室 `status=bookable`），无需取消重建 |

## 典型场景

### 1. 信息完整（正常路径）

```
用户：帮我约一个明天下午两点和张三的项目对齐会，30 分钟
→ 通过 wecomcli-contact 技能搜索「张三」→ 返回 2 个候选
→ 用文字询问：搜索到多个「张三」，请确认要邀请哪一位？（列出：张三 - 产品部 - 产品经理 / 张三 - 技术部 - 前端工程师）
→ 用户选择后，获得对应 userid
→ attendees 含他人内部成员（张三）→ 忙闲检查：用 [begin_time, end_time] 调 free list（自己 + 张三）；无冲突则继续，占线则用文字让用户「坚持这个时间 / 换一个时间」
→ 调用 meeting create（subject="项目对齐", begin_time="<明天日期> 14:00:00", end_time="<明天日期> 14:30:00", attendees=[{"userid": "userid1"}]）
→ 调用 meeting get 获取主题、时间、参会人姓名
→ 只展示三部分：主题、时间、参会人
```

### 2. 参数缺失（逐步补全）

```
用户：帮我开个会
→ 未明确是日程还是会议 → 用文字追问（请回复：日程 / 会议）
→ 用户答「会议」→ 留在本技能继续；若答「日程」→ 改用 wecomcli-calendar 技能
→ subject 缺失 → 用文字询问会议主题
→ begin_time 缺失 → 用文字询问开始时间（结合当前时间动态推荐候选项）
→ end_time 缺失 → 默认时长 1 小时（不追问），按 begin_time + 1h 推算
→ attendees 缺失 → 用文字追问参会人（地点、会议室等非必填项不追问）
→ attendees 含他人内部成员 → 忙闲检查（free list，占线则用文字问坚持/换时间）
→ 参数就绪 → 调用 meeting create → 展示结果
```

### 3. 通讯录多候选人

```
用户：帮我约张三、李四参加明天 10 点的需求评审，1 小时
→ 通过 wecomcli-contact 技能批量搜索「张三」「李四」
→ "张三" 返回 2 条（产品部 / 技术部）→ 用文字让用户选择
→ "李四" 返回 1 条 → 直接使用，无需确认
→ 汇总 userid → 忙闲检查：调 free list（自己 + 张三 + 李四）查 [begin_time, end_time]；占线则用文字让用户「坚持这个时间 / 换一个时间」 → 调用 meeting create → 展示结果
```

## 示例请求

**单次会议**（不传 `recurrence`）：

```json
{
  "subject": "产品需求评审",
  "begin_time": "<明天日期> 14:00:00",
  "end_time": "<明天日期> 15:00:00",
  "attendees": [
    {"userid": "userid1"},
    {"userid": "userid2"},
    {"userid": "userid3"}
  ],
  "description": "评审Q2需求文档",
  "meeting_room_id": "mrmxxxx",
  "timezone": {
    "timezone_id": "Asia/Shanghai",
    "timezone_offset": 28800
  }
}
```

**周期会议**（每周二、四，共 8 场；`begin_time` 已对齐到周二）：

```json
{
  "subject": "项目周会",
  "begin_time": "<最近的周二日期> 14:00:00",
  "end_time": "<最近的周二日期> 15:00:00",
  "attendees": [
    {"userid": "userid1"},
    {"userid": "userid2"}
  ],
  "mark_optional_attendees": ["userid2"],
  "recurrence": {
    "is_repeat": true,
    "repeat_type": "weekly",
    "repeat_day_of_week": ["TU", "TH"],
    "repeat_time": 8
  }
}
```

**周期会议**（每两周一次，到指定时刻结束）：

```json
{
  "subject": "双周复盘",
  "begin_time": "<最近的周三日期> 16:00:00",
  "end_time": "<最近的周三日期> 17:00:00",
  "attendees": [{"userid": "userid1"}],
  "recurrence": {
    "is_repeat": true,
    "repeat_type": "weekly",
    "repeat_interval": 2,
    "is_custom": true,
    "repeat_day_of_week": ["WE"],
    "repeat_until": "2026-12-31 23:59:59"
  }
}
```

> `meeting_room_id` 为可选，订会议室时才传，来自 `读取 wecomcli-calendar 技能` 的会议室查询（`rooms search`，见 [会议室查询参考](../../wecomcli-calendar/references/calendar-meeting-room.md)）；订会议室时只传 `meeting_room_id` 即可，无需再把会议室名重复填进 `location`（会议室名由后端关联返回）。`location`仅在用户给的是非会议室的普通文本地点时才填。
>
> `recurrence` 不传或 `is_repeat=false` 即单次会议；`repeat_time` 与 `repeat_until` 只需给一个；`repeat_interval` 必须搭配 `is_custom=true`；`mark_optional_attendees` 是平铺字符串数组，其成员须已在 `attendees` 中。

## 示例输出

> 创建成功后只输出三部分：主题、时间、参会人，禁止输出地点、会议号、入会链接等其他内容和额外语句。

```
主题：产品需求评审
时间：<明天日期> 14:00:00 - 15:00:00
参会人：张三、李四
```
