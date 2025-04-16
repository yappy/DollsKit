//! システム情報。

use super::SystemModule;
use chrono::prelude::*;

/// システム情報構造体。
#[derive(Clone)]
pub struct SystemInfo {
    /// 起動時間。
    pub started: chrono::DateTime<Local>,
}

impl SystemModule for SystemInfo {}

impl Default for SystemInfo {
    fn default() -> Self {
        Self::new()
    }
}

impl SystemInfo {
    pub fn new() -> Self {
        SystemInfo {
            started: Local::now(),
        }
    }
}
