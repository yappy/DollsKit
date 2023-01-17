//! システムモジュール関連。

pub mod camera;
pub mod discord;
pub mod health;
pub mod http;
pub mod sysinfo;
pub mod twitter;

use self::{
    camera::Camera, discord::Discord, health::Health, http::HttpServer, sysinfo::SystemInfo,
    twitter::Twitter,
};
use crate::sys::taskserver::Control;
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
    pub camera: SysModArc<camera::Camera>,
    pub twitter: SysModArc<twitter::Twitter>,
    pub discord: SysModArc<discord::Discord>,
    pub http: SysModArc<http::HttpServer>,

    /// 全 [SystemModule] にイベントを配送するための参照のリストを作る。
    event_target_list: Vec<SysModArc<dyn SystemModule>>,
}

impl SystemModules {
    pub fn new() -> Result<SystemModules> {
        info!("initialize system modules...");

        let wakeup_health_ck: Vec<_> = (0..24)
            .flat_map(|hour| (0..60).map(move |min| NaiveTime::from_hms_opt(hour, min, 0).unwrap()))
            .collect();
        let wakeup_health_tw = vec![
            NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
            NaiveTime::from_hms_opt(6, 0, 0).unwrap(),
            NaiveTime::from_hms_opt(12, 0, 0).unwrap(),
            NaiveTime::from_hms_opt(18, 0, 0).unwrap(),
        ];

        let wakeup_camera = vec![
            NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
            NaiveTime::from_hms_opt(3, 0, 0).unwrap(),
            NaiveTime::from_hms_opt(6, 0, 0).unwrap(),
            NaiveTime::from_hms_opt(9, 0, 0).unwrap(),
            NaiveTime::from_hms_opt(12, 0, 0).unwrap(),
            NaiveTime::from_hms_opt(15, 0, 0).unwrap(),
            NaiveTime::from_hms_opt(18, 0, 0).unwrap(),
            NaiveTime::from_hms_opt(21, 0, 0).unwrap(),
        ];

        let wakeup_twiter: Vec<_> = (0..24)
            .flat_map(|hour| {
                (0..60)
                    .step_by(5)
                    .map(move |min| NaiveTime::from_hms_opt(hour, min, 0).unwrap())
            })
            .collect();

        let wakeup_discord: Vec<_> = (0..24)
            .flat_map(|hour| {
                (0..60)
                    .step_by(10)
                    .map(move |min| NaiveTime::from_hms_opt(hour, min, 0).unwrap())
            })
            .collect();

        let mut event_target_list: Vec<SysModArc<dyn SystemModule>> = vec![];

        let sysinfo = Arc::new(TokioMutex::new(SystemInfo::new()));
        let health = Arc::new(TokioMutex::new(Health::new(
            wakeup_health_ck,
            wakeup_health_tw,
        )?));
        let camera = Arc::new(TokioMutex::new(Camera::new(wakeup_camera)?));
        let twitter = Arc::new(TokioMutex::new(Twitter::new(wakeup_twiter)?));
        let http = Arc::new(TokioMutex::new(HttpServer::new()?));
        let discord = Arc::new(TokioMutex::new(Discord::new(wakeup_discord)?));

        event_target_list.push(sysinfo.clone());
        event_target_list.push(health.clone());
        event_target_list.push(camera.clone());
        event_target_list.push(twitter.clone());
        event_target_list.push(http.clone());
        event_target_list.push(discord.clone());

        info!("OK: initialize system modules");
        Ok(Self {
            sysinfo,
            health,
            camera,
            twitter,
            discord,
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
