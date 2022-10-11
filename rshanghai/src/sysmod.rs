use self::sysinfo::SystemInfo;

pub mod sysinfo;

pub struct SystemModules {
    sysinfo: sysinfo::SystemInfo,
}

impl SystemModules {
    pub fn create() -> SystemModules {
        let sysinfo = SystemInfo::new();

        Self { sysinfo }
    }
}
