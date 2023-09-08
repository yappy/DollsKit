//! 設定データの管理。

use anyhow::{ensure, Context, Result};
use log::info;
use log::warn;
use serde::{Deserialize, Serialize};
use std::fs::remove_file;
use std::fs::OpenOptions;
use std::io::{Read, Write};
use std::os::unix::prelude::*;
use std::sync::RwLock;

use crate::sysmod::camera::CameraConfig;
use crate::sysmod::discord::DiscordConfig;
use crate::sysmod::health::HealthConfig;
use crate::sysmod::http::HttpConfig;
use crate::sysmod::openai::OpenAiConfig;
use crate::sysmod::twitter::TwitterConfig;

/// ロードする設定ファイルパス。
const CONFIG_FILE: &str = "config.toml";
/// デフォルト設定の出力パス。
const CONFIG_DEF_FILE: &str = "config_default.toml";

/// 設定データ(グローバル変数)。
static CONFIG: RwLock<Option<ConfigData>> = RwLock::new(None);

/// グローバルに保持するデータ。
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ConfigData {
    #[serde(default)]
    pub health: HealthConfig,
    #[serde(default)]
    pub camera: CameraConfig,
    #[serde(default)]
    pub twitter: TwitterConfig,
    #[serde(default)]
    pub discord: DiscordConfig,
    #[serde(default)]
    pub openai: OpenAiConfig,
    #[serde(default)]
    pub http: HttpConfig,
}

/// 設定データをロードする。
pub fn load() -> Result<()> {
    {
        // デフォルト設定ファイルを削除する
        info!("remove {}", CONFIG_DEF_FILE);
        if let Err(e) = remove_file(CONFIG_DEF_FILE) {
            warn!(
                "removing {} failed (the first time execution?): {}",
                CONFIG_DEF_FILE, e
            );
        }
        // デフォルト設定を書き出す
        // permission=600 でアトミックに必ず新規作成する、失敗したらエラー
        info!("writing default config to {}", CONFIG_DEF_FILE);
        let main_cfg: ConfigData = Default::default();
        let main_toml = toml::to_string(&main_cfg)?;
        let mut f = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(CONFIG_DEF_FILE)
            .with_context(|| format!("Failed to open {CONFIG_DEF_FILE}"))?;
        f.write_all(main_toml.as_bytes())
            .with_context(|| format!("Failed to write {CONFIG_DEF_FILE}"))?;
        info!("OK: written to {}", CONFIG_DEF_FILE);
        // close
    }

    let toml_str = {
        // 設定ファイルを読む
        // open 後パーミッションを確認し、危険ならエラーとする
        info!("loading config: {}", CONFIG_FILE);
        let mut f = OpenOptions::new()
            .read(true)
            .open(CONFIG_FILE)
            .with_context(|| format!("Failed to open {CONFIG_FILE} (the first execution?)"))
            .with_context(|| {
                format!("HINT: Copy {CONFIG_DEF_FILE} to {CONFIG_FILE} and try again")
            })?;

        let metadata = f.metadata()?;
        let permissions = metadata.permissions();
        let masked = permissions.mode() & 0o777;
        ensure!(
            masked == 0o600,
            "Config file permission is not 600: {:03o}",
            permissions.mode()
        );

        let mut toml_str = String::new();
        f.read_to_string(&mut toml_str)
            .with_context(|| format!("Failed to read {CONFIG_FILE}"))?;
        info!("OK: {} loaded", CONFIG_FILE);

        toml_str
        // close f
    };

    {
        let mut config = CONFIG.write().unwrap();
        *config = Some(toml::from_str(&toml_str)?);
    }

    Ok(())
}

pub fn get<F, R>(f: F) -> R
where
    F: FnOnce(&ConfigData) -> R,
{
    let config = CONFIG.read().unwrap();
    f(config.as_ref().unwrap())
}

#[cfg(test)]
mod tests {}
