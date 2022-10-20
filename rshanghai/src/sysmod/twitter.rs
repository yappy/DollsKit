use rand::Rng;

use crate::sys::config;
use crate::sys::taskserver::Control;
use super::SystemModule;
use std::collections::BTreeMap;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct Twitter {
    enabled: bool,
    fake_tweet: Option<bool>,
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

        let (fake_tweet,
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
            enabled, fake_tweet,
            consumer_key, consumer_secret, access_token, access_secret
        }
    }

    async fn twitter_task(&self, ctrl: &Control) {
        info!("[twitter] normal task");
    }

    async fn twitter_task_entry(ctrl: Control) {
        ctrl.sysmods().twitter.twitter_task(&ctrl).await;
    }

}

impl SystemModule for Twitter {
    fn on_start(&self, ctrl: &Control) {
        info!("[twitter] on_start");
        ctrl.spawn_oneshot_task("twitter", Twitter::twitter_task_entry);
    }
}

/*
// Twitter API 1.1
const  URL_ACCOUNT_VERIFY_CREDENTIALS: &str =
    "https://api.twitter.com/1.1/account/verify_credentials.json";
const URL_STATUSES_UPDATE: &str =
    "https://api.twitter.com/1.1/statuses/update.json";
const URL_STATUSES_HOME_TIMELINE: &str =
    "https://api.twitter.com/1.1/statuses/home_timeline.json";
const URL_STATUSES_USER_TIMELINE: &str =
    "https://api.twitter.com/1.1/statuses/user_timeline.json";
*/

// Twitter API v2
const URL_USERS_BY_USERNAME: &str =
    "https://api.twitter.com/2/users/by/username/";

/// HTTP header や query を表すデータ構造。
///
/// 署名時にソートを求められるのと、ハッシュテーブルだと最終的なリクエスト内での順番が
/// 一意にならないのが微妙な気がするので二分探索木を使うことにする。
type KeyValue = BTreeMap<String, String>;

/// https://developer.twitter.com/en/docs/authentication/oauth-1-0a/authorizing-a-request
fn create_oauth_field(consumer_key: &str, access_token: &str) -> KeyValue {
    let mut param = KeyValue::new();

    // oauth_consumer_key: アプリの識別子
    param.insert("oauth_consumer_key".into(), consumer_key.into());

    // oauth_nonce: ランダム値 (リプレイ攻撃対策)
    // 暗号学的安全性が必要か判断がつかないので安全な方にしておく
    // Twitter によるとランダムな英数字なら何でもいいらしいが、例に挙げられている
    // 32byte の乱数を BASE64 にして英数字のみを残したものとする
    let mut rng = rand::thread_rng();
    let rnd32: [u8; 4] = rng.gen();
    let rnd32_str = base64::encode(rnd32);
    let mut nonce_str = "".to_string();
    for c in rnd32_str.chars() {
        if c.is_alphanumeric() {
            nonce_str.push(c);
        }
    }
    param.insert("oauth_nonce".into(), nonce_str);

    // 署名は署名以外の o_auth フィールドに対しても行うので後で追加する
    // param.emplace("oauth_signature", sha1(...));

    // oauth_signature_method, oauth_timestamp, oauth_token, oauth_version
    param.insert("oauth_signature_method".to_string(), "HMAC-SHA1".into());
    let unix_epoch_sec = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    param.insert("oauth_timestamp".into(), unix_epoch_sec.to_string());
    param.insert("oauth_token".into(), access_token.into());
    param.insert("oauth_version".into(), "1.0".into());

    param
}

/// https://developer.twitter.com/en/docs/authentication/oauth-1-0a/creating-a-signature
fn create_signature(
    http_method: &str, base_url: &str,
    oauth_param: &KeyValue, query_param: &KeyValue,
    consumer_secret: &str, token_secret: &str)
{

}
