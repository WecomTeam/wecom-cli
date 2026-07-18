use std::{
    io::Write,
    path::{Path, PathBuf},
};

use crate::{fs_util::sanitize_filename, paths};

/// Maximum dedup index suffix; give up retrying once exceeded.
const MAX_DEDUP_INDEX: u32 = 100;

/// Detect the MIME type by file extension first, then by magic bytes, falling back to `application/octet-stream`.
pub fn detect_mime(file_name: Option<&str>, buffer: &[u8]) -> String {
    // Try to infer from the file extension first
    if let Some(name) = file_name {
        let ext = Path::new(name)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");
        let mime = mime_guess::from_ext(ext).first_or_octet_stream();
        if mime != mime::APPLICATION_OCTET_STREAM {
            return mime.to_string();
        }
    }

    // Then try to infer from magic bytes
    if let Some(kind) = infer::get(buffer) {
        return kind.mime_type().to_string();
    }

    "application/octet-stream".to_string()
}

/// Atomically save media data to the media directory, deduplicating file names when collisions occur.
pub async fn save_media(
    media_name: Option<&str>,
    media_id: Option<&str>,
    content_type: &str,
    data: &[u8],
) -> anyhow::Result<PathBuf> {
    let dir = paths::media_dir();

    tokio::fs::create_dir_all(&dir).await?;

    let (stem, ext) = determine_file_name(media_name, media_id, content_type);

    let mut tmp = tempfile::NamedTempFile::new_in(&dir)?;

    #[cfg(unix)]
    {
        // OHOS MAC policy: chmod succeeds but the actual mode is forced to
        // (owner|0o6)(group|0o6)0, i.e. 0o660 instead of 0o600. This is
        // kernel-enforced and cannot be changed from userspace. In the OHOS
        // app sandbox the group is typically the same app process, so the
        // wider permission is acceptable.
        use std::os::unix::fs::PermissionsExt;
        tmp.as_file()
            .set_permissions(std::fs::Permissions::from_mode(0o600))?;
    }

    tmp.write_all(data)?;
    tmp.as_file().sync_all()?;

    // First try the original file name without an index
    let target = dir.join(make_file_name(&stem, None, &ext));
    tmp = match tmp.persist_noclobber(&target) {
        Ok(_) => return Ok(target),
        Err(e) if e.error.kind() == std::io::ErrorKind::AlreadyExists => e.file,
        Err(e) => return Err(e.error.into()),
    };

    // File already exists; try stem.{index}.ext, index from 0 to MAX_DEDUP_INDEX
    for idx in 0..MAX_DEDUP_INDEX {
        let target = dir.join(make_file_name(&stem, Some(idx), &ext));
        tmp = match tmp.persist_noclobber(&target) {
            Ok(_) => return Ok(target),
            Err(e) if e.error.kind() == std::io::ErrorKind::AlreadyExists => e.file,
            Err(e) => return Err(e.error.into()),
        };
    }

    anyhow::bail!(
        "媒体文件保存失败，目标文件已存在：{}",
        target.to_string_lossy()
    );
}

/// Build a file name in the format `stem[.index].ext`.
fn make_file_name(stem: &str, index: Option<u32>, ext: &str) -> String {
    match (index, ext.is_empty()) {
        (None, true) => stem.to_string(),
        (None, false) => format!("{stem}.{ext}"),
        (Some(i), true) => format!("{stem}.{i}"),
        (Some(i), false) => format!("{stem}.{i}.{ext}"),
    }
}

/// Derive the (stem, extension) pair from media name, media ID, or content type.
pub fn determine_file_name(
    media_name: Option<&str>,
    media_id: Option<&str>,
    content_type: &str,
) -> (String, String) {
    if let Some(name) = media_name.and_then(sanitize_filename) {
        let path = Path::new(&name);
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(&name)
            .to_string();
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_string();
        return (stem, ext);
    }

    let stem = media_id
        .and_then(sanitize_filename)
        .unwrap_or_else(|| "media".to_string());

    let ext = mime_guess::get_mime_extensions_str(content_type)
        .and_then(|exts| exts.first())
        .copied()
        .unwrap_or("bin")
        .to_string();

    (stem, ext)
}
