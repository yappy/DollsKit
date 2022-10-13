use crate::sys::config;
use crate::sys::taskserver::Control;
use super::SystemModule;

pub struct Twitter {
    enabled: bool,
}

impl Twitter {
    pub fn new() -> Self {
        info!("[twitter] initialize");
        let enabled =
            config::get_bool(&["twitter", "enabled"])
            .expect("config error: twitter.enabled");
        if enabled {
            info!("[twitter] enabled");
        }
        else {
            info!("[twitter] disabled");
        }

        Twitter { enabled }
    }
}

impl SystemModule for Twitter {
    fn on_start(&self, ctrl: &Control) {
        info!("[twitter] on_start");
        ctrl.spawn_oneshot_task("twitter", twitter_task);
    }
}

async fn twitter_task(_ctrl: Control) {
    info!("[twitter] normal task");
}
