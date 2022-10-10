use std::future::Future;


pub struct TaskServer {
    rt: tokio::runtime::Runtime,
}

impl TaskServer {
    pub fn new() -> Self {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();

        TaskServer { rt }
    }

    pub fn register_oneshot_task<F>(&self, name: &str, /*TODO: delay,*/f: F)
        where F: Future + Send + 'static
    {
        let name = name.to_string();
        self.rt.spawn(async move {
            info!("[{}] start", name);
            f.await;
            info!("[{}] finish", name);
        });
    }

    pub fn register_periodic_task<F>(&self, name: &str, f: F)
        where F: FnOnce() + Send + 'static
    {

    }

    pub fn run(&self) {
        self.rt.block_on(self.main());
    }

    /// tokio 管理下のメインスレッド。
    async fn main(&self) {
        info!("TaskServer started");
    }
}
