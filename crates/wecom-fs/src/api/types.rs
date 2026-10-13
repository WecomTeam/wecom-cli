//! Abstract stream contracts and metadata snapshots exchanged through
//! [`crate::Fs`].
//!
//! Business code handles files as [`FileReader`] / [`FileWriter`] trait
//! objects and reads attributes from the [`FileMeta`] snapshot, so it stays
//! independent of any concrete I/O backend.

use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::time::SystemTime;

use tokio::io::{AsyncRead, AsyncWrite};

use crate::api::Result;

/// Boxed future returned by [`crate::Fs`] trait methods.
pub type FsFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T>> + Send + 'a>>;

/// Read-end of a file opened via [`crate::Fs::open_for_read`], with fd-level
/// metadata access.
pub trait FsRead: AsyncRead + Send + Unpin {
    /// Metadata of the opened file (fd-level, TOCTOU-safe).
    fn metadata<'a>(&'a self) -> FsFuture<'a, FileMeta>;
}

/// Boxed read-end returned by [`crate::Fs::open_for_read`].
pub type FileReader = Box<dyn FsRead>;

/// Write-end of a file created via [`crate::Fs::create_file`] /
/// [`crate::Fs::create_file_unique`], with durability support and the ability
/// to discard (roll back) the file it created.
pub trait FsWrite: AsyncWrite + Send + Unpin {
    /// Flush buffers and sync file content and metadata to stable storage.
    fn sync_all<'a>(&'a mut self) -> FsFuture<'a, ()>;

    /// Metadata of the opened file (fd-level).
    fn metadata<'a>(&'a self) -> FsFuture<'a, FileMeta>;

    /// Discard the file this write-end created: close the handle and remove
    /// that file. Used to roll back an output that never became a valid
    /// artifact (a reservation whose first request failed, an interrupted
    /// download).
    ///
    /// This is **not** a user-directed deletion: it undoes *this* handle's own
    /// `create_file`, so it must bypass any user-facing delete policy an
    /// implementor attaches to [`crate::Fs::remove_file`] (approval prompt,
    /// recycle bin, overwrite backup). Because the write-end itself carries the
    /// fact "I created this file", the target is never expressed as a
    /// free-standing path and cannot be redirected at another file. A target
    /// already gone counts as success.
    fn discard(self: Box<Self>) -> FsFuture<'static, ()>;
}

/// Boxed write-end returned by [`crate::Fs::create_file`] /
/// [`crate::Fs::create_file_unique`].
pub type FileWriter = Box<dyn FsWrite>;

/// File metadata snapshot.
#[derive(Debug, Clone)]
pub struct FileMeta {
    /// File size in bytes.
    pub len: u64,
    /// Last modification time, when available.
    pub modified: Option<SystemTime>,
    /// Whether the path is a directory.
    pub is_dir: bool,
    /// Whether the path is a regular file.
    pub is_file: bool,
}

/// A single directory entry returned by [`crate::Fs::list_dir`].
#[derive(Debug, Clone)]
pub struct DirEntry {
    /// Full path of the entry.
    pub path: PathBuf,
    /// Whether the entry is a directory.
    pub is_dir: bool,
    /// Whether the entry is a regular file.
    pub is_file: bool,
}
