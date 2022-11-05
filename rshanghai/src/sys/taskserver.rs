//! 非同期タスクを管理する。
use crate::sysmod::SystemModules;
use anyhow::Result;
use chrono::prelude::*;
use log::{error, info, trace};
use std::future::Future;
use std::sync::Arc;
use tokio::signal::unix::{signal, SignalKind};
use tokio::sync::mpsc;

type ShutdownTx = mpsc::UnboundedSender<()>;
type ShutdownRx = mpsc::UnboundedReceiver<()>;
/// タスク完了通知側 (複数)
type CompleteSyncChild = mpsc::Sender<()>;
/// タスク完了待ち側 (単数)
type CompleteSyncParent = mpsc::Receiver<()>;

/// [Arc] により [TaskServer] と各非同期タスク間で共有されるデータ。
struct InternalControl {
    rt: tokio::runtime::Runtime,
    sysmods: SystemModules,
    shutdown_tx: ShutdownTx,
    shutdown_rx: ShutdownRx,
}

/// 各非同期タスクに clone され渡されるコントロールハンドル。
///
/// 各フィールドは private で、メソッドで機能を提供する。
///
/// [Clone::clone] 可能で、複製すると [Arc] がインクリメントされる。
/// また、全タスク完了待ちのための channel も clone される。
#[derive(Clone)]
pub struct Control {
    /// private で [InternalControl] への [Arc] を持つ。
    internal: Arc<InternalControl>,
    /// サーバ側は clone されたこれがすべて drop されるまで待機する。
    complete_sync: Option<CompleteSyncChild>,
}

pub enum RunResult {
    Shutdown,
    Reboot,
}

/// タスクサーバ本体。
pub struct TaskServer {
    /// 各タスクに clone して渡すオリジナルの [Control]。
    ctrl: Control,
    /// [Control::complete_sync] の対向。
    ///
    /// clone されたすべての [Control::complete_sync] が drop するまで待機する。
    /// <https://tokio.rs/tokio/topics/shutdown>
    complete_sync_wait: CompleteSyncParent,
}

impl Control {
    /// 1回限りのタスクを生成して実行開始する。
    ///
    /// F: [Control] を引数に、T を返す関数。
    /// T: Future<Output = anyhow::Result<()> かつスレッド間移動可能。
    ///
    /// つまり、F は [Control] を引数に、anyhow::Result<()> を返す async function。
    pub fn spawn_oneshot_task<F, T>(&self, name: &str, f: F)
    where
        F: FnOnce(Control) -> T,
        T: Future<Output = Result<()>> + Send + 'static,
    {
        // move するデータを準備する
        let name = name.to_string();
        let ctrl = self.clone();
        let future = f(ctrl);

        self.internal.rt.spawn(async move {
            info!("[{}] start (one-shot)", name);

            let result = future.await;

            if let Err(e) = result {
                error!("[{}] finish (error): {:?}", name, e);
            } else {
                info!("[{}] finish (success)", name);
            }
        });
    }

    /// time_list
    pub fn spawn_periodic_task<F, T>(&self, name: &str, wakeup_list: &[NaiveTime], f: F)
    where
        F: Fn(Control) -> T + Send + 'static,
        T: Future<Output = Result<()>> + Send + 'static,
    {
        // move するデータを準備する
        let name = name.to_string();
        let ctrl = self.clone();

        // 空でなくソート済み、秒以下がゼロなのを確認後
        // 今日のその時刻からなる Local DateTime に変換する
        // TODO: is_sorted() がまだ unstable
        assert!(!wakeup_list.is_empty(), "time list is empty");
        let sorted = wakeup_list.windows(2).all(|t| t[0] <= t[1]);
        assert!(sorted, "time list is not sorted");
        let today = Local::today();
        let mut dt_list: Vec<_> = wakeup_list
            .iter()
            .map(|time| {
                assert_eq!(time.second(), 0);
                assert_eq!(time.nanosecond(), 0);
                today.and_time(*time).unwrap()
            })
            .collect();

        // wakeup time list を最初の LOG_LIMIT 個までログに出力する
        const LOG_LIMIT: usize = 5;
        let log_iter = wakeup_list.iter().take(LOG_LIMIT);
        let mut str = log_iter.enumerate().fold(String::new(), |sum, (i, v)| {
            let str = if i == 0 {
                format!("{}", v)
            } else {
                format!(", {}", v)
            };
            sum + &str
        });
        if wakeup_list.len() > LOG_LIMIT {
            str += &format!(", ... ({} items)", wakeup_list.len());
        }
        info!("[{}] registered as a periodic task", name);
        info!("[{}] wakeup time: {}", name, str);

        self.internal.rt.spawn(async move {
            type CDuration = chrono::Duration;
            type TDuration = tokio::time::Duration;

            loop {
                // 現在時刻を取得して分までに切り捨てる
                let now = Local::now();
                let now_hmd = now.date().and_hms(now.hour(), now.minute(), 0);
                let next_min = now_hmd + CDuration::minutes(1);
                trace!("[{}] periodic task check: {}", name, now_hmd);

                // 起動時刻リスト内で二分探索
                match dt_list.binary_search(&now_hmd) {
                    Ok(ind) => {
                        // 一致するものを発見したので続行
                        trace!("[{}] hit in time list: {}", name, now_hmd);
                    }
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
                        } else {
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
                    }
                }

                let future = f(ctrl.clone());
                info!("[{}] start (periodic)", name);
                let result = future.await;
                if let Err(e) = result {
                    error!("[{}] finish (error): {:?}", name, e);
                } else {
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

        let (shutdown_tx, shutdown_rx) = mpsc::unbounded_channel();
        let (complete_sync, complete_sync_wait) = mpsc::channel(1);

        let internal = InternalControl {
            rt,
            sysmods,
            shutdown_tx,
            shutdown_rx,
        };
        let ctrl = Control {
            internal: Arc::new(internal),
            complete_sync: Some(complete_sync),
        };

        TaskServer {
            ctrl,
            complete_sync_wait,
        }
    }

    pub fn spawn_oneshot_task<F, T>(&self, name: &str, f: F)
    where
        F: FnOnce(Control) -> T,
        T: Future<Output = Result<()>> + Send + 'static,
    {
        self.ctrl.spawn_oneshot_task(name, f);
    }

    /// 実行を開始し、完了するまでブロックする。
    ///
    /// self の所有権は consume する。一度しか実行できない。
    pub fn run(mut self) -> RunResult {
        // async block へ move するためのコピーを作る
        let ctrl = self.ctrl.clone();
        // オリジナルの complete_sync を self から奪って drop しておく
        drop(self.ctrl.complete_sync.take().unwrap());

        self.ctrl.internal.rt.block_on(async move {
            // SystemModule 全体に on_start イベントを配送
            ctrl.internal.sysmods.on_start(&ctrl).await;

            // この async block をシグナル処理に使う
            let mut sigint = signal(SignalKind::interrupt()).unwrap();
            let mut sigterm = signal(SignalKind::terminate()).unwrap();
            let mut sighup = signal(SignalKind::hangup()).unwrap();

            let run_result;
            tokio::select! {
                _ = sigint.recv() => {
                    info!("[signal] SIGINT");
                    run_result = RunResult::Shutdown;
                },
                _ = sigterm.recv() => {
                    info!("[signal] SIGTERM");
                    run_result = RunResult::Shutdown;
                },
                _ = sighup.recv() => {
                    info!("[signal] SIGHUP");
                    run_result = RunResult::Reboot;
                },
            }

            // 全タスク完了待ち
            // この async block で持っている分の complete_sync を
            // ctrl ごと drop する
            // オリジナルの self.ctrl.complete_sync も drop 済みなので
            // 残りは各 async task に clone された Control
            drop(ctrl);
            // 全 complete_sync (Sender) が drop されるまで待つ
            // その時 None を返す
            // それ以外の用法はしないので、データは送信されてこない
            info!("waiting for all tasks to be completed....");
            let sync_result = self.complete_sync_wait.recv().await;
            assert!(sync_result.is_none());
            info!("OK: all tasks are completed");

            run_result
        })
    }
}
