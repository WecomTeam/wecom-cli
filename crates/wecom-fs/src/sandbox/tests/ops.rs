//! ## 模块摘要：sandbox 异步 I/O 操作（经 Fs 门面与 inherent 跳板）
//!
//! ### 关键接口
//! - [SandboxedFs::atomic_write] — 原子写入：先写临时文件，再 rename 到目标路径
//! - [SandboxedFs::create_file] / [Fs::create_file_unique] — 沙箱内创建文件
//! - [Fs::read_to_string] / [SandboxedFs::metadata] — 读取内容与元信息（fd 级断言）
//! - [SandboxedFs::list_dir] — 列出目录下所有条目（非递归）
//! - [SandboxedFs::open_for_read] — 流式读取句柄
//! - [SandboxedFs::check_readable] / [SandboxedFs::check_writable] — 方向校验（内部经 blocking 池派发）
//!
//! ### 关键分支与异常路径
//! - 路径逃逸 writable/readable roots → Err("目标路径超出可访问范围")
//! - 已存在文件 create_file → Err(AlreadyExists)
//! - create_file_unique 冲突 → 自动追加随机后缀重试
//! - None roots（无限制模式）→ 跳过 roots 校验（denylist 仍生效）
//! - 分离的 readable/writable roots → 读写操作分别校验不同 root 列表
//!
//! ### 上下游交互
//! - 上游：Fs trait 实现与直接构造的 [SandboxedFs] 实例
//! - 下游：sandbox::io（根句柄 I/O）、sandbox::policy（判定）

use std::fs as stdfs;
use std::path::Path;

use tempfile::TempDir;

use super::{fs_with_read_roots, fs_with_roots, fs_with_write_roots};
use crate::api::Fs;
use crate::sandbox::*;

// ── atomic_write ──

/// P1：[SandboxedFs::atomic_write] 拒绝写入 roots 之外的路径
/// 条件：目标文件位于 forbidden 目录（不在 writable roots 内）
/// 断言：返回 Err，错误信息包含 "目标路径超出可访问范围"，目标文件未被创建
#[tokio::test]
async fn atomic_write_rejects_path_outside_roots() {
    let allowed = TempDir::new().unwrap();
    let forbidden = TempDir::new().unwrap();
    let target = forbidden.path().join("escaped.txt");

    let fs = fs_with_roots(&[allowed.path()]);
    let result = fs.atomic_write(&target, b"bad", 0o600).await;

    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(msg.contains("目标路径超出可访问范围"), "msg = {msg}");
    assert!(!target.exists());
}

/// P0：[SandboxedFs::atomic_write] 允许写入 roots 内的路径
/// 条件：目标文件位于 writable roots 内
/// 断言：写入成功，文件内容为 "ok"
#[tokio::test]
async fn atomic_write_allows_path_within_roots() {
    let tmp = TempDir::new().unwrap();
    let target = tmp.path().join("allowed.txt");

    let fs = fs_with_roots(&[tmp.path()]);
    fs.atomic_write(&target, b"ok", 0o600).await.unwrap();

    assert_eq!(stdfs::read_to_string(&target).unwrap(), "ok");
}

/// P1：[SandboxedFs::atomic_write] 相对路径被拒绝（调用方须在入口锚定为绝对路径）
/// 条件：传入相对路径 "rel.txt"
/// 断言：返回 Err（validation），文件未创建
#[tokio::test]
async fn atomic_write_rejects_relative_path() {
    let tmp = TempDir::new().unwrap();
    let fs = fs_with_roots(&[tmp.path()]);
    assert!(
        fs.atomic_write(Path::new("rel.txt"), b"data", 0o600)
            .await
            .is_err()
    );
    assert!(!tmp.path().join("rel.txt").exists());
}

/// P2：[SandboxedFs::atomic_write] 覆盖已有文件
/// 条件：目标路径已存在一个旧文件
/// 断言：写入成功，文件内容被更新为新值
#[tokio::test]
async fn atomic_write_overwrites_existing_file() {
    let tmp = TempDir::new().unwrap();
    let target = tmp.path().join("overwrite.txt");
    stdfs::write(&target, "old").unwrap();

    let fs = fs_with_roots(&[tmp.path()]);
    fs.atomic_write(&target, b"new", 0o600).await.unwrap();
    assert_eq!(stdfs::read_to_string(&target).unwrap(), "new");
}

// ── create_file ──

/// P0：[SandboxedFs::create_file] 正常创建文件
/// 条件：路径在 roots 内，目标文件不存在
/// 断言：文件存在，文件名为 "output.txt"
#[tokio::test]
async fn create_file_success() {
    let tmp = TempDir::new().unwrap();
    let fs = fs_with_roots(&[tmp.path()]);
    let (path, _file) = fs
        .create_file(&tmp.path().join("output.txt"))
        .await
        .unwrap();
    assert!(path.exists());
    assert_eq!(path.file_name().unwrap(), "output.txt");
}

/// P0：[SandboxedFs::create_file] 自动创建多级父目录
/// 条件：路径含深层嵌套 "deep/nested/file.txt"
/// 断言：文件创建成功且存在
#[tokio::test]
async fn create_file_creates_parent_dirs() {
    let tmp = TempDir::new().unwrap();
    let fs = fs_with_roots(&[tmp.path()]);
    let (path, _file) = fs
        .create_file(&tmp.path().join("deep/nested/file.txt"))
        .await
        .unwrap();
    assert!(path.exists());
}

/// P2：[SandboxedFs::create_file] 自动创建的中间目录权限收紧为 0o700
/// 条件：路径含两级新建父目录
/// 断言：两级目录的 mode 均为 0o700（与根句柄建根时一致，不吃 umask 默认 0o777）
#[cfg(unix)]
#[tokio::test]
async fn create_file_creates_parent_dirs_with_owner_only_mode() {
    use std::os::unix::fs::PermissionsExt;

    let tmp = TempDir::new().unwrap();
    let fs = fs_with_roots(&[tmp.path()]);
    fs.create_file(&tmp.path().join("deep/nested/file.txt"))
        .await
        .unwrap();
    for dir in ["deep", "deep/nested"] {
        let mode = stdfs::metadata(tmp.path().join(dir))
            .unwrap()
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, 0o700, "{dir} mode = {mode:o}");
    }
}

/// P1：[SandboxedFs::create_file] 拒绝在 roots 之外创建文件
/// 条件：目标路径在 forbidden 目录
/// 断言：返回 Err
#[tokio::test]
async fn create_file_rejects_outside_roots() {
    let allowed = TempDir::new().unwrap();
    let forbidden = TempDir::new().unwrap();
    let fs = fs_with_roots(&[allowed.path()]);
    let result = fs.create_file(&forbidden.path().join("escape.txt")).await;
    assert!(result.is_err());
}

/// P1：[SandboxedFs::create_file] 目标文件已存在时返回错误（create_new 语义）
/// 条件：目标文件已被预先创建
/// 断言：返回 Err
#[tokio::test]
async fn create_file_already_exists_returns_err() {
    let tmp = TempDir::new().unwrap();
    stdfs::write(tmp.path().join("exists.txt"), "data").unwrap();
    let fs = fs_with_roots(&[tmp.path()]);
    let result = fs.create_file(&tmp.path().join("exists.txt")).await;
    assert!(result.is_err());
}

// ── create_file_unique（Fs trait 默认实现）──

/// P0：[Fs::create_file_unique] 无冲突时使用原名创建
/// 条件：目标文件不存在
/// 断言：文件存在，文件名为 "report.json"
#[tokio::test]
async fn create_file_unique_no_collision() {
    let tmp = TempDir::new().unwrap();
    let fs = fs_with_roots(&[tmp.path()]);
    let (path, _) = fs
        .create_file_unique(&tmp.path().join("report.json"))
        .await
        .unwrap();
    assert!(path.exists());
    assert_eq!(path.file_name().unwrap(), "report.json");
}

/// P0：[Fs::create_file_unique] 有冲突时自动加随机后缀（保留原始扩展名）
/// 条件：同名文件 "report.json" 已存在
/// 断言：新文件名 ≠ "report.json"，但包含 "report." 和 ".json"
#[tokio::test]
async fn create_file_unique_with_collision() {
    let tmp = TempDir::new().unwrap();
    stdfs::write(tmp.path().join("report.json"), "existing").unwrap();

    let fs = fs_with_roots(&[tmp.path()]);
    let (path, _) = fs
        .create_file_unique(&tmp.path().join("report.json"))
        .await
        .unwrap();
    assert!(path.exists());
    assert_ne!(path.file_name().unwrap(), "report.json");
    assert!(path.to_string_lossy().contains("report."));
    assert!(path.to_string_lossy().contains(".json"));
}

/// P1：[Fs::create_file_unique] 无扩展名文件冲突时加随机后缀，不产生多余点号
/// 条件：同名无扩展名文件 "noext" 已存在
/// 断言：新文件名以 "noext." 开头且不含第二个点
#[tokio::test]
async fn create_file_unique_no_extension_with_collision() {
    let tmp = TempDir::new().unwrap();
    // Pre-create file without extension to trigger `None` ext branch
    stdfs::write(tmp.path().join("noext"), "existing").unwrap();

    let fs = fs_with_roots(&[tmp.path()]);
    let (path, _) = fs
        .create_file_unique(&tmp.path().join("noext"))
        .await
        .unwrap();
    assert!(path.exists());
    assert_ne!(path.file_name().unwrap(), "noext");
    // Should be "noext.<random>" with no trailing extension
    let name = path.file_name().unwrap().to_string_lossy();
    assert!(name.starts_with("noext."), "name = {name}");
    let after_stem = name.strip_prefix("noext.").unwrap();
    assert!(
        !after_stem.contains('.'),
        "no-extension file should not have extra dot: {name}"
    );
}

/// P1：[Fs::create_file_unique] roots 之外拒绝创建
/// 条件：目标路径在 forbidden 目录
/// 断言：返回 Err
#[tokio::test]
async fn create_file_unique_rejects_outside_roots() {
    let allowed = TempDir::new().unwrap();
    let forbidden = TempDir::new().unwrap();
    let fs = fs_with_roots(&[allowed.path()]);
    let result = fs
        .create_file_unique(&forbidden.path().join("escape.txt"))
        .await;
    assert!(result.is_err());
}

// ── read_to_string（Fs trait 默认实现）──

/// P0：[Fs::read_to_string] 正常读取文件内容
/// 条件：文件存在且在 readable roots 内
/// 断言：返回 "hello world"
#[tokio::test]
async fn read_to_string_success() {
    let tmp = TempDir::new().unwrap();
    let file = tmp.path().join("data.txt");
    stdfs::write(&file, "hello world").unwrap();

    let fs = fs_with_roots(&[tmp.path()]);
    assert_eq!(fs.read_to_string(&file).await.unwrap(), "hello world");
}

/// P1：[Fs::read_to_string] 拒绝读取 roots 之外的文件
/// 条件：文件在 forbidden 目录
/// 断言：返回 Err
#[tokio::test]
async fn read_to_string_rejects_outside_roots() {
    let allowed = TempDir::new().unwrap();
    let forbidden = TempDir::new().unwrap();
    let file = forbidden.path().join("secret.txt");
    stdfs::write(&file, "secret").unwrap();

    let fs = fs_with_roots(&[allowed.path()]);
    assert!(fs.read_to_string(&file).await.is_err());
}

/// P1：[Fs::read_to_string] 读取不存在的文件返回 I/O 错误
/// 条件：文件不存在但路径在 roots 内
/// 断言：返回 Err，错误信息包含 "Failed to open"
#[tokio::test]
async fn read_to_string_nonexistent_returns_io_err() {
    let tmp = TempDir::new().unwrap();
    let fs = fs_with_roots(&[tmp.path()]);
    let result = fs.read_to_string(&tmp.path().join("missing.txt")).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Failed to open"));
}

/// P2：[Fs::read_to_string] 读取非 UTF-8 文件返回 I/O 错误
/// 条件：文件内容为无效 UTF-8 字节序列（0xFF 0xFE）
/// 断言：返回 Err，错误信息包含 "Failed to read"
#[tokio::test]
async fn read_to_string_invalid_utf8_returns_io_err() {
    let tmp = TempDir::new().unwrap();
    let file = tmp.path().join("binary.bin");
    stdfs::write(&file, [0xFF, 0xFE, 0x00, 0x01]).unwrap();

    let fs = fs_with_roots(&[tmp.path()]);
    let result = fs.read_to_string(&file).await;
    assert!(result.is_err());
    assert!(
        result.unwrap_err().to_string().contains("Failed to read"),
        "non-UTF-8 content should produce 'Failed to read' error"
    );
}

/// P1：[Fs::read_to_string] 相对路径被拒绝（调用方须在入口锚定为绝对路径）
/// 条件：传入相对路径 "rel-read.txt"，文件在 roots 内已存在
/// 断言：返回 Err（validation）
#[tokio::test]
async fn read_to_string_rejects_relative_path() {
    let tmp = TempDir::new().unwrap();
    stdfs::write(tmp.path().join("rel-read.txt"), "relative").unwrap();

    let fs = fs_with_roots(&[tmp.path()]);
    assert!(fs.read_to_string(Path::new("rel-read.txt")).await.is_err());
}

// ── metadata ──

/// P0：[SandboxedFs::metadata] 正常获取文件元信息
/// 条件：5 字节文件在 roots 内
/// 断言：len() == 5，is_file() == true
#[tokio::test]
async fn metadata_returns_file_info() {
    let tmp = TempDir::new().unwrap();
    let file = tmp.path().join("test.bin");
    stdfs::write(&file, "12345").unwrap();

    let fs = fs_with_roots(&[tmp.path()]);
    let meta = fs.metadata(&file).await.unwrap();
    assert_eq!(meta.len(), 5);
    assert!(meta.is_file());
}

/// P1：[SandboxedFs::metadata] 拒绝获取 roots 之外的文件元信息
/// 条件：文件在 forbidden 目录
/// 断言：返回 Err
#[tokio::test]
async fn metadata_rejects_outside_roots() {
    let allowed = TempDir::new().unwrap();
    let forbidden = TempDir::new().unwrap();
    let file = forbidden.path().join("secret.txt");
    stdfs::write(&file, "data").unwrap();

    let fs = fs_with_roots(&[allowed.path()]);
    assert!(fs.metadata(&file).await.is_err());
}

/// P1：[SandboxedFs::metadata] 不存在的文件返回 I/O 错误
/// 条件：文件不存在但路径在 roots 内
/// 断言：返回 Err，错误信息包含 "Failed to stat" 或 "Failed to open"
#[tokio::test]
async fn metadata_nonexistent_returns_io_err() {
    let tmp = TempDir::new().unwrap();
    let fs = fs_with_roots(&[tmp.path()]);
    let result = fs.metadata(&tmp.path().join("missing")).await;
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("Failed to stat") || msg.contains("Failed to open"),
        "msg = {msg}"
    );
}

// ── list_dir ──

/// P0：[SandboxedFs::list_dir] 返回所有条目，调用方可过滤为仅文件
/// 条件：目录下有 2 个文件和 1 个子目录
/// 断言：list_dir 返回 3 个条目；过滤 is_file 后得 2 个
#[tokio::test]
async fn list_dir_returns_all_entries() {
    let tmp = TempDir::new().unwrap();
    stdfs::write(tmp.path().join("a.txt"), "a").unwrap();
    stdfs::write(tmp.path().join("b.txt"), "b").unwrap();
    stdfs::create_dir(tmp.path().join("subdir")).unwrap();

    let fs = fs_with_roots(&[tmp.path()]);
    let entries = fs.list_dir(tmp.path()).await.unwrap();
    // list_dir returns ALL entries (files + dirs), no filtering.
    assert_eq!(entries.len(), 3);
    assert_eq!(entries.iter().filter(|e| e.is_file).count(), 2);
    assert_eq!(
        entries.iter().filter(|e| e.is_dir).count(),
        1,
        "list_dir must return subdirectory entries"
    );
}

/// P1：[SandboxedFs::list_dir] 空目录返回空列表
/// 条件：目录存在但为空
/// 断言：entries.is_empty()
#[tokio::test]
async fn list_dir_empty_dir() {
    let tmp = TempDir::new().unwrap();
    let fs = fs_with_roots(&[tmp.path()]);
    let entries = fs.list_dir(tmp.path()).await.unwrap();
    assert!(entries.is_empty());
}

/// P1：[SandboxedFs::list_dir] 拒绝列出 roots 之外的目录
/// 条件：目录在 forbidden 目录
/// 断言：返回 Err
#[tokio::test]
async fn list_dir_rejects_outside_roots() {
    let allowed = TempDir::new().unwrap();
    let forbidden = TempDir::new().unwrap();
    let fs = fs_with_roots(&[allowed.path()]);
    assert!(fs.list_dir(forbidden.path()).await.is_err());
}

/// P1：[SandboxedFs::list_dir] 不存在的目录返回 I/O 错误
/// 条件：目录不存在但路径在 roots 内
/// 断言：返回 Err，错误信息包含 "Failed to read directory"
#[tokio::test]
async fn list_dir_nonexistent_dir_returns_io_err() {
    let tmp = TempDir::new().unwrap();
    let fs = fs_with_roots(&[tmp.path()]);
    let result = fs.list_dir(&tmp.path().join("no-dir")).await;
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("Failed to read directory")
    );
}

// ── open_for_read ──

/// P1：[SandboxedFs::open_for_read] 成功打开沙盒内文件
/// 条件：文件在 readable roots 内，调用 open_for_read()
/// 断言：返回 Ok，含 resolved path 和可读取的 File
#[tokio::test]
async fn open_for_read_success() {
    use tokio::io::AsyncReadExt;
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("stream.txt");
    stdfs::write(&file, "streaming content").unwrap();

    let fs = fs_with_roots(&[dir.path()]);
    let (resolved, mut tokio_file) = fs.open_for_read(&file).await.unwrap();
    assert!(resolved.ends_with("stream.txt"));

    let mut buf = String::new();
    tokio_file.read_to_string(&mut buf).await.unwrap();
    assert_eq!(buf, "streaming content");
}

/// P1：[SandboxedFs::open_for_read] 拒绝打开 roots 外文件
/// 条件：文件在 readable roots 外
/// 断言：返回 Err
#[tokio::test]
async fn open_for_read_rejects_outside_roots() {
    let allowed = TempDir::new().unwrap();
    let forbidden = TempDir::new().unwrap();
    let file = forbidden.path().join("secret.txt");
    stdfs::write(&file, "secret").unwrap();

    let fs = fs_with_roots(&[allowed.path()]);
    assert!(fs.open_for_read(&file).await.is_err());
}

// ══════════════════════════════════════════════════════════════
//  Tests — 读写方向各自的 Policy（读写 roots 分离）
// ══════════════════════════════════════════════════════════════

/// P0：[Fs::read_to_string] 只读 root 允许读取文件
/// 条件：readable 含 read_dir，writable 不含
/// 断言：读取成功返回 "hello"
#[tokio::test]
async fn read_only_root_allows_read_to_string() {
    let read_dir = TempDir::new().unwrap();
    let write_dir = TempDir::new().unwrap();
    let file = read_dir.path().join("data.txt");
    stdfs::write(&file, "hello").unwrap();

    let fs = SandboxedFs::new()
        .with_read_policy(Policy::new().with_allowed_dirs(&[read_dir.path()]))
        .with_write_policy(Policy::new().with_allowed_dirs(&[write_dir.path()]));
    assert_eq!(fs.read_to_string(&file).await.unwrap(), "hello");
}

/// P1：[SandboxedFs::create_file] 只读 root 阻止创建文件
/// 条件：writable 不含 read_dir
/// 断言：返回 Err，错误信息包含 "目标路径超出可访问范围"
#[tokio::test]
async fn read_only_root_blocks_create_file() {
    let read_dir = TempDir::new().unwrap();
    let write_dir = TempDir::new().unwrap();

    let fs = SandboxedFs::new()
        .with_read_policy(Policy::new().with_allowed_dirs(&[read_dir.path()]))
        .with_write_policy(Policy::new().with_allowed_dirs(&[write_dir.path()]));

    let result = fs.create_file(&read_dir.path().join("forbidden.txt")).await;
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(msg.contains("目标路径超出可访问范围"), "msg = {msg}");
}

/// P0：[SandboxedFs::create_file] 只写 root 允许创建文件
/// 条件：writable 含 write_dir，readable 不含
/// 断言：创建成功返回 Ok
#[tokio::test]
async fn write_only_root_allows_create_file() {
    let read_dir = TempDir::new().unwrap();
    let write_dir = TempDir::new().unwrap();

    let fs = SandboxedFs::new()
        .with_read_policy(Policy::new().with_allowed_dirs(&[read_dir.path()]))
        .with_write_policy(Policy::new().with_allowed_dirs(&[write_dir.path()]));

    let result = fs.create_file(&write_dir.path().join("ok.txt")).await;
    assert!(result.is_ok());
}

/// P1：[Fs::read_to_string] 只写 root 阻止读取文件
/// 条件：readable 不含 write_dir，文件已存在
/// 断言：返回 Err，错误信息包含 "目标路径超出可访问范围"
#[tokio::test]
async fn write_only_root_blocks_read_to_string() {
    let read_dir = TempDir::new().unwrap();
    let write_dir = TempDir::new().unwrap();
    let file = write_dir.path().join("secret.txt");
    stdfs::write(&file, "data").unwrap();

    let fs = SandboxedFs::new()
        .with_read_policy(Policy::new().with_allowed_dirs(&[read_dir.path()]))
        .with_write_policy(Policy::new().with_allowed_dirs(&[write_dir.path()]));

    let result = fs.read_to_string(&file).await;
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(msg.contains("目标路径超出可访问范围"), "msg = {msg}");
}

/// P1：[SandboxedFs::metadata] 只写 root 阻止获取元数据
/// 条件：readable 不含 write_dir
/// 断言：返回 Err
#[tokio::test]
async fn write_only_root_blocks_metadata() {
    let read_dir = TempDir::new().unwrap();
    let write_dir = TempDir::new().unwrap();
    let file = write_dir.path().join("meta.txt");
    stdfs::write(&file, "data").unwrap();

    let fs = SandboxedFs::new()
        .with_read_policy(Policy::new().with_allowed_dirs(&[read_dir.path()]))
        .with_write_policy(Policy::new().with_allowed_dirs(&[write_dir.path()]));

    assert!(fs.metadata(&file).await.is_err());
}

/// P1：[SandboxedFs::list_dir] 只写 root 阻止列出目录
/// 条件：readable 不含 write_dir
/// 断言：返回 Err
#[tokio::test]
async fn write_only_root_blocks_list_dir() {
    let read_dir = TempDir::new().unwrap();
    let write_dir = TempDir::new().unwrap();
    stdfs::write(write_dir.path().join("a.txt"), "a").unwrap();

    let fs = SandboxedFs::new()
        .with_read_policy(Policy::new().with_allowed_dirs(&[read_dir.path()]))
        .with_write_policy(Policy::new().with_allowed_dirs(&[write_dir.path()]));

    assert!(fs.list_dir(write_dir.path()).await.is_err());
}

/// P1：[SandboxedFs::atomic_write] 只读 root 阻止原子写入
/// 条件：writable 不含 read_dir
/// 断言：返回 Err 且目标文件未创建
#[tokio::test]
async fn read_only_root_blocks_atomic_write() {
    let read_dir = TempDir::new().unwrap();
    let write_dir = TempDir::new().unwrap();
    let target = read_dir.path().join("no-write.txt");

    let fs = SandboxedFs::new()
        .with_read_policy(Policy::new().with_allowed_dirs(&[read_dir.path()]))
        .with_write_policy(Policy::new().with_allowed_dirs(&[write_dir.path()]));

    let result = fs.atomic_write(&target, b"bad", 0o600).await;
    assert!(result.is_err());
    assert!(!target.exists());
}

/// P1：[SandboxedFs::atomic_write] 可写 root 允许原子写入
/// 条件：readable + writable 均含 write_dir
/// 断言：写入成功，内容为 "data"
#[tokio::test]
async fn writable_root_allows_atomic_write() {
    let write_dir = TempDir::new().unwrap();
    let target = write_dir.path().join("ok.txt");

    let fs = fs_with_roots(&[write_dir.path()]);

    fs.atomic_write(&target, b"data", 0o600).await.unwrap();
    assert_eq!(stdfs::read_to_string(&target).unwrap(), "data");
}

/// P0：[SandboxedFs::with_policy] 同时在读写 roots 内可完整读写
/// 条件：同一 Policy 同时用于读写两个方向
/// 断言：读和写操作均成功
#[tokio::test]
async fn both_roots_allow_full_access() {
    let dir = TempDir::new().unwrap();
    let fs = fs_with_roots(&[dir.path()]);

    // Read
    let file = dir.path().join("rw.txt");
    stdfs::write(&file, "hello").unwrap();
    assert_eq!(fs.read_to_string(&file).await.unwrap(), "hello");

    // Write
    let new_file = dir.path().join("created.txt");
    let (path, _) = fs.create_file(&new_file).await.unwrap();
    assert!(path.exists());
}

// ══════════════════════════════════════════════════════════════
//  Tests — unrestricted SandboxedFs（None roots）
// ══════════════════════════════════════════════════════════════

/// P0：[Fs::read_to_string] 无限制模式可读取任意位置文件
/// 条件：SandboxedFs::new()，文件在其他临时目录
/// 断言：读取成功返回 "hello"
#[tokio::test]
async fn unrestricted_fs_allows_read_anywhere() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("data.txt");
    stdfs::write(&file, "hello").unwrap();

    let fs = SandboxedFs::new();
    assert_eq!(fs.read_to_string(&file).await.unwrap(), "hello");
}

/// P0：[SandboxedFs::create_file] 无限制模式可创建文件
/// 条件：SandboxedFs::new()
/// 断言：创建成功
#[tokio::test]
async fn unrestricted_fs_allows_create_file_anywhere() {
    let dir = TempDir::new().unwrap();
    let fs = SandboxedFs::new();
    let result = fs.create_file(&dir.path().join("unrestricted.txt")).await;
    assert!(result.is_ok());
}

/// P0：[SandboxedFs::atomic_write] 无限制模式可原子写入
/// 条件：SandboxedFs::new()
/// 断言：写入成功，内容正确
#[tokio::test]
async fn unrestricted_fs_allows_atomic_write_anywhere() {
    let dir = TempDir::new().unwrap();
    let target = dir.path().join("anywhere.txt");
    let fs = SandboxedFs::new();
    fs.atomic_write(&target, b"data", 0o600).await.unwrap();
    assert_eq!(stdfs::read_to_string(&target).unwrap(), "data");
}

/// P1：[SandboxedFs::list_dir] 无限制模式下允许列出目录
/// 条件：SandboxedFs::new 创建无限制实例
/// 断言：返回正确的文件列表
#[tokio::test]
async fn unrestricted_fs_allows_list_dir() {
    let dir = TempDir::new().unwrap();
    stdfs::write(dir.path().join("a.txt"), "a").unwrap();
    stdfs::write(dir.path().join("b.txt"), "b").unwrap();

    let fs = SandboxedFs::new();
    let files: Vec<_> = fs
        .list_dir(dir.path())
        .await
        .unwrap()
        .into_iter()
        .filter(|e| e.is_file)
        .collect();
    assert_eq!(files.len(), 2);
}

/// P1：[SandboxedFs::metadata] 无限制模式下允许获取元数据
/// 条件：SandboxedFs::new 创建无限制实例
/// 断言：获取成功，文件大小正确
#[tokio::test]
async fn unrestricted_fs_allows_metadata() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("info.txt");
    stdfs::write(&file, "hello").unwrap();

    let fs = SandboxedFs::new();
    let meta = fs.metadata(&file).await.unwrap();
    assert_eq!(meta.len(), 5);
}

/// P1：[Fs::read_to_string] 未配置读方向 roots 时不限制读操作
/// 条件：仅注入写方向 Policy（读方向保持无限制）
/// 断言：读取成功返回 "hello"
#[tokio::test]
async fn none_readable_roots_allows_read() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("data.txt");
    stdfs::write(&file, "hello").unwrap();

    let fs = fs_with_write_roots(&[dir.path()]);
    assert_eq!(fs.read_to_string(&file).await.unwrap(), "hello");
}

/// P1：[SandboxedFs::create_file] 未配置写方向 roots 时不限制写操作
/// 条件：仅注入读方向 Policy（写方向保持无限制）
/// 断言：创建成功
#[tokio::test]
async fn none_writable_roots_allows_write() {
    let dir = TempDir::new().unwrap();
    let fs = fs_with_read_roots(&[dir.path()]);
    let result = fs
        .create_file(&dir.path().join("unrestricted-write.txt"))
        .await;
    assert!(result.is_ok());
}

// ══════════════════════════════════════════════════════════════
//  Tests — 方向校验入口
// ══════════════════════════════════════════════════════════════

/// P1：[SandboxedFs::check_readable] 允许 roots 内的路径
/// 条件：路径在 readable roots 内
/// 断言：返回 Ok，返回的路径以原始文件名结尾
#[tokio::test]
async fn check_readable_allows_path_within_roots() {
    let tmp = TempDir::new().unwrap();
    let file = tmp.path().join("visible.txt");
    stdfs::write(&file, "data").unwrap();

    let fs = fs_with_roots(&[tmp.path()]);
    let result = fs.check_readable(&file).await;
    assert!(result.is_ok());
    assert!(result.unwrap().ends_with("visible.txt"));
}

/// P1：[SandboxedFs::check_readable] 拒绝 roots 之外的路径
/// 条件：路径在 forbidden 目录
/// 断言：返回 Err
#[tokio::test]
async fn check_readable_rejects_outside_roots() {
    let allowed = TempDir::new().unwrap();
    let forbidden = TempDir::new().unwrap();

    let fs = fs_with_roots(&[allowed.path()]);
    assert!(
        fs.check_readable(forbidden.path().join("secret.txt"))
            .await
            .is_err()
    );
}

/// P1：[SandboxedFs::check_writable] 允许 roots 内的路径
/// 条件：路径在 writable roots 内
/// 断言：返回 Ok
#[tokio::test]
async fn check_writable_allows_path_within_roots() {
    let tmp = TempDir::new().unwrap();
    let fs = fs_with_roots(&[tmp.path()]);
    assert!(
        fs.check_writable(tmp.path().join("new-file.txt"))
            .await
            .is_ok()
    );
}

/// P1：[SandboxedFs::check_writable] 拒绝 roots 之外的路径
/// 条件：路径在 forbidden 目录
/// 断言：返回 Err
#[tokio::test]
async fn check_writable_rejects_outside_roots() {
    let allowed = TempDir::new().unwrap();
    let forbidden = TempDir::new().unwrap();

    let fs = fs_with_roots(&[allowed.path()]);
    assert!(
        fs.check_writable(forbidden.path().join("escape.txt"))
            .await
            .is_err()
    );
}

/// P1：[SandboxedFs::check_readable] 相对路径被拒绝（调用方须在入口锚定为绝对路径）
/// 条件：传入相对路径 "rel.txt"
/// 断言：返回 Err（validation）
#[tokio::test]
async fn check_readable_rejects_relative_path() {
    let fs = SandboxedFs::new();
    assert!(fs.check_readable(Path::new("rel.txt")).await.is_err());
}

// ── FsWrite::discard ──

/// P0：[FsWrite::discard] 删除本写端创建的文件（即便已写入部分内容）
/// 条件：create_file 建文件并写入 "partial" 后调用 discard
/// 断言：discard 返回 Ok，文件从磁盘移除（回退不限空文件）
#[tokio::test]
async fn created_file_discard_removes_written_file() {
    use tokio::io::AsyncWriteExt;

    let tmp = TempDir::new().unwrap();
    let fs = fs_with_roots(&[tmp.path()]);
    // Go through the `Fs` trait: the inherent `create_file` returns a bare
    // `std::fs::File`, only the trait returns the `FileWriter` with `discard`.
    let fs: &dyn Fs = &fs;
    let path = tmp.path().join("out.bin");

    let (real, mut writer) = fs.create_file(&path).await.unwrap();
    writer.write_all(b"partial").await.unwrap();
    assert!(real.exists());

    writer.discard().await.unwrap();
    assert!(!real.exists(), "discard must remove the created file");
}

/// P1：[FsWrite::discard] 正常 drop（不 discard）保留文件
/// 条件：create_file 建文件后直接 drop 写端
/// 断言：文件仍在磁盘上（只有 discard 才删，drop 只关句柄）
#[tokio::test]
async fn created_file_drop_keeps_file() {
    let tmp = TempDir::new().unwrap();
    let fs = fs_with_roots(&[tmp.path()]);
    let fs: &dyn Fs = &fs;
    let path = tmp.path().join("kept.bin");

    let (real, writer) = fs.create_file(&path).await.unwrap();
    drop(writer);
    assert!(real.exists(), "a normal drop must keep the file");
}
