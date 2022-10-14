use crate::sys::config;
use crate::sys::taskserver::Control;
use super::SystemModule;

pub struct Twitter {
    enabled: bool,
    fake: Option<bool>,
    consumer_key   : Option<String>,
    consumer_secret: Option<String>,
    access_token   : Option<String>,
    access_secret  : Option<String>,
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
        let fake = Some(true);

        let (fake,
            consumer_key, consumer_secret,
            access_token, access_secret)
        = if enabled {
            (
                Some(config::get_bool(&["twitter", "fake_tweet"])
                    .expect("config error: twitter.fake_tweet")),
                Some(config::get_string(&["twitter", "consumer_key"])
                    .expect("config error: twitter.consumer_key")),
                Some(config::get_string(&["twitter", "consumer_secret"])
                    .expect("config error: twitter.consumer_secret")),
                Some(config::get_string(&["twitter", "access_token"])
                    .expect("config error: twitter.access_token")),
                Some(config::get_string(&["twitter", "access_secret"])
                    .expect("config error: twitter.access_secret")),
            )
        }
        else {
            (None, None, None, None, None)
        };

        Twitter {
            enabled, fake,
            consumer_key, consumer_secret, access_token, access_secret
        }
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
