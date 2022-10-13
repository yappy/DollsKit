//! システムモジュール関連。

pub mod sysinfo;
pub mod twitter;

use crate::sys::taskserver::Control;
use self::{sysinfo::SystemInfo, twitter::Twitter};
use std::sync::Arc;

trait SystemModule : Sync + Send {
    /// [SystemModule] の初期化時には [TaskServer] がまだ存在しないので
    /// タスクの登録はこのタイミングまで遅延させる。
    fn on_start(&self, _ctrl: &Control) {}
}

pub struct SystemModules {
    pub sysinfo: Arc<sysinfo::SystemInfo>,
    pub twitter: Arc<twitter::Twitter>,

    event_target_list: Vec<Arc<dyn SystemModule>>,
}

impl SystemModules {
    pub fn new() -> SystemModules {
        let mut event_target_list: Vec<Arc<dyn SystemModule>>= vec![];

        let sysinfo = Arc::new(SystemInfo::new());
        event_target_list.push(sysinfo.clone());
        let twitter = Arc::new(Twitter::new());
        event_target_list.push(twitter.clone());

        Self { sysinfo, twitter , event_target_list }
    }

    pub fn on_start(&self, ctrl: &Control) {
        for sysmod in self.event_target_list.iter() {
            sysmod.on_start(ctrl);
        }
    }
}
