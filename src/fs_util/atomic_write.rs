use std::fs;
use std::io::Write;
use std::path::Path;

use anyhow::Result;

/// Atomically write `data` (bytes) to `path`, overwriting if the file already exists.
///
/// Uses `tempfile::NamedTempFile` to create a temporary file in the same directory,
/// then atomically renames it to the target path via `persist`.
pub fn atomic_write(path: &Path, data: &[u8], mode: Option<u32>) -> Result<()> {
    let Some(parent) = path.parent() else {
        anyhow::bail!("无效文件路径：{}", path.display());
    };

    fs::create_dir_all(parent)?;

    // Create a temp file in the same directory (same filesystem → atomic rename).
    let mut tmp = tempfile::NamedTempFile::new_in(parent)?;

    // Set permissions before writing content (avoid permission window).
    // OHOS MAC policy: chmod succeeds but the actual mode is forced to
    // (owner|0o6)(group|0o6)0 — see the test mod comment below for details.
    // The call still succeeds on OHOS, just with a wider mode than requested.
    #[cfg(unix)]
    if let Some(m) = mode {
        use std::os::unix::fs::PermissionsExt;
        tmp.as_file()
            .set_permissions(fs::Permissions::from_mode(m))?;
    }

    tmp.write_all(data)?;
    tmp.as_file().flush()?;
    tmp.as_file().sync_all()?;

    // Atomically rename to the target path.
    tmp.persist(path)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn tmp() -> TempDir {
        tempfile::tempdir().unwrap()
    }

    // -----------------------------------------------------------------------
    // atomic_write
    // -----------------------------------------------------------------------

    #[test]
    fn write_creates_file_with_correct_content() {
        let dir = tmp();
        let path = dir.path().join("test.bin");
        let data = b"hello world";

        atomic_write(&path, data, None).unwrap();
        assert_eq!(fs::read(&path).unwrap(), data);
    }

    #[test]
    fn write_creates_parent_directories() {
        let dir = tmp();
        let path = dir.path().join("a").join("b").join("deep.bin");

        atomic_write(&path, b"nested", None).unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"nested");
    }

    #[test]
    fn overwrite_replaces_existing_file() {
        let dir = tmp();
        let path = dir.path().join("overwrite.bin");

        atomic_write(&path, b"first", None).unwrap();
        atomic_write(&path, b"second", None).unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"second");
    }

    #[test]
    fn no_temp_file_left_after_success() {
        let dir = tmp();
        let path = dir.path().join("clean.bin");

        atomic_write(&path, b"ok", None).unwrap();

        let entries: Vec<_> = fs::read_dir(dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .collect();
        // Only the target file should remain; no `.tmp.*` leftovers.
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].file_name(), "clean.bin");
    }

    #[test]
    fn write_empty_data() {
        let dir = tmp();
        let path = dir.path().join("empty.bin");

        atomic_write(&path, b"", None).unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"");
    }

    // -----------------------------------------------------------------------
    // Unix-specific: file permissions
    // -----------------------------------------------------------------------

    // OHOS MAC policy: chmod returns success but the actual mode is forced to
    // (owner|0o6)(group|0o6)0, i.e. owner+group always get at least rw and
    // other is always 0. The 0o600/0o400 assertions cannot be satisfied on
    // OHOS, so the entire unix permission test mod is cfg-gated.
    #[cfg(all(unix, not(target_env = "ohos")))]
    mod unix {
        use super::*;
        use std::os::unix::fs::PermissionsExt;

        #[test]
        fn write_sets_file_permissions() {
            let dir = tmp();
            let path = dir.path().join("secret.bin");

            atomic_write(&path, b"secret", Some(0o600)).unwrap();

            let perms = fs::metadata(&path).unwrap().permissions();
            assert_eq!(perms.mode() & 0o777, 0o600);
        }

        #[test]
        fn write_sets_readonly_permissions() {
            let dir = tmp();
            let path = dir.path().join("readonly.bin");

            atomic_write(&path, b"ro", Some(0o400)).unwrap();

            let perms = fs::metadata(&path).unwrap().permissions();
            assert_eq!(perms.mode() & 0o777, 0o400);
        }
    }
}
