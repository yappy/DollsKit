//! ディレクトリ関連。

use anyhow::{Context, Result};
use std::path::PathBuf;

const APP_NAME: &str = "shanghai";

/// e.g. `$HOME/.config/shanghai`
///
/// https://specifications.freedesktop.org/basedir/latest/
pub fn config_dir() -> Result<PathBuf> {
    let config_dir = dirs::config_local_dir().context("Cannot get config dir")?;

    Ok(config_dir.join(APP_NAME))
}

/// e.g. `$HOME/.local/share/shanghai`
///
/// https://specifications.freedesktop.org/basedir/latest/
pub fn share_dir() -> Result<PathBuf> {
    let data_dir = dirs::data_local_dir().context("Cannot get data dir")?;

    Ok(data_dir.join(APP_NAME))
}

/// e.g. `$HOME/.cache/shanghai
///
/// https://specifications.freedesktop.org/basedir/latest/
pub fn cache_dir() -> Result<PathBuf> {
    let home_dir = dirs::cache_dir().context("Cannot get home dir")?;

    Ok(home_dir.join(APP_NAME))
}
