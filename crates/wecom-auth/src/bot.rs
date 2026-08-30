use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

/// Bot 凭据（`botid + secret`）。
///
/// 序列化字段名与旧版凭据文件保持一致（`id` / `secret` / `create_time`），
/// 保证 `credentials.enc` / `bot.enc` 的既有密文可继续解密。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotCredential {
    pub id: String,
    pub secret: String,
    pub create_time: u64,
}

impl BotCredential {
    /// Create a new `BotCredential` with `create_time` set to the current timestamp.
    pub fn new(id: String, secret: String) -> Self {
        let create_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        Self {
            id,
            secret,
            create_time,
        }
    }
}
