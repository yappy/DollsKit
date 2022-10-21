use crate::sys::config;
use crate::sys::taskserver::Control;
use crate::sys::net;
use super::SystemModule;
use std::collections::BTreeMap;
use std::time::{SystemTime, UNIX_EPOCH};
use rand::Rng;

pub struct Twitter {
    enabled: bool,
    fake_tweet: bool,
    consumer_key   : String,
    consumer_secret: String,
    access_token   : String,
    access_secret  : String,
}

impl Default for Twitter {
    fn default() -> Self {
        Self {
            enabled: false, fake_tweet: true,
            consumer_key: "".into(), consumer_secret: "".into(),
            access_token: "".into(), access_secret: "".into()
        }
    }
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

        if enabled {
            let (fake_tweet,
                consumer_key, consumer_secret,
                access_token, access_secret) =
            (
                config::get_bool(&["twitter", "fake_tweet"])
                    .expect("config error: twitter.fake_tweet"),
                config::get_string(&["twitter", "consumer_key"])
                    .expect("config error: twitter.consumer_key"),
                config::get_string(&["twitter", "consumer_secret"])
                    .expect("config error: twitter.consumer_secret"),
                config::get_string(&["twitter", "access_token"])
                    .expect("config error: twitter.access_token"),
                config::get_string(&["twitter", "access_secret"])
                    .expect("config error: twitter.access_secret"),
            );

            Twitter {
                enabled, fake_tweet,
                consumer_key, consumer_secret, access_token, access_secret
            }
        }
        else {
            Twitter {
                enabled, ..Default::default()
            }
        }
    }

    async fn twitter_task(&self, ctrl: &Control) -> Result<(), String> {
        info!("[twitter] normal task");
        Ok(())
    }

    async fn twitter_task_entry(ctrl: Control) -> Result<(), String> {
        ctrl.sysmods().twitter.twitter_task(&ctrl).await
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
/// 一意にならないため、Sorted Map として B-Tree を使うことにする
type KeyValue = BTreeMap<String, String>;


/// OAuth 1.0a 認証のための KeyValue セットを生成する。
///
/// oauth_signature フィールドはこれらを含むデータを元に計算する必要があるので
/// まだ設定しない。
/// 乱数による nonce やタイムスタンプが含まれるため、呼び出すたびに結果は変わる。
///
/// 詳細:
/// <https://developer.twitter.com/en/docs/authentication/oauth-1-0a/authorizing-a-request>
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

    // 署名は署名以外の oauth_* フィールドに対しても行う
    // 今はまだ不明なので後で追加する
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

/// HMAC-SHA1 署名を計算する。
/// この結果を oauth_signature フィールドに設定する必要がある。
///
/// * oauth_param: HTTP header 内の Authorization: OAuth 関連フィールド。
/// * query_param: URL 末尾の query。
/// * body_param: HTTP request body にあるパラメータ (POST data)。
///
/// 詳細:
/// <https://developer.twitter.com/en/docs/authentication/oauth-1-0a/creating-a-signature>
///
/// oauth_param, query_param, body_param 内でキーの重複があると panic する。
fn create_signature(
    http_method: &str, base_url: &str,
    oauth_param: &KeyValue, query_param: &KeyValue, body_param: &KeyValue,
    consumer_secret: &str, token_secret: &str)
    -> String
{
    // "Collecting the request method and URL"
    // Example:
    // http_method = POST
    // base_url = https://api.twitter.com/1.1/statuses/update.json

    // "Collecting parameters"
    // 以下の percent encode 前データを percent encode しながら 1つにまとめて
    // キーの辞書順にソートする
    // キーの重複は Twitter では認められていないのでシンプルに考えて OK
    // * URL 末尾の query
    // * request body
    // * HTTP header の oauth_* パラメタ
    //
    // 1. Percent encode every key and value that will be signed.
    // 2. Sort the list of parameters alphabetically [1] by encoded key [2].
    // 3. For each key/value pair:
    // 4. Append the encoded key to the output string.
    // 5. Append the ‘=’ character to the output string.
    // 6. Append the encoded value to the output string.
    // 7. If there are more key/value pairs remaining, append a ‘&’ character to the output string.

    // 1-2
    let mut param = KeyValue::new();
    let encode_add =
    |param: &mut KeyValue, src: &KeyValue| {
        for (k, v) in src.iter() {
            let old = param.insert(
                net::percent_encode(k),
                net::percent_encode(v));
            if old.is_some() {
                panic!("duplicate key: {}", k);
            }
        }
    };
    encode_add(&mut param, oauth_param);
    encode_add(&mut param, query_param);
    encode_add(&mut param, body_param);

    // 3-7
    let mut parameter_string = "".to_string();
    let mut is_first = true;
    for (k, v) in param {
        if is_first {
            is_first = false;
        }
        else {
            parameter_string.push('&');
        }
        parameter_string.push_str(&k);
        parameter_string.push('=');
        parameter_string.push_str(&v);
    }

    // "Creating the signature base string"
    // "signature base string" by OAuth spec
    // 署名対象となる文字列を生成する
    // method, url, param を & でつなげるだけ
    //
    // 1. Convert the HTTP Method to uppercase and set the output string equal to this value.
    // 2. Append the ‘&’ character to the output string.
    // 3. Percent encode the URL and append it to the output string.
    // 4. Append the ‘&’ character to the output string.
    // 5. Percent encode the parameter string and append it to the output string.
    let mut signature_base_string = "".to_string();
    signature_base_string.push_str(&http_method.to_ascii_uppercase());
    signature_base_string.push('&');
    signature_base_string.push_str(&net::percent_encode(base_url));
    signature_base_string.push('&');
    signature_base_string.push_str(&net::percent_encode(&parameter_string));

    // "Getting a signing key"
    // 署名鍵は consumer_secret と token_secret をエスケープして & でつなぐだけ
    let mut signing_key = "".to_string();
    signing_key.push_str(consumer_secret);
    signing_key.push('&');
    signing_key.push_str(token_secret);

    // "Calculating the signature"
    // HMAC SHA1
    let result = net::hmac_sha1(
        signing_key.as_bytes(),
        signature_base_string.as_bytes());

    // base64 encode したものを署名として "oauth_signature" に設定する
    base64::encode(result.into_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    // https://developer.twitter.com/en/docs/authentication/oauth-1-0a/creating-a-signature
    #[test]
    fn tweitter_sample_signature() {
        let method = "POST";
        let url = "https://api.twitter.com/1.1/statuses/update.json";

        // This is just an example in the Twitter API document
        // Not a real secret key
        let mut oauth_param = KeyValue::new();
        oauth_param.insert("oauth_consumer_key".into(), "xvz1evFS4wEEPTGEFPHBog".into());
        oauth_param.insert("oauth_nonce".into(), "kYjzVBB8Y0ZFabxSWbWovY3uYSQ2pTgmZeNu2VS4cg".into());
        oauth_param.insert("oauth_signature_method".into(), "HMAC-SHA1".into());
        oauth_param.insert("oauth_timestamp".into(), "1318622958".into());
        oauth_param.insert("oauth_token".into(), "370773112-GmHxMAgYyLbNEtIKZeRNFsMKPR9EyMZeS9weJAEb".into());
        oauth_param.insert("oauth_version".into(), "1.0".into());

        let mut query_param = KeyValue::new();
        query_param.insert("include_entities".into(), "true".into());

        let mut body_param = KeyValue::new();
        body_param.insert("status".into(), "Hello Ladies + Gentlemen, a signed OAuth request!".into());

        // This is just an example in the Twitter API document
        // Not a real secret key
        let consumer_secret = "kAcSOqF21Fu85e7zjz7ZN2U4ZRhfV3WpwPAoE3Z7kBw";
        let token_secret = "LswwdoUaIvS8ltyTt5jkRh4J50vUPVVHtR2YPi5kE";

        let result = create_signature(
            method, url,
            &oauth_param, &query_param, &body_param, consumer_secret, token_secret);

        assert_eq!(result, "hCtSmYh+iHYCEqBWrE7C7hYmtUk=");
    }
}
