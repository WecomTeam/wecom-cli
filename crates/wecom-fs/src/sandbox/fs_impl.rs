//! Async execution facade: the per-operation methods and the [`Fs`]
//! implementation for [`SandboxedFs`].
//!
//! Every operation is the same hop — [`SandboxedFs::resolve`] on the async
//! thread (cheap, no I/O), then the synchronous primitive from [`super::io`]
//! on the blocking pool with the direction's policy moved in.  The
//! security-relevant choice therefore reduces to "which policy does this
//! operation use": `self.read` for read-only operations, `self.write` for
//! anything that creates, modifies or deletes.
//!
//! The inherent methods exist because the trait returns capability types
//! (`FileReader` / `FileWriter` / [`FileMeta`]) while the primitives return
//! concrete `std` / `tokio` handles; [`TokioFile`] bridges the two.
//! `Fs::read_to_string` and `Fs::create_file_unique` are *not* overridden —
//! the trait defaults build on the methods below and need no sandbox-specific
//! variant.

use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use super::{Policy, SandboxedFs, blocking, io};
use crate::api::{
    DirEntry, Error, FileMeta, FileReader, FileWriter, Fs, FsAccess, FsFuture, FsRead, FsWrite,
    Result,
};

// ── Inherent async operations ───────────────────────────────

impl SandboxedFs {
    /// Resolve + TOCTOU-safe **create** of a new file with `0o600`
    /// permissions (`create_new` semantics), returning the resolved path and
    /// the opened file.
    pub(super) async fn create_file(&self, path: &Path) -> Result<(PathBuf, std::fs::File)> {
        let abs = self.resolve(path)?;
        let policy = self.write.clone();
        blocking("create_file", move || io::create_file(&abs, &policy)).await
    }

    /// Open `path` for reading, returning the resolved path and a
    /// [`tokio::fs::File`] wrapping the sandbox-validated fd.
    pub(super) async fn open_for_read(&self, path: &Path) -> Result<(PathBuf, tokio::fs::File)> {
        let abs = self.resolve(path)?;
        let policy = self.read.clone();
        let (real, file) = blocking("open_for_read", move || io::open_file(&abs, &policy)).await?;
        Ok((real, tokio::fs::File::from_std(file)))
    }

    /// File metadata, taken from the opened fd rather than by re-statting the
    /// path (so the check and the use share one object).
    pub(super) async fn metadata(&self, path: &Path) -> Result<std::fs::Metadata> {
        let abs = self.resolve(path)?;
        let policy = self.read.clone();
        blocking("metadata", move || {
            let (real, file) = io::open_file(&abs, &policy)?;
            file.metadata()
                .map_err(|e| Error::io(format!("Failed to stat {}", real.display()), e))
        })
        .await
    }

    /// List all entries (files and subdirectories) of a directory; callers
    /// filter by type themselves.
    pub(super) async fn list_dir(&self, dir: &Path) -> Result<Vec<DirEntry>> {
        let abs = self.resolve(dir)?;
        let policy = self.read.clone();
        blocking("list_dir", move || io::list_dir(&abs, &policy)).await
    }

    /// Atomically write `data` to `path` (temp file → fsync → rename),
    /// replacing any existing file.
    pub(super) async fn atomic_write(
        &self,
        path: &Path,
        data: &[u8],
        mode: u32,
    ) -> Result<PathBuf> {
        let abs = self.resolve(path)?;
        let data = data.to_vec();
        let policy = self.write.clone();
        blocking("atomic_write", move || {
            io::atomic_write(&abs, &data, mode, &policy)
        })
        .await
    }

    /// Remove a single file.
    pub(super) async fn remove_file(&self, path: &Path) -> Result<()> {
        let abs = self.resolve(path)?;
        let policy = self.write.clone();
        blocking("remove_file", move || io::remove_file(&abs, &policy)).await
    }
}

// ── Stream adapter ──────────────────────────────────────────

/// Convert std metadata to the crate-agnostic [`FileMeta`] snapshot.
fn file_meta_from_std(m: &std::fs::Metadata) -> FileMeta {
    FileMeta {
        len: m.len(),
        modified: m.modified().ok(),
        is_dir: m.is_dir(),
        is_file: m.is_file(),
    }
}

/// Read-end: a tokio file handle satisfying the [`FsRead`] contract.
struct TokioFile(tokio::fs::File);

impl tokio::io::AsyncRead for TokioFile {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.0).poll_read(cx, buf)
    }
}

impl FsRead for TokioFile {
    fn metadata<'a>(&'a self) -> FsFuture<'a, FileMeta> {
        Box::pin(async move {
            let m = tokio::fs::File::metadata(&self.0)
                .await
                .map_err(|e| Error::io("Failed to read file metadata", e))?;
            Ok(file_meta_from_std(&m))
        })
    }
}

/// Write-end of a file created through [`SandboxedFs::create_file`]: the tokio
/// handle plus the resolved path and write policy needed to discard it.
///
/// `discard` removes the created file through the same write policy — a plain
/// policy-checked unlink, identical to `remove_file` at this layer (the two
/// only diverge for implementors that attach user-facing semantics to
/// `remove_file`). A normal drop just closes the handle and keeps the file.
struct CreatedFile {
    file: tokio::fs::File,
    path: PathBuf,
    policy: Arc<Policy>,
}

impl tokio::io::AsyncWrite for CreatedFile {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        Pin::new(&mut self.file).poll_write(cx, buf)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.file).poll_flush(cx)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.file).poll_shutdown(cx)
    }

    fn poll_write_vectored(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bufs: &[std::io::IoSlice<'_>],
    ) -> Poll<std::io::Result<usize>> {
        Pin::new(&mut self.file).poll_write_vectored(cx, bufs)
    }

    fn is_write_vectored(&self) -> bool {
        self.file.is_write_vectored()
    }
}

impl FsWrite for CreatedFile {
    fn sync_all<'a>(&'a mut self) -> FsFuture<'a, ()> {
        Box::pin(async move {
            tokio::fs::File::sync_all(&self.file)
                .await
                .map_err(|e| Error::io("Failed to sync file", e))
        })
    }

    fn metadata<'a>(&'a self) -> FsFuture<'a, FileMeta> {
        Box::pin(async move {
            let m = tokio::fs::File::metadata(&self.file)
                .await
                .map_err(|e| Error::io("Failed to read file metadata", e))?;
            Ok(file_meta_from_std(&m))
        })
    }

    fn discard(self: Box<Self>) -> FsFuture<'static, ()> {
        let CreatedFile { file, path, policy } = *self;
        Box::pin(async move {
            // Release this handle before unlinking (Windows refuses to remove a
            // file with an open handle). Remove through the write policy — the
            // same unlink `remove_file` performs at this layer.
            drop(file);
            blocking("discard", move || io::remove_file(&path, &policy)).await
        })
    }
}

// ── `Fs` bridge ─────────────────────────────────────────────

impl Fs for SandboxedFs {
    fn metadata<'a>(&'a self, path: &'a Path) -> FsFuture<'a, FileMeta> {
        Box::pin(async move {
            let m = SandboxedFs::metadata(self, path).await?;
            Ok(file_meta_from_std(&m))
        })
    }

    fn open_for_read<'a>(&'a self, path: &'a Path) -> FsFuture<'a, (PathBuf, FileReader)> {
        Box::pin(async move {
            let (real, file) = SandboxedFs::open_for_read(self, path).await?;
            Ok((real, Box::new(TokioFile(file)) as FileReader))
        })
    }

    fn list_dir<'a>(&'a self, dir: &'a Path) -> FsFuture<'a, Vec<DirEntry>> {
        Box::pin(async move { SandboxedFs::list_dir(self, dir).await })
    }

    fn create_file<'a>(&'a self, path: &'a Path) -> FsFuture<'a, (PathBuf, FileWriter)> {
        Box::pin(async move {
            let (real, file) = SandboxedFs::create_file(self, path).await?;
            let writer = CreatedFile {
                file: tokio::fs::File::from_std(file),
                path: real.clone(),
                policy: Arc::clone(&self.write),
            };
            Ok((real, Box::new(writer) as FileWriter))
        })
    }

    fn atomic_write<'a>(
        &'a self,
        path: &'a Path,
        data: &'a [u8],
        mode: Option<u32>,
    ) -> FsFuture<'a, PathBuf> {
        Box::pin(
            async move { SandboxedFs::atomic_write(self, path, data, mode.unwrap_or(0o600)).await },
        )
    }

    fn remove_file<'a>(&'a self, path: &'a Path) -> FsFuture<'a, ()> {
        Box::pin(async move { SandboxedFs::remove_file(self, path).await })
    }

    fn resolve<'a>(&'a self, path: &'a Path, access: FsAccess) -> FsFuture<'a, PathBuf> {
        Box::pin(async move {
            // All access modes require UTF-8 paths: error messages quote the
            // raw text form, and non-UTF-8 bytes would defeat the
            // dangerous-character screening.  Reject rather than quote a
            // lossy rendering.  `SandboxedFs::resolve` applies the same
            // rejection to every operation entry point.
            let raw = path.to_str().ok_or_else(|| {
                Error::validation(format!("路径包含非 UTF-8 字符: {}", path.display()))
            })?;
            match access {
                FsAccess::Read => SandboxedFs::resolve_readable(self, raw).await,
                FsAccess::Write => SandboxedFs::check_writable(self, path).await,
                FsAccess::WriteDir => SandboxedFs::resolve_writable_dir(self, path).await,
            }
        })
    }
}
