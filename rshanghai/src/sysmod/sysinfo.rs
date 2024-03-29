use super::SystemModule;
use chrono::prelude::*;

#[derive(Clone)]
pub struct SystemInfo {
    pub started: chrono::DateTime<Local>,
}

impl SystemModule for SystemInfo {}

impl SystemInfo {
    pub fn new() -> Self {
        SystemInfo {
            started: Local::now(),
        }
    }
}
