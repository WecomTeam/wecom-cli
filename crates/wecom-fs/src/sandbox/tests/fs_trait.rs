//! ## 模块摘要：Fs trait 统一入口（意图分派与默认实现）
//!
//! ### 关键接口
//! - [Fs] trait 的各方法在 [SandboxedFs] 上的实现
//! - trait 默认实现（唯一后缀重试等）经统一入口生效
//!
//! ### 关键分支与异常路径
//! - 不同 [FsAccess] 意图 → 分派到对应方向的 roots 校验
//! - 非 UTF-8 路径 → 在 Fs 边界被拒绝
//!
//! ### 上下游交互
//! - 上游：业务代码全程通过 `Arc<dyn Fs>` 调用
//! - 下游：sandbox::io 原语、mod.rs 的 resolve_* / check_* 入口

use std::fs as stdfs;
use std::path::{Path, PathBuf};

use tempfile::TempDir;

use super::fs_with_roots;
use crate::api::*;

// ── Fs trait 统一入口：意图分派与默认实现 ─────────────────────────────

/// P0：[Fs::resolve] WriteDir 意图分派到目录逻辑而非文件逻辑。
/// 条件：roots 内分别预置同名文件、同名目录、以及不存在的路径；经 trait
/// 统一入口（`&dyn Fs`）以 FsAccess::WriteDir 调用
/// 断言：文件 → Err 且文案含「无效目录路径」；目录与不存在路径 → Ok
#[tokio::test]
async fn resolve_write_dir_dispatches_to_dir_logic() {
    let tmp = TempDir::new().unwrap();
    let fs = fs_with_roots(&[tmp.path()]);
    let fs: &dyn Fs = &fs;

    // 已存在同名文件 → 拒绝（分派错到 Write 逻辑时此断言会漏）。
    let occupied = tmp.path().join("occupied");
    stdfs::write(&occupied, "x").unwrap();
    let err = fs
        .resolve(&occupied, FsAccess::WriteDir)
        .await
        .expect_err("file target must be rejected for WriteDir");
    assert!(err.to_string().contains("无效目录路径"), "err = {err}");

    // 已存在目录 → 通过。
    let dir_target = tmp.path().join("subdir");
    stdfs::create_dir_all(&dir_target).unwrap();
    fs.resolve(&dir_target, FsAccess::WriteDir)
        .await
        .expect("existing dir target passes");

    // 不存在 → 通过（由调用方后续创建）。
    fs.resolve(&tmp.path().join("not-yet"), FsAccess::WriteDir)
        .await
        .expect("missing dir target passes");
}

/// P1：[Fs trait 默认实现] 只实现 8 个必需方法的最小 Fs（不 override
/// read_to_string / create_file_unique）：默认 read_to_string
/// 经 open_for_read 流式读回；默认 create_file_unique 在目标已占用时自动
/// 追加随机后缀重试且原文件不动。
/// 条件：真实 TempDir 上的直通实现（无准入）；预置 a.txt
/// 断言：默认实现行为符合 trait 契约
#[tokio::test]
async fn trait_default_read_to_string_and_create_file_unique() {
    use std::pin::Pin;
    use std::task::{Context, Poll};

    /// 最小实现：直通真实文件系统，不 override 任何默认方法。
    #[derive(Debug)]
    struct MinimalFs;

    struct FileEnd(tokio::fs::File);

    impl tokio::io::AsyncRead for FileEnd {
        fn poll_read(
            mut self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            buf: &mut tokio::io::ReadBuf<'_>,
        ) -> Poll<std::io::Result<()>> {
            Pin::new(&mut self.0).poll_read(cx, buf)
        }
    }

    impl tokio::io::AsyncWrite for FileEnd {
        fn poll_write(
            mut self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            buf: &[u8],
        ) -> Poll<std::io::Result<usize>> {
            Pin::new(&mut self.0).poll_write(cx, buf)
        }
        fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
            Pin::new(&mut self.0).poll_flush(cx)
        }
        fn poll_shutdown(
            mut self: Pin<&mut Self>,
            cx: &mut Context<'_>,
        ) -> Poll<std::io::Result<()>> {
            Pin::new(&mut self.0).poll_shutdown(cx)
        }
    }

    fn meta_from(m: &std::fs::Metadata) -> FileMeta {
        FileMeta {
            len: m.len(),
            modified: m.modified().ok(),
            is_dir: m.is_dir(),
            is_file: m.is_file(),
        }
    }

    impl FsRead for FileEnd {
        fn metadata<'a>(&'a self) -> FsFuture<'a, FileMeta> {
            Box::pin(async move {
                let m = self
                    .0
                    .metadata()
                    .await
                    .map_err(|e| Error::io("stat read-end", e))?;
                Ok(meta_from(&m))
            })
        }
    }

    impl FsWrite for FileEnd {
        fn sync_all<'a>(&'a mut self) -> FsFuture<'a, ()> {
            Box::pin(async move {
                self.0
                    .sync_all()
                    .await
                    .map_err(|e| Error::io("sync write-end", e))
            })
        }
        fn metadata<'a>(&'a self) -> FsFuture<'a, FileMeta> {
            Box::pin(async move {
                let m = self
                    .0
                    .metadata()
                    .await
                    .map_err(|e| Error::io("stat write-end", e))?;
                Ok(meta_from(&m))
            })
        }
        /// This trait-default test double carries no path; the default-method
        /// tests never exercise discard, so it is a no-op.
        fn discard(self: Box<Self>) -> FsFuture<'static, ()> {
            Box::pin(async { Ok(()) })
        }
    }

    impl Fs for MinimalFs {
        fn metadata<'a>(&'a self, path: &'a Path) -> FsFuture<'a, FileMeta> {
            Box::pin(async move {
                let m = tokio::fs::metadata(path)
                    .await
                    .map_err(|e| Error::io("stat", e))?;
                Ok(meta_from(&m))
            })
        }

        fn open_for_read<'a>(&'a self, path: &'a Path) -> FsFuture<'a, (PathBuf, FileReader)> {
            Box::pin(async move {
                let f = tokio::fs::File::open(path)
                    .await
                    .map_err(|e| Error::io("open", e))?;
                Ok((path.to_path_buf(), Box::new(FileEnd(f)) as FileReader))
            })
        }

        fn list_dir<'a>(&'a self, dir: &'a Path) -> FsFuture<'a, Vec<DirEntry>> {
            Box::pin(async move {
                let mut entries = tokio::fs::read_dir(dir)
                    .await
                    .map_err(|e| Error::io("read_dir", e))?;
                let mut out = Vec::new();
                while let Some(entry) = entries
                    .next_entry()
                    .await
                    .map_err(|e| Error::io("next entry", e))?
                {
                    let ft = entry
                        .file_type()
                        .await
                        .map_err(|e| Error::io("file type", e))?;
                    out.push(DirEntry {
                        path: entry.path(),
                        is_dir: ft.is_dir(),
                        is_file: ft.is_file(),
                    });
                }
                Ok(out)
            })
        }

        fn create_file<'a>(&'a self, path: &'a Path) -> FsFuture<'a, (PathBuf, FileWriter)> {
            Box::pin(async move {
                let f = tokio::fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(path)
                    .await
                    .map_err(|e| Error::io("create", e))?;
                Ok((path.to_path_buf(), Box::new(FileEnd(f)) as FileWriter))
            })
        }

        fn atomic_write<'a>(
            &'a self,
            path: &'a Path,
            data: &'a [u8],
            _mode: Option<u32>,
        ) -> FsFuture<'a, PathBuf> {
            Box::pin(async move {
                tokio::fs::write(path, data)
                    .await
                    .map_err(|e| Error::io("write", e))?;
                Ok(path.to_path_buf())
            })
        }

        fn remove_file<'a>(&'a self, path: &'a Path) -> FsFuture<'a, ()> {
            Box::pin(async move {
                tokio::fs::remove_file(path)
                    .await
                    .map_err(|e| Error::io("remove file", e))
            })
        }

        fn resolve<'a>(&'a self, path: &'a Path, _access: FsAccess) -> FsFuture<'a, PathBuf> {
            Box::pin(async move { Ok(path.to_path_buf()) })
        }
    }

    let tmp = TempDir::new().unwrap();
    let fs: &dyn Fs = &MinimalFs;
    let target = tmp.path().join("a.txt");
    stdfs::write(&target, "old").unwrap();

    // 默认 read_to_string：经 open_for_read 流式读回。
    let content = fs
        .read_to_string(&target)
        .await
        .expect("default read_to_string");
    assert_eq!(content, "old");

    // 默认 create_file_unique：目标已占用 → 追加随机后缀重试，原文件不动。
    let (unique, _writer) = fs
        .create_file_unique(&target)
        .await
        .expect("default create_file_unique");
    assert_ne!(unique, target, "unique path must differ");
    let name = unique
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default()
        .to_string();
    assert!(
        name.starts_with("a.") && name.ends_with(".txt"),
        "unique name should be stem.<suffix>.ext, got {name}"
    );
    assert_eq!(stdfs::read_to_string(&target).unwrap(), "old");
}
