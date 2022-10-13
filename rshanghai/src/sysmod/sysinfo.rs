use super::SystemModule;
use std::sync::RwLock;
use chrono::prelude::*;

pub struct Info {
    started: chrono::DateTime<Local>,
}

pub struct SystemInfo {
    info: RwLock<Info>,
}

impl SystemModule for SystemInfo {}

impl SystemInfo {
    pub fn new() -> Self {
        let info = RwLock::new(Info {
            started: Local::now()
        });

        SystemInfo { info }
    }

    pub fn get<F>(&self, f: F)
        where F: FnOnce(&Info)
    {
        f(&self.info.read().unwrap());
    }

    pub fn set<F>(&self, f: F)
        where F: FnOnce(&mut Info)
    {
        f(&mut self.info.write().unwrap());
    }
}
