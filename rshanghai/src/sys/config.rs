//! 設定データの管理。

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::sync::RwLock;

pub use crate::sysmod::camera::CameraConfig;
pub use crate::sysmod::discord::DiscordConfig;
pub use crate::sysmod::health::HealthConfig;
pub use crate::sysmod::http::HttpConfig;
use crate::sysmod::openai::OpenAiConfig;
pub use crate::sysmod::openai::OpenAiPrompt;
pub use crate::sysmod::twitter::TwitterConfig;
pub use crate::sysmod::twitter::TwitterContents;

/// 設定データ(グローバル変数)。
static CONFIG: RwLock<Option<ConfigData>> = RwLock::new(None);

/// グローバルに保持するデータ。
#[derive(Default)]
struct ConfigData {
    main: MainConfig,
}

#[derive(Default, Serialize, Deserialize)]
struct MainConfig {
    health: HealthConfig,
    camera: CameraConfig,
    twitter: TwitterConfig,
    discord: DiscordConfig,
    openai: OpenAiConfig,
    http: HttpConfig,
}

/// 設定データを初期化する。
///
/// 全データを [Default] で上書きする。
pub fn init() {
    let mut config = CONFIG.write().unwrap();
    *config = Some(Default::default());
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
}
