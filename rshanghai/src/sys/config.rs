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
use crate::sysmod::line::LineConfig;
use crate::sysmod::openai::OpenAiConfig;
use crate::sysmod::twitter::TwitterConfig;

/// ロードする設定ファイルパス。
const CONFIG_FILE: &str = "config.toml";
/// デフォルト設定の出力パス。
const CONFIG_DEF_FILE: &str = "config_default.toml";
/// 現在設定の出力パス。
const CONFIG_CUR_FILE: &str = "config_current.toml";

/// 設定データ(グローバル変数)。
static CONFIG: RwLock<Option<Config>> = RwLock::new(None);

/// 設定データ。
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub health: HealthConfig,
    #[serde(default)]
    pub camera: CameraConfig,
    #[serde(default)]
    pub twitter: TwitterConfig,
    #[serde(default)]
    pub discord: DiscordConfig,
    #[serde(default)]
    pub line: LineConfig,
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
        let main_cfg: Config = Default::default();
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

    // グローバル変数に設定する
    let mut config = CONFIG.write().unwrap();
    *config = Some(toml::from_str(&toml_str)?);

    {
        // 現在設定ファイルを削除する
        info!("remove {}", CONFIG_CUR_FILE);
        if let Err(e) = remove_file(CONFIG_CUR_FILE) {
            warn!(
                "removing {} failed (the first time execution?): {}",
                CONFIG_CUR_FILE, e
            );
        }
        // 現在設定を書き出す
        // permission=600 でアトミックに必ず新規作成する、失敗したらエラー
        info!("writing current config to {}", CONFIG_CUR_FILE);
        let main_toml = toml::to_string(&*config)?;
        let mut f = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(CONFIG_CUR_FILE)
            .with_context(|| format!("Failed to open {CONFIG_CUR_FILE}"))?;
        f.write_all(main_toml.as_bytes())
            .with_context(|| format!("Failed to write {CONFIG_CUR_FILE}"))?;
        info!("OK: written to {}", CONFIG_CUR_FILE);
        // close
    }

    Ok(())
}

pub struct ConfigGuard;

impl Drop for ConfigGuard {
    fn drop(&mut self) {
        unset();
    }
}

#[must_use]
pub fn set(cfg: Config) -> ConfigGuard {
    let mut config = CONFIG.write().unwrap();
    assert!(config.is_none());
    *config = Some(cfg);
    ConfigGuard
}

pub fn unset() {
    let mut config = CONFIG.write().unwrap();
    assert!(config.is_some());
    *config = None;
}

pub fn get<F, R>(f: F) -> R
where
    F: FnOnce(&Config) -> R,
{
    let config = CONFIG.read().unwrap();
    f(config.as_ref().unwrap())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    #[serial(config)]
    fn test_if() {
        let _unset = set(Default::default());
        get(|cfg| println!("{:?}", cfg));
    }
}
