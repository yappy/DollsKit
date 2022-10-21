//! システムモジュール関連。

pub mod sysinfo;
pub mod twitter;

use crate::sys::taskserver::Control;
use self::{sysinfo::SystemInfo, twitter::Twitter};
use chrono::NaiveTime;
use log::{info};
use std::sync::Arc;

pub trait SystemModule : Sync + Send {
    /// [SystemModule] の初期化時には [crate::sys::taskserver::TaskServer] がまだ存在しないので
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
        info!("initialize system modules...");

        let wakeup_twiter: Vec<_> = (0..24)
            .flat_map(|hour| {
                (0..60)
                    .step_by(5)
                    .map(move |min| NaiveTime::from_hms(hour, min, 0))
            }).collect();

        let mut event_target_list: Vec<Arc<dyn SystemModule>>= vec![];

        let sysinfo = Arc::new(SystemInfo::new());
        event_target_list.push(sysinfo.clone());
        let twitter = Arc::new(Twitter::new(wakeup_twiter));
        event_target_list.push(twitter.clone());

        info!("OK: initialize system modules");
        Self { sysinfo, twitter , event_target_list }
    }

    pub fn on_start(&self, ctrl: &Control) {
        info!("invoke on_start for system modules...");
        for sysmod in self.event_target_list.iter() {
            sysmod.on_start(ctrl);
        }
        info!("OK: invoke on_start for system modules");
    }
}
