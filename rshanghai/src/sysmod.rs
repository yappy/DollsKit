//! システムモジュール関連。

pub mod health;
pub mod http;
pub mod sysinfo;
pub mod twitter;

use self::{sysinfo::SystemInfo, twitter::Twitter};
use crate::{
    sys::taskserver::Control,
    sysmod::{health::Health, http::HttpServer},
};
use anyhow::Result;
use chrono::NaiveTime;
use log::info;
use std::sync::Arc;
use tokio::sync::Mutex as TokioMutex;

/// システムモジュールが実装するトレイト。
pub trait SystemModule: Sync + Send {
    /// [SystemModule] の初期化時には [crate::sys::taskserver::TaskServer] がまだ存在しないので
    /// タスクの登録はこのタイミングまで遅延させる。
    fn on_start(&self, _ctrl: &Control) {}
}

/// [SystemModules] 内の [SystemModule] はマルチスレッドにアクセスされるため、
/// ロックが必要かつ await 可能。
type SysModArc<T> = Arc<TokioMutex<T>>;

/// タスクのエントリポイントに渡される引数からアクセス可能な [SystemModule] のリスト。
/// デッドロックに注意。
///
/// ## デッドロックについて
/// それぞれの [SystemModule] はアクセスする前にロックを取得する必要があるが、
/// 複数同時にロックする場合、その順番に気を付けないと
/// デッドロックを引き起こす可能性がある。
pub struct SystemModules {
    pub sysinfo: SysModArc<sysinfo::SystemInfo>,
    pub health: SysModArc<health::Health>,
    pub twitter: SysModArc<twitter::Twitter>,
    pub http: SysModArc<http::HttpServer>,

    /// 全 [SystemModule] にイベントを配送するための参照のリストを作る。
    event_target_list: Vec<SysModArc<dyn SystemModule>>,
}

impl SystemModules {
    pub fn new() -> Result<SystemModules> {
        info!("initialize system modules...");

        let wakeup_health_ck: Vec<_> = (0..24)
            .flat_map(|hour| (0..60).map(move |min| NaiveTime::from_hms(hour, min, 0)))
            .collect();
        let wakeup_health_tw = vec![
            NaiveTime::from_hms(0, 0, 0),
            NaiveTime::from_hms(6, 0, 0),
            NaiveTime::from_hms(12, 0, 0),
            NaiveTime::from_hms(18, 0, 0),
        ];

        let wakeup_twiter: Vec<_> = (0..24)
            .flat_map(|hour| {
                (0..60)
                    .step_by(5)
                    .map(move |min| NaiveTime::from_hms(hour, min, 0))
            })
            .collect();

        let mut event_target_list: Vec<SysModArc<dyn SystemModule>> = vec![];

        let sysinfo = Arc::new(TokioMutex::new(SystemInfo::new()));
        let health = Arc::new(TokioMutex::new(Health::new(
            wakeup_health_ck,
            wakeup_health_tw,
        )?));
        let twitter = Arc::new(TokioMutex::new(Twitter::new(wakeup_twiter)?));
        let http = Arc::new(TokioMutex::new(HttpServer::new()?));

        event_target_list.push(sysinfo.clone());
        event_target_list.push(health.clone());
        event_target_list.push(twitter.clone());
        event_target_list.push(http.clone());

        info!("OK: initialize system modules");
        Ok(Self {
            sysinfo,
            health,
            twitter,
            http,
            event_target_list,
        })
    }

    pub async fn on_start(&self, ctrl: &Control) {
        info!("invoke on_start for system modules...");
        for sysmod in self.event_target_list.iter() {
            sysmod.lock().await.on_start(ctrl);
        }
        info!("OK: invoke on_start for system modules");
    }
}
