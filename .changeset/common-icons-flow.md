---
'@wecom/cli': patch
'@wecom/cli-darwin-arm64': patch
'@wecom/cli-darwin-x64': patch
'@wecom/cli-linux-arm64': patch
'@wecom/cli-linux-x64': patch
'@wecom/cli-win32-x64': patch
---

写入/同步失败时回退半成品文件

- `--output` 指定的文件在首页请求前即预留创建；首页请求失败时回退删除该预留文件，不再留下 0 字节空壳。
- 分页输出（`--page-count`）首页写入成功后发生错误的，已写入的 NDJSON 内容保留在文件中。
- 二进制下载流中断（写入或落盘同步失败）时回退删除半成品文件；完整下载的结果（包括服务端合法返回的 0 字节空文件）正常保留。
