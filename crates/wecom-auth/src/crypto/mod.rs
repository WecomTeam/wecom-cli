//! Crypto utilities for secure credential storage.
//!
//! AES-256-GCM 加密（随机 nonce，输出 `nonce || ciphertext || tag`），
//! 密钥经系统 keyring 持久化、文件回退，详见 [`keystore`]。

pub mod cipher;
mod keystore;

pub(crate) use keystore::{
    encrypt_data, encryption_key_path, generate_random_key, load_key_from_file,
    load_key_from_keyring, save_key, try_decrypt_data,
};

use std::path::Path;

use crate::error::AuthError;

/// Atomically write `data` (bytes) to `path` (temp file in the same directory → atomic rename).
///
/// 先写同目录临时文件（persist 前设置权限，避免目标路径短暂可见过高权限），
/// fsync 后原子 rename 到目标路径。
pub(crate) fn atomic_write(path: &Path, data: &[u8], mode: u32) -> Result<(), AuthError> {
    let parent = path
        .parent()
        .ok_or_else(|| AuthError::Storage(format!("无效文件路径: {}", path.display())))?;

    std::fs::create_dir_all(parent)
        .map_err(|e| AuthError::Storage(format!("创建目录 {} 失败: {e}", parent.display())))?;

    let mut tmp = tempfile::NamedTempFile::new_in(parent)
        .map_err(|e| AuthError::Storage(format!("在 {parent:?} 中创建临时文件失败: {e}")))?;

    // Set permissions on the temp file *before* persisting so the file is
    // never visible at the target path with overly-permissive mode.
    #[cfg(unix)]
    {
        use std::io::Write as _;
        use std::os::unix::fs::PermissionsExt;
        tmp.as_file()
            .set_permissions(std::fs::Permissions::from_mode(mode))
            .map_err(|e| AuthError::Storage(format!("设置临时文件权限失败: {e}")))?;
        tmp.write_all(data)
            .map_err(|e| AuthError::Storage(format!("写入临时文件失败: {e}")))?;
    }
    #[cfg(not(unix))]
    {
        use std::io::Write as _;
        tmp.write_all(data)
            .map_err(|e| AuthError::Storage(format!("写入临时文件失败: {e}")))?;
    }

    tmp.as_file()
        .sync_all()
        .map_err(|e| AuthError::Storage(format!("同步临时文件失败: {e}")))?;

    tmp.persist(path)
        .map_err(|e| AuthError::Storage(format!("原子写入 {} 失败: {e}", path.display())))?;
    Ok(())
}
