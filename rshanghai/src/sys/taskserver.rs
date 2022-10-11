//! 非同期タスクを管理する。

use std::{future::Future};
use std::sync::Arc;
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender, UnboundedReceiver};

type ShutdownTx = UnboundedSender<()>;
type ShutdownRx = UnboundedReceiver<()>;

/// [Control] の [Arc] 内データ。
struct InternalControl {
    rt: tokio::runtime::Runtime,
    shutdown_tx: ShutdownTx,
    shutdown_rx: ShutdownRx,
}

/// [Arc] により [TaskServer] と全タスク間で共有されるコントロールハンドル。
///
/// [Clone] 可能で、複製すると [Arc] がインクリメントされる。
#[derive(Clone)]
pub struct Control {
    internal: Arc<InternalControl>,
}

/// タスクサーバ本体。
pub struct TaskServer {
    ctrl: Control,
}

impl Control {
    pub fn spawn_oneshot_task<F, T>(&self, name: &str, f: F)
    where
        F: FnOnce(Control) -> T,
        T: Future + Send + 'static
    {
        let name = name.to_string();
        let ctrl = self.clone();
        let future = f(ctrl);
        self.internal.rt.spawn(async move {
            info!("[{}] start", name);
            future.await;
            info!("[{}] finish", name);
        });
    }
}

impl TaskServer {
    /// タスクサーバを初期化して開始する。
    pub fn new() -> Self {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();
        let (shutdown_tx, shutdown_rx) = unbounded_channel();

        let internal = InternalControl { rt, shutdown_tx, shutdown_rx };
        TaskServer { ctrl: Control { internal: Arc::new(internal) } }
    }

    pub fn spawn_oneshot_task<F, Fut>(&self, name: &str, f: F)
    where
        F: FnOnce(Control) -> Fut,
        Fut: Future + Send + 'static
    {
        self.ctrl.spawn_oneshot_task(name, f);
    }

    pub fn wait_for_shutdown(&self)
    {
        self.ctrl.internal.rt.block_on(async {
            loop {
                // TODO: wait for shutdown
            }
        });
    }

}
