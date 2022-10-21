//! 非同期タスクを管理する。
use std::future::Future;
use std::sync::Arc;
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender, UnboundedReceiver};
use crate::sysmod::SystemModules;

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
            info!("[{}] start", name);
            let result = future.await;
            if let Err(r) = result {
                error!("[{}] finish (error): {}", name, r.to_string());
            }
            else {
                info!("[{}] finish (success)", name);
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
