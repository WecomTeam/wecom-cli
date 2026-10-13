//! Filesystem capability re-exports and reqwest glue helpers.
//!
//! The [`Fs`] trait and its abstract types live in the `wecom-fs` crate (the
//! `api` module) and are re-exported here, so that business code and the
//! sandbox implementation share one capability definition.
//!
//! Business code depends only on the [`Fs`] trait — never on a concrete
//! implementation.  The implementation is injected by the caller when
//! building a [`crate::Client`] — one configured instance per domain
//! (private state vs. workspace); all path-resolution and access-control
//! semantics belong to the implementor.

mod reqwest_ext;

use std::path::{Path, PathBuf};

pub use wecom_fs::{
    DirEntry, FileMeta, FileReader, FileWriter, Fs, FsAccess, FsFuture, FsRead, FsWrite,
};

use crate::{Error, Result};

/// Absolutize an externally-derived path against `cwd`.
///
/// [`Fs`] implementations require absolute paths; relative input from CLI
/// arguments or model payloads is anchored here, at the construction
/// boundary, before the first [`Fs`] call.  The anchor is the owning
/// client's configured working directory ([`crate::Client::cwd`]), pinned
/// at client construction — never a late `std::env::current_dir()` read,
/// so the base cannot drift mid-run.  Pure path arithmetic — no
/// filesystem access, and no `.` / `..` folding (the implementor's
/// resolve step owns normalization).
pub fn absolutize(cwd: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        return path.to_path_buf();
    }
    cwd.join(path)
}

/// Sanitize a filename for safe use on all platforms.
///
/// Delegates to [`wecom_fs::sanitize_filename`]; retained so existing
/// `fs::sanitize_filename` call sites stay put.
pub fn sanitize_filename(name: &str) -> String {
    wecom_fs::sanitize_filename(name)
}

pub use reqwest_ext::{content_disposition_filename, open_as_multipart_part, stream_to_file};

/// Pre-flight check: verify that the file at `file_path` does not exceed
/// `max_size` bytes, so callers can fail fast on oversized files without
/// wasting network I/O.
pub async fn check_file_size_limit(fs: &dyn Fs, file_path: &Path, max_size: u64) -> Result<()> {
    let file_size = fs.metadata(file_path).await?.len;

    if file_size > max_size {
        tracing::warn!(file_size, limit = max_size, "file exceeds size limit");
        return Err(Error::validation(format!(
            "文件 \"{}\" 大小超过 {:.1} MB 限制",
            file_path.display(),
            max_size as f64 / 1_048_576.0,
        )));
    }

    Ok(())
}

/// Test doubles for this crate's unit tests. Working-filesystem tests use
/// [`wecom_fs::SandboxedFs`] directly; [`ErrFs`] covers rejection propagation
/// and [`FailingWriterFs`] covers write/sync failures with rollback.
#[cfg(test)]
pub(crate) mod testing {
    use std::future::Future;
    use std::path::{Path, PathBuf};
    use std::pin::Pin;
    use std::sync::{Arc, Mutex};

    use wecom_fs::{DirEntry, Error, FileMeta, FileReader, FileWriter, FsAccess, FsWrite, Result};

    use super::Fs;

    /// Filesystem stub whose every operation fails with [`Error::Permission`],
    /// for testing error propagation through consumers.
    #[derive(Debug, Default)]
    pub(crate) struct ErrFs;

    impl ErrFs {
        fn reject<T>(path: &Path) -> Result<T> {
            Err(Error::Permission(format!(
                "目标路径超出可访问范围: {}",
                path.display()
            )))
        }
    }

    impl Fs for ErrFs {
        /// 全部拒绝的测试替身：准入入口同样拒绝。
        fn resolve<'a>(
            &'a self,
            path: &'a Path,
            _access: FsAccess,
        ) -> Pin<Box<dyn Future<Output = Result<PathBuf>> + Send + 'a>> {
            Box::pin(async move { Self::reject(path) })
        }

        fn metadata<'a>(
            &'a self,
            path: &'a Path,
        ) -> Pin<Box<dyn Future<Output = Result<FileMeta>> + Send + 'a>> {
            Box::pin(async move { Self::reject(path) })
        }

        fn open_for_read<'a>(
            &'a self,
            path: &'a Path,
        ) -> Pin<Box<dyn Future<Output = Result<(PathBuf, FileReader)>> + Send + 'a>> {
            Box::pin(async move { Self::reject(path) })
        }

        fn list_dir<'a>(
            &'a self,
            dir: &'a Path,
        ) -> Pin<Box<dyn Future<Output = Result<Vec<DirEntry>>> + Send + 'a>> {
            Box::pin(async move { Self::reject(dir) })
        }

        fn create_file<'a>(
            &'a self,
            path: &'a Path,
        ) -> Pin<Box<dyn Future<Output = Result<(PathBuf, FileWriter)>> + Send + 'a>> {
            Box::pin(async move { Self::reject(path) })
        }

        fn atomic_write<'a>(
            &'a self,
            path: &'a Path,
            _data: &'a [u8],
            _mode: Option<u32>,
        ) -> Pin<Box<dyn Future<Output = Result<PathBuf>> + Send + 'a>> {
            Box::pin(async move { Self::reject(path) })
        }

        fn remove_file<'a>(
            &'a self,
            path: &'a Path,
        ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>> {
            Box::pin(async move { Self::reject(path) })
        }
    }

    /// Write-end stub whose `write_all` (`fail_on_write = true`) or
    /// `sync_all` (`fail_on_write = false`) is injected to fail. `discard`
    /// really unlinks the recorded path, so rollback assertions can observe it.
    #[derive(Debug)]
    struct FailingWriter {
        fail_on_write: bool,
        path: PathBuf,
    }

    impl tokio::io::AsyncWrite for FailingWriter {
        fn poll_write(
            self: Pin<&mut Self>,
            _cx: &mut std::task::Context<'_>,
            buf: &[u8],
        ) -> std::task::Poll<std::io::Result<usize>> {
            if self.fail_on_write {
                std::task::Poll::Ready(Err(std::io::Error::other("injected write failure")))
            } else {
                std::task::Poll::Ready(Ok(buf.len()))
            }
        }

        fn poll_flush(
            self: Pin<&mut Self>,
            _cx: &mut std::task::Context<'_>,
        ) -> std::task::Poll<std::io::Result<()>> {
            std::task::Poll::Ready(Ok(()))
        }

        fn poll_shutdown(
            self: Pin<&mut Self>,
            _cx: &mut std::task::Context<'_>,
        ) -> std::task::Poll<std::io::Result<()>> {
            std::task::Poll::Ready(Ok(()))
        }
    }

    impl FsWrite for FailingWriter {
        fn sync_all<'a>(&'a mut self) -> wecom_fs::FsFuture<'a, ()> {
            Box::pin(async move {
                if self.fail_on_write {
                    Ok(())
                } else {
                    Err(Error::other("injected sync failure".into()))
                }
            })
        }

        fn metadata<'a>(&'a self) -> wecom_fs::FsFuture<'a, FileMeta> {
            Box::pin(async {
                Ok(FileMeta {
                    len: 0,
                    modified: None,
                    is_dir: false,
                    is_file: true,
                })
            })
        }

        #[allow(clippy::disallowed_methods)] // 测试替身：真实删除本写端创建的文件
        fn discard(self: Box<Self>) -> wecom_fs::FsFuture<'static, ()> {
            Box::pin(async move {
                match std::fs::remove_file(&self.path) {
                    Ok(()) => Ok(()),
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
                    Err(e) => Err(Error::io("discard stub file", e)),
                }
            })
        }
    }

    /// File-backed fs stub for rollback assertions: `create_file` creates a
    /// real empty file at the requested path and records it; the returned
    /// writer fails on write or on sync per configuration and really unlinks
    /// the file on `discard` (the rollback path). Every other operation rejects
    /// with [`Error::Permission`].
    #[derive(Debug, Clone, Default)]
    pub(crate) struct FailingWriterFs {
        fail_on_write: bool,
        created: Arc<Mutex<Vec<PathBuf>>>,
    }

    impl FailingWriterFs {
        /// Stub whose writer fails in `write_all` (`fail_on_write = true`) or
        /// in `sync_all` (`fail_on_write = false`).
        pub(crate) fn new(fail_on_write: bool) -> Self {
            Self {
                fail_on_write,
                created: Arc::new(Mutex::new(Vec::new())),
            }
        }

        /// Paths handed out by `create_file`, in creation order.
        pub(crate) fn created_paths(&self) -> Vec<PathBuf> {
            self.created.lock().unwrap().clone()
        }

        /// Build a client whose workspace fs is this stub.
        pub(crate) fn client(self, config_dir: &Path) -> crate::Client {
            crate::Client::builder()
                .workspace_fs(Arc::new(self))
                .config_dir(config_dir)
                .build()
                .expect("build failing-writer client")
        }

        fn reject<T>(path: &Path) -> Result<T> {
            Err(Error::Permission(format!(
                "目标路径超出可访问范围: {}",
                path.display()
            )))
        }
    }

    impl Fs for FailingWriterFs {
        fn metadata<'a>(
            &'a self,
            path: &'a Path,
        ) -> Pin<Box<dyn Future<Output = Result<FileMeta>> + Send + 'a>> {
            Box::pin(async move { Self::reject(path) })
        }

        fn open_for_read<'a>(
            &'a self,
            path: &'a Path,
        ) -> Pin<Box<dyn Future<Output = Result<(PathBuf, FileReader)>> + Send + 'a>> {
            Box::pin(async move { Self::reject(path) })
        }

        fn list_dir<'a>(
            &'a self,
            dir: &'a Path,
        ) -> Pin<Box<dyn Future<Output = Result<Vec<DirEntry>>> + Send + 'a>> {
            Box::pin(async move { Self::reject(dir) })
        }

        #[allow(clippy::disallowed_methods)] // 测试替身：真实落盘空文件供回退断言
        fn create_file<'a>(
            &'a self,
            path: &'a Path,
        ) -> Pin<Box<dyn Future<Output = Result<(PathBuf, FileWriter)>> + Send + 'a>> {
            let fail_on_write = self.fail_on_write;
            let created = Arc::clone(&self.created);
            let path = path.to_path_buf();
            Box::pin(async move {
                std::fs::File::create(&path).map_err(|e| Error::io("create stub file", e))?;
                created.lock().unwrap().push(path.clone());
                Ok((
                    path.clone(),
                    Box::new(FailingWriter {
                        fail_on_write,
                        path,
                    }) as FileWriter,
                ))
            })
        }

        fn atomic_write<'a>(
            &'a self,
            path: &'a Path,
            _data: &'a [u8],
            _mode: Option<u32>,
        ) -> Pin<Box<dyn Future<Output = Result<PathBuf>> + Send + 'a>> {
            Box::pin(async move { Self::reject(path) })
        }

        #[allow(clippy::disallowed_methods)] // 测试替身：真实删除 create_file 落盘的文件
        fn remove_file<'a>(
            &'a self,
            path: &'a Path,
        ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>> {
            Box::pin(async move {
                std::fs::remove_file(path).map_err(|e| Error::io("remove stub file", e))
            })
        }

        fn resolve<'a>(
            &'a self,
            path: &'a Path,
            _access: FsAccess,
        ) -> Pin<Box<dyn Future<Output = Result<PathBuf>> + Send + 'a>> {
            Box::pin(async move { Self::reject(path) })
        }
    }
}

#[cfg(test)]
mod tests {
    //! ## 模块摘要：fs（消费侧测试替身 ErrFs、文件大小预检与路径锚定）
    //!
    //! ### 关键接口
    //! - [ErrFs] — 全部操作一律拒绝（Permission）的测试替身
    //! - [FailingWriterFs] — 真实落盘 + 可注入 write/sync 失败、discard 真实删除的测试替身
    //! - [check_file_size_limit] — 文件大小预检
    //! - [absolutize] — 相对路径锚定到 client cwd（构造点调用）
    //!
    //! ### 关键分支与异常路径
    //! - ErrFs 任一方法 → Err(Permission)，消息含 "目标路径超出可访问范围"
    //! - FailingWriterFs：create 真实落盘；writer 在 write 或 sync 侧失败；discard 真实删除
    //! - FailingWriter::discard：目标已消失 → Ok（幂等）；目标不可删除 → Err(Io)
    //! - FailingWriterFs 底层 I/O 失败（目标目录 / 文件不存在）→ Err(Io)，不记录新路径
    //! - 文件小于 / 等于 / 超过 max_size → Ok / Ok / Err(validation)
    //! - absolutize：相对路径 → 拼 cwd；绝对路径 → 原样返回
    //!
    //! ### 上下游交互
    //! - 上游：本 crate 各消费侧测试注入该替身（如 directive / service / builtins / reqwest_ext）

    use wecom_fs::Error as FsError;

    use super::testing::{ErrFs, FailingWriterFs};
    use super::{Fs, FsAccess, absolutize, check_file_size_limit};

    /// P0：[absolutize] 相对路径拼接到给定 cwd
    /// 条件：cwd = "/work"，输入 "sub/file.txt"
    /// 断言：返回 "/work/sub/file.txt"
    #[test]
    fn absolutize_joins_relative_path_onto_cwd() {
        assert_eq!(
            absolutize(
                std::path::Path::new("/work"),
                std::path::Path::new("sub/file.txt")
            ),
            std::path::PathBuf::from("/work/sub/file.txt")
        );
    }

    /// P0：[absolutize] 绝对路径原样返回（不拼 cwd）
    /// 条件：输入 "/etc/hosts"
    /// 断言：返回 "/etc/hosts"
    #[test]
    fn absolutize_keeps_absolute_path() {
        assert_eq!(
            absolutize(
                std::path::Path::new("/work"),
                std::path::Path::new("/etc/hosts")
            ),
            std::path::PathBuf::from("/etc/hosts")
        );
    }

    /// P0：[ErrFs] 读侧方法一律返回 Permission 错误
    /// 条件：对 ErrFs 调用 resolve / metadata / open_for_read / list_dir
    /// 断言：均返回 Err(Permission)，消息含 "目标路径超出可访问范围"
    #[tokio::test]
    async fn err_fs_rejects_read_operations() {
        let fs = ErrFs;
        let path = std::path::Path::new("/tmp/anywhere.txt");

        let err = fs.resolve(path, FsAccess::Read).await.err();
        assert!(
            matches!(err, Some(FsError::Permission(_))),
            "resolve should reject with Permission"
        );
        let err = fs.metadata(path).await.err();
        assert!(
            matches!(err, Some(FsError::Permission(_))),
            "metadata should reject with Permission"
        );
        let err = fs.open_for_read(path).await.err();
        assert!(
            matches!(err, Some(FsError::Permission(_))),
            "open_for_read should reject with Permission"
        );
        let err = fs.list_dir(path).await.err();
        assert!(
            matches!(err, Some(FsError::Permission(_))),
            "list_dir should reject with Permission"
        );
    }

    /// P0：[ErrFs] 写侧方法一律返回 Permission 错误
    /// 条件：对 ErrFs 调用 create_file / atomic_write / remove_file
    /// 断言：均返回 Err(Permission)
    #[tokio::test]
    async fn err_fs_rejects_write_operations() {
        let fs = ErrFs;
        let path = std::path::Path::new("/tmp/anywhere.txt");

        let err = fs.create_file(path).await.err();
        assert!(
            matches!(err, Some(FsError::Permission(_))),
            "create_file should reject with Permission"
        );
        let err = fs.atomic_write(path, b"data", None).await.err();
        assert!(
            matches!(err, Some(FsError::Permission(_))),
            "atomic_write should reject with Permission"
        );
        let err = fs.remove_file(path).await.err();
        assert!(
            matches!(err, Some(FsError::Permission(_))),
            "remove_file should reject with Permission"
        );
    }

    // ── FailingWriterFs ──

    /// P2：[FailingWriterFs] 夹具自检：非落盘方法一律拒绝，create 真实创建并记录，
    ///       writer 按配置在 write / sync 侧失败，discard 真实删除（回退路径）
    /// 条件：对桩调用全部 Fs 方法与 writer 的 flush/shutdown/metadata/sync_all/discard
    /// 断言：Fs 非 create/remove 方法均 Err(Permission)；fail_on_write=true 时 write 失败且
    ///       sync_all 通过；false 时 write 成功而 sync_all 失败；discard 后文件消失
    #[tokio::test]
    #[allow(clippy::disallowed_methods)] // 测试夹具直接观察真实落盘文件
    async fn failing_writer_fs_fixture_self_check() {
        use tokio::io::AsyncWriteExt;

        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("stub.txt");

        let fs = FailingWriterFs::new(true);
        assert!(matches!(
            fs.metadata(&path).await,
            Err(FsError::Permission(_))
        ));
        assert!(fs.open_for_read(&path).await.is_err());
        assert!(fs.list_dir(tmp.path()).await.is_err());
        assert!(fs.atomic_write(&path, b"x", None).await.is_err());
        assert!(fs.resolve(&path, FsAccess::Read).await.is_err());

        let (created, mut writer) = fs.create_file(&path).await.unwrap();
        assert_eq!(created, path);
        assert_eq!(fs.created_paths(), vec![path.clone()]);
        assert!(path.exists(), "create_file must land a real file");
        writer.flush().await.unwrap();
        writer.shutdown().await.unwrap();
        assert!(writer.metadata().await.unwrap().is_file);
        // fail_on_write = true：失败注入在 write 侧，sync_all 走 Ok 侧
        assert!(writer.write_all(b"x").await.is_err());
        writer.sync_all().await.unwrap();
        // discard 真实删除本写端创建的文件（回退路径）。
        writer.discard().await.unwrap();
        assert!(!path.exists(), "discard must really unlink the file");

        // 另一侧：write 成功、sync_all 失败。
        let fs2 = FailingWriterFs::new(false);
        let (_, mut writer2) = fs2.create_file(&path).await.unwrap();
        writer2.write_all(b"x").await.unwrap();
        assert!(writer2.sync_all().await.is_err());
        assert_eq!(fs2.created_paths(), vec![path.clone()]);
    }

    /// P2：[FailingWriterFs] create_file / remove_file 的底层 I/O 失败透传为 Err(Io)
    /// 条件：create_file 目标位于不存在的目录；remove_file 目标文件不存在
    /// 断言：均返回 Err(Io)，消息含 "stub file"，且未记录新路径
    #[tokio::test]
    async fn failing_writer_fs_io_failures_propagate() {
        let fs = FailingWriterFs::new(true);
        let tmp = tempfile::tempdir().unwrap();
        let in_missing_dir = tmp.path().join("no-such-dir").join("f.txt");

        let err = fs.create_file(&in_missing_dir).await.err();
        assert!(
            matches!(err, Some(FsError::Io { .. })),
            "create_file in missing dir should fail with Io"
        );
        assert!(fs.created_paths().is_empty());

        let missing_file = tmp.path().join("never-created.bin");
        let err = fs.remove_file(&missing_file).await.err();
        assert!(
            matches!(err, Some(FsError::Io { .. })),
            "remove_file of missing file should fail with Io"
        );
    }

    /// P2：[FailingWriter::discard] 目标已消失视为成功、目标不可删除报 Err(Io)
    /// 条件：create_file 后手工删除文件再 discard（NotFound）；把占位替换为同名目录再 discard（非 NotFound 删除失败）
    /// 断言：前者 Ok；后者 Err(Io) 且消息含 "discard stub file"，目录保持存在
    #[tokio::test]
    #[allow(clippy::disallowed_methods)] // 测试夹具直接操作文件系统，构造 discard 的两条边界分支
    async fn failing_writer_discard_edge_branches() {
        let tmp = tempfile::tempdir().unwrap();
        let fs = FailingWriterFs::new(false);

        // 1. 目标已被移除：撤回幂等，视为成功
        let gone = tmp.path().join("gone.bin");
        let (_, writer) = fs.create_file(&gone).await.unwrap();
        std::fs::remove_file(&gone).unwrap();
        writer
            .discard()
            .await
            .expect("a vanished target counts as discarded");

        // 2. 目标被替换为同名目录：unlink 报非 NotFound 错误，透传为 Err(Io)
        let replaced = tmp.path().join("replaced.bin");
        let (_, writer) = fs.create_file(&replaced).await.unwrap();
        std::fs::remove_file(&replaced).unwrap();
        std::fs::create_dir(&replaced).unwrap();
        let err = writer
            .discard()
            .await
            .expect_err("a directory must not be discardable");
        assert!(
            matches!(err, FsError::Io { .. }),
            "discard failure should surface as Io: {err:?}"
        );
        assert!(replaced.is_dir(), "the directory must stay intact");
    }

    // ── check_file_size_limit ──

    /// P0：[check_file_size_limit] 文件小于限制时返回 Ok
    /// 条件：创建 10 字节文件，max_size = 1024
    /// 断言：返回 Ok(())
    #[tokio::test]
    #[allow(clippy::disallowed_methods)] // 测试夹具直接落盘构造前置文件
    async fn check_file_size_limit_small_file_passes() {
        let tmp = tempfile::tempdir().unwrap();
        let file_path = tmp.path().join("small.txt");
        std::fs::write(&file_path, b"1234567890").unwrap();

        let fs = wecom_fs::SandboxedFs::new();
        let result = check_file_size_limit(&fs, &file_path, 1024).await;
        assert!(result.is_ok());
    }

    /// P0：[check_file_size_limit] 文件超过限制时返回 Err
    /// 条件：创建 10 字节文件，max_size = 4
    /// 断言：返回 Err，错误信息包含文件路径
    #[tokio::test]
    #[allow(clippy::disallowed_methods)] // 测试夹具直接落盘构造前置文件
    async fn check_file_size_limit_oversized_file_fails() {
        let tmp = tempfile::tempdir().unwrap();
        let file_path = tmp.path().join("big.txt");
        std::fs::write(&file_path, b"1234567890").unwrap();
        let file_path_str = file_path.to_string_lossy().to_string();

        let fs = wecom_fs::SandboxedFs::new();
        let result = check_file_size_limit(&fs, &file_path, 4).await;
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains(&file_path_str),
            "error should mention file path: {msg}"
        );
    }

    /// P1：[check_file_size_limit] 文件大小等于限制时返回 Ok
    /// 条件：创建 10 字节文件，max_size = 10
    /// 断言：返回 Ok(())（等于限制不视为超限）
    #[tokio::test]
    #[allow(clippy::disallowed_methods)] // 测试夹具直接落盘构造前置文件
    async fn check_file_size_limit_equal_limit_passes() {
        let tmp = tempfile::tempdir().unwrap();
        let file_path = tmp.path().join("exact.txt");
        std::fs::write(&file_path, b"1234567890").unwrap();

        let fs = wecom_fs::SandboxedFs::new();
        let result = check_file_size_limit(&fs, &file_path, 10).await;
        assert!(result.is_ok());
    }
}
