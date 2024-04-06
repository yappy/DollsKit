//! 非同期タスクを管理する。

use crate::sysmod::SystemModules;
use anyhow::Result;
use chrono::prelude::*;
use log::{error, info, trace};
use std::future::Future;
use std::sync::Arc;
use tokio::select;
use tokio::signal::unix::{signal, SignalKind};
use tokio::sync::watch;

/// システムシャットダウン開始通知送信側 (単数)
type CancelTx = watch::Sender<bool>;
/// システムシャットダウン開始通知受信側 (複数)
type CancelRx = watch::Receiver<bool>;

/// [TaskServer] と各非同期タスク間で共有されるデータ。
///
/// インスタンスは1つだけ生成され、[Arc] により共有される。
pub struct Controller {
    /// Tokio ランタイム。
    rt: tokio::runtime::Runtime,
    /// 全システムモジュールのリスト。
    sysmods: SystemModules,
    /// システムシャットダウン時、true が設定送信される。
    ///
    /// また、シャットダウンシーケンスにおいて、全タスクの完了待ちのためにも使う。
    /// サーバ側は clone されたこれがすべて drop されるまで待機する。
    cancel_rx: std::sync::Mutex<Option<CancelRx>>,
}

pub type Control = Arc<Controller>;
//pub type WeakControl = Weak<Controller>;

/// [TaskServer::run] の返す実行終了種別。
pub enum RunResult {
    Shutdown,
    Reboot,
}

/// タスクサーバ本体。
pub struct TaskServer {
    /// 各タスクに clone して渡すオリジナルの [Control]。
    ctrl: Control,
    /// システムシャットダウン時の中断リクエストの送信側。
    /// <https://tokio.rs/tokio/topics/shutdown>
    /// "Telling things to shut down" + "Waiting for things to finish shutting down"
    cancel_tx: CancelTx,
}

impl Controller {
    /// [crate::sysmod::SystemModule] リストへの参照を取得する。
    pub fn sysmods(&self) -> &SystemModules {
        &self.sysmods
    }

    /// キャンセル通知を待つ。
    pub async fn wait_cancel_rx(&self) {
        // mutex をロックして Receiver を取得し、その clone を作る
        let org = self.cancel_rx.lock().unwrap().as_mut().map(|rx| rx.clone());
        if let Some(mut rx) = org {
            // clone した Receiver 上で待つ
            rx.changed().await.unwrap();
        } else {
            // 既に drop されていた場合はすぐに返る
        }
    }
}

/// 1回限りのタスクを生成して実行開始する。
///
/// F: [Control] を引数に、T を返す関数。
/// T: Future<Output = anyhow::Result<()> かつスレッド間移動可能。
///
/// つまり、F は [Control] を引数に、anyhow::Result<()> を返す async function。
pub fn spawn_oneshot_task<F, T>(ctrl: &Control, name: &str, f: F)
where
    F: Fn(Control) -> T + Send + Sync + 'static,
    T: Future<Output = Result<()>> + Send,
{
    // move するデータを準備する
    let name = name.to_string();
    let ctrl_move = Arc::clone(ctrl);

    ctrl.rt.spawn(async move {
        info!("[{}] start (one-shot)", name);

        // ctrl を clone して future へ move する
        let future = f(Arc::clone(&ctrl_move));
        let result = future.await;
        // drop clone of ctrl

        if let Err(e) = result {
            error!("[{}] finish (error): {:?}", name, e);
        } else {
            info!("[{}] finish (success)", name);
        }
        // drop ctrl
    });
}

pub fn spawn_oneshot_fn<F>(ctrl: &Control, name: &str, f: F)
where
    F: Future<Output = Result<()>> + Send + 'static,
{
    // move するデータを準備する
    let name = name.to_string();

    ctrl.rt.spawn(async move {
        info!("[{}] start (one-shot)", name);

        let result = f.await;

        if let Err(e) = result {
            error!("[{}] finish (error): {:?}", name, e);
        } else {
            info!("[{}] finish (success)", name);
        }
        // drop ctrl
    });
}

/// 周期タスクを生成する。
///
/// wakeup_list: 起動時刻。以下を満たさないと panic する。
/// * second 以下が 0 である。(分単位)
/// * 昇順ソート済みである。
///
/// F: [Control] を引数に、T を返す関数。
/// T: Future<Output = anyhow::Result<()> かつスレッド間移動可能。
///
/// つまり、F は [Control] を引数に、anyhow::Result<()> を返す async function。
pub fn spawn_periodic_task<F, T>(ctrl: &Control, name: &str, wakeup_list: &[NaiveTime], f: F)
where
    F: Fn(Control) -> T + Send + Sync + 'static,
    T: Future<Output = Result<()>> + Send + Sync + 'static,
{
    // move するデータを準備する
    let name = name.to_string();
    let ctrl_move = Arc::clone(ctrl);

    // 空でなくソート済み、秒以下がゼロなのを確認後
    // 今日のその時刻からなる NaiveDateTime に変換する
    // TODO: is_sorted() がまだ unstable
    assert!(!wakeup_list.is_empty(), "time list is empty");
    let sorted = wakeup_list.windows(2).all(|t| t[0] <= t[1]);
    assert!(sorted, "time list is not sorted");
    let today = Local::now().date_naive();
    let mut dt_list: Vec<_> = wakeup_list
        .iter()
        .map(|time| {
            assert_eq!(time.second(), 0);
            assert_eq!(time.nanosecond(), 0);
            today.and_time(*time)
        })
        .collect();

    // wakeup time list を最初の LOG_LIMIT 個までログに出力する
    const LOG_LIMIT: usize = 5;
    let log_iter = wakeup_list.iter().take(LOG_LIMIT);
    let mut str = log_iter.enumerate().fold(String::new(), |sum, (i, v)| {
        let str = if i == 0 {
            format!("{v}")
        } else {
            format!(", {v}")
        };
        sum + &str
    });
    if wakeup_list.len() > LOG_LIMIT {
        str += &format!(", ... ({} items)", wakeup_list.len());
    }
    info!("[{}] registered as a periodic task", name);
    info!("[{}] wakeup time: {}", name, str);

    // spawn async task
    ctrl.rt.spawn(async move {
        type CDuration = chrono::Duration;
        type TDuration = tokio::time::Duration;

        loop {
            // 現在時刻を取得して分までに切り捨てる
            let now = Local::now().naive_local();
            let now_hmd = now.date().and_hms_opt(now.hour(), now.minute(), 0).unwrap();
            let next_min = now_hmd + CDuration::try_minutes(1).unwrap();
            trace!("[{}] periodic task check: {}", name, now_hmd);

            // 起動時刻リスト内で二分探索
            match dt_list.binary_search(&now_hmd) {
                Ok(_ind) => {
                    // 一致するものを発見したので続行
                    trace!("[{}] hit in time list: {}", name, now_hmd);
                }
                Err(ind) => {
                    // ind = insertion point
                    trace!("[{}] not found in time list: {}", name, now_hmd);
                    // 起きるべき時刻は dt_list[ind]
                    if ind < dt_list.len() {
                        let target_dt = dt_list[ind] + CDuration::try_seconds(1).unwrap();
                        let sleep_duration = target_dt - Local::now().naive_local();
                        let sleep_sec = sleep_duration.num_seconds().clamp(0, i64::MAX) as u64;
                        trace!("[{}] target: {}, sleep_sec: {}", name, target_dt, sleep_sec);
                        select! {
                            _ = tokio::time::sleep(TDuration::from_secs(sleep_sec)) => {}
                            _ = ctrl_move.wait_cancel_rx() => {
                                info!("[{}] cancel periodic task", name);
                                return;
                            }
                        }

                        trace!("[{}] wake up", name);
                    } else {
                        // 一番後ろよりも現在時刻が後
                        // 起動時刻リストをすべて1日ずつ後ろにずらす
                        for dt in dt_list.iter_mut() {
                            let tomorrow = dt.date() + CDuration::try_days(1).unwrap();
                            let time = dt.time();
                            *dt = tomorrow.and_time(time);
                            trace!("[{}] advance time list by 1 day", name);
                        }
                    }
                    continue;
                }
            }

            // ctrl を clone して future 内に move する
            let future = f(ctrl_move.clone());
            info!("[{}] start (periodic)", name);
            let result = future.await;
            // drop clone of ctrl
            if let Err(e) = result {
                error!("[{}] finish (error): {:?}", name, e);
            } else {
                info!("[{}] finish (success)", name);
            }

            // 次の "分" を狙って sleep する
            // 目標は安全のため hh:mm:05 を狙う
            let target_dt = next_min + CDuration::try_seconds(5).unwrap();
            // タスクの実行に1分以上かかると負になるが、
            // chrono::Duration は負数を許している
            // その場合は 0 に補正する
            let sleep_duration = target_dt - Local::now().naive_local();
            let sleep_sec = sleep_duration.num_seconds().clamp(0, i64::MAX) as u64;
            trace!("[{}] target: {}, sleep_sec: {}", name, target_dt, sleep_sec);
            select! {
                _ = tokio::time::sleep(tokio::time::Duration::from_secs(sleep_sec)) => {}
                _ = ctrl_move.wait_cancel_rx() => {
                    info!("[{}] cancel periodic task", name);
                    return;
                }
            }
            trace!("[{}] wake up", name);
        }
        // drop ctrl
    });
}

impl TaskServer {
    /// タスクサーバを生成して初期化する。
    pub fn new(sysmods: SystemModules) -> Self {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();

        // tx は self へ move
        // rx は root Control へ move
        let (cancel_tx, cancel_rx) = watch::channel(false);

        let internal = Controller {
            rt,
            sysmods,
            cancel_rx: std::sync::Mutex::new(Some(cancel_rx)),
        };
        let ctrl = Arc::new(internal);

        TaskServer { ctrl, cancel_tx }
    }

    /// [spawn_oneshot_task] を内蔵の [Self::ctrl] を使って呼び出す。
    pub fn spawn_oneshot_task<F, T>(&self, name: &str, f: F)
    where
        F: Fn(Control) -> T + Send + Sync + 'static,
        T: Future<Output = Result<()>> + Send,
    {
        spawn_oneshot_task(&self.ctrl, name, f);
    }

    /// 実行を開始し、完了するまでブロックする。
    ///
    /// self の所有権を consume するため、一度しか実行できない。
    pub fn run(self) -> RunResult {
        let ctrl = Arc::clone(&self.ctrl);
        self.ctrl.rt.block_on(async move {
            // SystemModule 全体に on_start イベントを配送
            ctrl.sysmods.on_start(&ctrl).await;

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

            // 値を true に設定して全タスクにキャンセルリクエストを通知する
            self.cancel_tx.send_replace(true);
            // 全タスク完了待ち
            // オリジナルの cancel_rx を self.ctrl から奪って drop しておく
            drop(ctrl.cancel_rx.lock().unwrap().take());
            // 全 cancel_rx が drop されるまで待つ
            info!("waiting for all tasks to be completed....");
            self.cancel_tx.closed().await;
            info!("OK: all tasks are completed");

            run_result
        })
        // drop self (self.ctrl)
    }
}
