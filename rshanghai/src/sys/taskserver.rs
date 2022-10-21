//! 非同期タスクを管理する。
use crate::sysmod::SystemModules;
use std::future::Future;
use std::sync::Arc;
use chrono::prelude::*;
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender, UnboundedReceiver};
use log::{error, info, trace};


type ShutdownTx = UnboundedSender<()>;
type ShutdownRx = UnboundedReceiver<()>;

/// [Control] の [Arc] 内データ。
struct InternalControl {
    rt: tokio::runtime::Runtime,
    sysmods: SystemModules,
    shutdown_tx: ShutdownTx,
    shutdown_rx: ShutdownRx,
}

/// [Arc] により [TaskServer] と全非同期タスク間で共有されるコントロールハンドル。
///
/// [Clone] 可能で、複製すると [Arc] がインクリメントされる。
#[derive(Clone)]
pub struct Control {
    /// private で [InternalControl] への [Arc] を持つ。
    internal: Arc<InternalControl>,
}

/// タスクサーバ本体。
pub struct TaskServer {
    /// [TaskServer] も [Control] への参照を1つ持つ。
    ctrl: Control,
}

impl Control {
    /// 1回限りのタスクを生成して実行開始する。
    ///
    /// F: [Control] を引数に、T を返す関数。
    /// T: Future<Output = Result<(), R> かつスレッド間移動可能。
    /// R: [ToString::to_string()] 可能。
    ///
    /// つまり、F は [Control] を引数に、Result<(), R> を返す async function。
    /// R は to_string() 可能な型。
    pub fn spawn_oneshot_task<F, T, R>(&self, name: &str, f: F)
    where
        F: FnOnce(Control) -> T,
        T: Future<Output = Result<(), R>> + Send + 'static,
        R: ToString,
    {
        // move するデータを準備する
        let name = name.to_string();
        let ctrl = self.clone();
        let future = f(ctrl);

        self.internal.rt.spawn(async move {
            info!("[{}] start (one-shot)", name);
            let result = future.await;
            if let Err(r) = result {
                error!("[{}] finish (error): {}", name, r.to_string());
            }
            else {
                info!("[{}] finish (success)", name);
            }
        });
    }

    /// time_list
    pub fn spawn_periodic_task<F, T, R>(
        &self, name: &str, time_list: &[NaiveTime], f: F)
    where
        F: Fn(Control) -> T + Send + 'static,
        T: Future<Output = Result<(), R>> + Send + 'static,
        R: ToString + Send,
    {
        // move するデータを準備する
        let name = name.to_string();
        let ctrl = self.clone();

        // 空でなくソート済み、秒以下がゼロなのを確認後
        // 今日のその時刻からなる Local DateTime に変換する
        assert!(!time_list.is_empty(), "time list is empty");
        let sorted = time_list.windows(2).all(|t| t[0] <= t[1]);
        assert!(sorted, "time list is not sorted");
        let today = Local::today();
        let mut dt_list: Vec<_> = time_list.iter().map(|time| {
            assert!(time.second() == 0);
            assert!(time.nanosecond() == 0);
            today.and_time(*time).unwrap()
        }).collect();

        self.internal.rt.spawn(async move {
            type CDuration = chrono::Duration;
            type TDuration = tokio::time::Duration;
            info!("[{}] registered as periodic task", name);

            loop {
                // 現在時刻を取得して分までに切り捨てる
                let now = Local::now();
                let now_hmd = now
                    .date()
                    .and_hms(now.hour(), now.minute(), 0);
                let next_min = now_hmd + CDuration::minutes(1);
                trace!("[{}] periodic task check: {}", name, now_hmd);

                // 起動時刻リスト内で二分探索
                match dt_list.binary_search(&now_hmd) {
                    Ok(ind) => {
                        // 一致するものを発見したので続行
                        trace!("[{}] hit in time list: {}", name, now_hmd);
                    },
                    Err(ind) => {
                        // ind = insertion point
                        trace!("[{}] not found in time list: {}", name, now_hmd);
                        // 起きるべき時刻は dt_list[ind]
                        if ind < dt_list.len() {
                            let target_dt = dt_list[ind] + CDuration::seconds(1);
                            let sleep_duration = target_dt - Local::now();
                            let sleep_sec = sleep_duration.num_seconds().clamp(0, i64::MAX) as u64;
                            trace!("[{}] target: {}, sleep_sec: {}", name, target_dt, sleep_sec);
                            tokio::time::sleep(TDuration::from_secs(sleep_sec)).await;
                            trace!("[{}] wake up", name);
                        }
                        else {
                            // 一番後ろよりも現在時刻が後
                            // 起動時刻リストをすべて1日ずつ後ろにずらす
                            for dt in dt_list.iter_mut() {
                                let tomorrow = dt.date() + CDuration::days(1);
                                let time = dt.time();
                                *dt = tomorrow.and_time(time).unwrap();
                                trace!("[{}] advance time list by 1 day", name);
                            }
                        }
                        continue;
                    },
                }

                let future = f(ctrl.clone());
                info!("[{}] start (periodic)", name);
                let result = future.await;
                if let Err(r) = result {
                    error!("[{}] finish (error): {}", name, r.to_string());
                }
                else {
                    info!("[{}] finish (success)", name);
                }

                // 次の "分" を狙って sleep する
                // 目標は安全のため hh:mm:05 を狙う
                let target_dt = next_min + CDuration::seconds(5);
                // タスクの実行に1分以上かかると負になるが、
                // chrono::Duration は負数を許している
                // その場合は 0 に補正する
                let sleep_duration = target_dt - Local::now();
                let sleep_sec = sleep_duration.num_seconds().clamp(0, i64::MAX) as u64;
                trace!("[{}] target: {}, sleep_sec: {}", name, target_dt, sleep_sec);
                tokio::time::sleep(tokio::time::Duration::from_secs(sleep_sec)).await;
                trace!("[{}] wake up", name);
            }
        });
    }

    /// [crate::sysmod::SystemModule] リストを取得する。
    pub fn sysmods(&self) -> &SystemModules {
        &self.internal.sysmods
    }
}

impl TaskServer {
    /// タスクサーバを初期化して開始する。
    pub fn new(sysmods: SystemModules) -> Self {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();
        let (shutdown_tx, shutdown_rx) = unbounded_channel();

        let internal = InternalControl { rt, sysmods, shutdown_tx, shutdown_rx };
        let ctrl = Control { internal: Arc::new(internal) };
        TaskServer { ctrl }
    }

    pub fn spawn_oneshot_task<F, T, R>(&self, name: &str, f: F)
    where
        F: FnOnce(Control) -> T,
        T: Future<Output = Result<(), R>> + Send + 'static,
        R: ToString,
    {
        self.ctrl.spawn_oneshot_task(name, f);
    }

    pub fn sysmod_start(&self) {
        self.ctrl.internal.sysmods.on_start(&self.ctrl);
    }

    pub fn run(&self) {
        self.ctrl.internal.rt.block_on(async {
            loop {
                // TODO: wait for shutdown
            }
        });
    }

}

fn check_time(now: DateTime<Local>) {
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn datetime() {
        check_time(Local::now());
    }
}
