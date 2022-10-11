//! システムモジュール関連。

pub mod sysinfo;

use self::sysinfo::SystemInfo;

pub struct SystemModules {
    sysinfo: sysinfo::SystemInfo,
}

impl SystemModules {
    pub fn create() -> SystemModules {
        let sysinfo = SystemInfo::new();

        Self { sysinfo }
    }
}
