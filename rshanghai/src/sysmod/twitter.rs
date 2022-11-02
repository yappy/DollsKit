use crate::sys::config;
use crate::sys::taskserver::Control;
use crate::sys::net;
use super::SystemModule;
use chrono::NaiveTime;
use log::{info, debug};
use serde::{Serialize, Deserialize};
use std::collections::{BTreeMap, HashMap};
use std::time::{SystemTime, UNIX_EPOCH};
use rand::Rng;


// Twitter API v2
const URL_USERS_ME: &str =
    "https://api.twitter.com/2/users/me";
const URL_USERS_BY: &str =
    "https://api.twitter.com/2/users/by";
const LIMIT_USERS_BY: usize = 100;

macro_rules! URL_USERS_TIMELINES_HOME {
    () => { "https://api.twitter.com/2/users/{}/timelines/reverse_chronological" };
}
macro_rules! URL_USERS_TWEET {
    () => { "https://api.twitter.com/2/users/{}/tweets" };
}

const URL_TWEETS: &str =
    "https://api.twitter.com/2/tweets";


#[derive(Clone, Debug, Serialize, Deserialize)]
struct User {
    id: String,
    name: String,
    username: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct UsersMe {
    data: User,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct UsersBy {
    data: Vec<User>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Tweet {
    id: String,
    text: String,
    author_id: Option<String>,
    edit_history_tweet_ids: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Meta {
    /// ドキュメントには count とあるが、レスポンスの例は result_count になっている。
    result_count: u64,
    /// [result_count] = 0 だと存在しない
    newest_id: Option<String>,
    /// [result_count] = 0 だと存在しない
    oldest_id: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Timeline {
    data: Vec<Tweet>,
    meta: Meta,
}

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
struct TweetParamReply {
    in_reply_to_tweet_id: String,
}

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
struct TweetParamPoll {
    duration_minutes: u32,
    options: Vec<String>,
}

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
struct TweetParam {
    #[serde(skip_serializing_if = "Option::is_none")]
    poll: Option<TweetParamPoll>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reply: Option<TweetParamReply>,
    /// 本文。media.media_ids が無いなら必須。
    #[serde(skip_serializing_if = "Option::is_none")]
    text: Option<String>
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct TweetResponse {
    data: TweetResponseData,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct TweetResponseData {
    id: String,
    text: String,
}


#[derive(Clone, Serialize, Deserialize)]
struct TwitterConfig {
    enabled: bool,
    debug_exec_once: bool,
    fake_tweet: bool,
    consumer_key   : String,
    consumer_secret: String,
    access_token   : String,
    access_secret  : String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TimelineCheck {
    user_names: Vec<String>,
    pattern: Vec<(Vec<String>, Vec<String>)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TwitterContents {
    timeline_check: Vec<TimelineCheck>,
}

pub struct Twitter {
    config: TwitterConfig,
    contents: TwitterContents,
    wakeup_list: Vec<NaiveTime>,
    /// タイムラインチェックの際の走査開始 tweet id。
    ///
    /// 初期状態は None で、未取得状態を表す。
    /// 最初の設定は、自身の最新ツイートを取得して設定する。
    /// ツイートを行うと最新ツイートが変わってしまうため、
    /// ツイート時、この値が None ならばツイート前に設定を行う。
    ///
    /// ツイート成功後、その ID で更新する。
    tl_check_since_id: Option<String>,
    /// 自身の User オブジェクト。最初の1回だけ取得を行う。
    my_user_cache: Option<User>,
    /// screen name -> User オブジェクトのマップ。
    user_id_cache: HashMap<String, User>,
}

impl Twitter {
    pub fn new(wakeup_list: Vec<NaiveTime>) -> Self {
        info!("[twitter] initialize");

        let jsobj = config::get_object(&["twitter"]).expect("config error: twitter");
        let config: TwitterConfig = serde_json::from_value(jsobj).unwrap();
        let jsobj = config::get_object(&["tw_contents"]).expect("config error: tw_contents");
        let contents: TwitterContents = serde_json::from_value(jsobj).unwrap();

        Twitter {
            config,
            contents,
            wakeup_list,
            tl_check_since_id: None,
            my_user_cache: None,
            user_id_cache: Default::default(),
        }
    }

    /// Twitter 巡回タスク。
    async fn twitter_task(&mut self, ctrl: &Control) -> Result<(), String> {
        info!("[tw_check] periodic check task");

        // 自分の ID
        let me = self.get_my_id().await?;
        info!("[tw_check] user_me: {:?}", me);

        // チェック開始 ID
        let since_id = self.get_since_id().await?;
        info!("[tw_check] since_id: {}", since_id);

        // 設定ファイル中の全 user name (screen name) から ID を得る
        info!("[tw_check] get all user info from screen name");
        // borrow checker (E0502) が手強すぎて勝てないので諦めてコピーを取る
        let tlc_list = self.contents.timeline_check.clone();
        for tlcheck in tlc_list.iter() {
            self.resolve_ids(&tlcheck.user_names).await?;
        }
        info!("[tw_check] user id cache size: {}", self.user_id_cache.len());

        // 以降メイン処理

        // 自分の最終ツイート以降のタイムラインを得る
        let tl = self.users_timelines_home(&me.id, &since_id).await?;
        info!("{} tweets fetched", tl.data.len());

        // 反応設定のブロックごとに全ツイートを走査する
        let mut reply_buf = vec![];
        for ch in self.contents.timeline_check.iter() {
            // 自分のツイートには反応しない
            let tliter = tl.data.iter()
            .filter(|tw| {
                tw.id != me.id
            });
            for tw in tliter {
                // author_id が user_names リストに含まれているものでフィルタ
                let user_match = ch.user_names.iter().any(|user_name| {
                    let user = self.resolve_id_from_cache(user_name);
                    match user {
                        Some(user) => *tw.author_id.as_ref().unwrap() == user.id,
                        // id 取得に失敗しているので無視
                        None => false,
                    }
                });
                if !user_match {
                    continue;
                }
                // pattern 判定
                for (pats, msgs) in ch.pattern.iter() {
                    // 配列内のすべてのパターンを満たす
                    let match_hit = pats.iter().all(|pat| {
                        Self::pattern_match(pat, &tw.text)
                    });
                    if match_hit {
                        info!("FIND: {:?}", tw);
                        // 配列からリプライをランダムに1つ選ぶ
                        let rnd_idx = rand::thread_rng().gen_range(0..msgs.len());
                        // リプライツイート (id, text) を一旦バッファする
                        // E0502 回避
                        reply_buf.push((
                            tw.author_id.clone(),
                            msgs[rnd_idx].clone(),
                        ));
                        // 複数種類では反応しない
                        // 反応は1回のみ
                        break;
                    }
                }
            }
        }

        for (reply_to, text) in reply_buf {
            let param = TweetParam {
                text: Some(text.clone()),
                ..Default::default()
            };
            self.tweet(param).await?;
        }

        //test
        /*
        let param = TweetParam {
            poll: Some(TweetParamPoll {
                duration_minutes: 60 * 24,
                options: vec!["ホワイト".into(), "ブラック".into()],
            }),
            text: Some("?".into()),
            ..Default::default()
        };
        let resp = self.tweets_post(param).await?;
        info!("tweet result: {:?}", resp);
        */

        Ok(())
    }

    /// text から pat を検索する。
    /// 先頭が '^' だとそれで始まる場合のみ。
    /// 末尾が '$' だとそれで終わる場合のみ。
    fn pattern_match(pat: &str, text: &str) -> bool {
        let count = pat.chars().count();
        if count == 0 {
            return false;
        }
        let match_start = pat.starts_with('^');
        let match_end = pat.ends_with('$');
        let begin = pat.char_indices()
            .nth(if match_start {1} else {0})
            .unwrap_or((0, '\0'))
            .0;
        let end = pat.char_indices()
            .nth(if match_end {count - 1} else {count})
            .unwrap_or((pat.len(), '\0'))
            .0;
        let pat = &pat[begin..end];
        if pat.is_empty() {
            return false;
        }

        if match_start && match_end {
            text == pat
        }
        else if match_start {
            text.starts_with(pat)
        }
        else if match_end {
            text.ends_with(pat)
        }
        else {
            text.contains(pat)
        }
    }

    /// 自分のツイートリストを得て最終ツイート ID を得る(キャッシュ付き)。
    async fn get_since_id(&mut self) -> Result<String, String> {
        let me = self.get_my_id().await?;
        if self.tl_check_since_id == None {
            let usertw = self.users_tweets(&me.id).await?;
            // API は成功したが最新 ID が得られなかった場合は "1" を設定する
            self.tl_check_since_id = Some(
                usertw.meta.newest_id.unwrap_or_else(|| "1".into()));
        }

        Ok(self.tl_check_since_id.clone().unwrap())
    }

    /// [TwitterConfig::fake_tweet] 設定に対応したツイート。
    async fn tweet(&mut self, param: TweetParam) -> Result<(), String> {
        // tl_check_since_id が None なら自分の最新ツイート ID を取得して設定する
        self.get_since_id().await?;

        if !self.config.fake_tweet {
            // real tweet!
            self.tweets_post(param).await?;
            Ok(())
        }
        else {
            info!("Fake tweet: {:?}", param);
            Ok(())
        }
    }

    async fn twitter_task_entry(ctrl: Control) -> Result<(), String> {
        let mut twitter = ctrl.sysmods().twitter.write().await;
        twitter.twitter_task(&ctrl).await
    }

    /// 自身の Twitter ID を返す。
    /// [Self::users_me] の キャッシュ付きバージョン。
    async fn get_my_id(&mut self) -> Result<User, String> {
        if let Some(user) = &self.my_user_cache {
            Ok(user.clone())
        }
        else {
            Ok(self.users_me().await?.data)
        }
    }

    fn resolve_id_from_cache(&self, user: &String) -> Option<&User> {
        self.user_id_cache.get(user)
    }

    /// user name (screen name) から id を取得する。
    ///
    /// 結果は [Self::user_id_cache] に入れる。
    /// 凍結等で取得できない可能性があり、その場合は無視される。
    async fn resolve_ids(&mut self, user_names: &[String]) -> Result<(), String> {
        // user_id_cache にないユーザ名を集める
        let unknown_users: Vec<_> = user_names.iter()
            .filter_map(|user| {
                if !self.user_id_cache.contains_key(user) {
                    Some(user.clone())
                }
                else {
                    None
                }
            })
            .collect();

        // LIMIT_USERS_BY 個ずつ GET リクエストしてハッシュテーブルにキャッシュする
        let mut start = 0_usize;
        while start < unknown_users.len() {
            let end = std::cmp::min(unknown_users.len(), start + LIMIT_USERS_BY);
            let result = self.users_by(&unknown_users[start..end]).await?;
            for user in result.data.iter() {
                info!("[twitter] resolve username: {} => {}", user.username, user.id);
                self.user_id_cache.insert(user.username.clone(), user.clone());
            }

            start += LIMIT_USERS_BY;
        }

        Ok(())
    }

    async fn users_me(&self) -> Result<UsersMe, String> {
        let resp = self.http_oauth_get(
            URL_USERS_ME,
            &KeyValue::new()).await;
        let json_str = process_response(resp).await?;
        let obj: UsersMe = convert_from_json(&json_str)?;

        Ok(obj)
    }

    async fn users_by(&self, users: &[String]) -> Result<UsersBy, String> {
        if !(1..LIMIT_USERS_BY).contains(&users.len()) {
            panic!("{} limit over: {}", URL_USERS_BY, users.len());
        }
        let users_str = users.join(",");
        let resp = self.http_oauth_get(
            URL_USERS_BY,
            &BTreeMap::from([("usernames".into(), users_str)])).await;
        let json_str = process_response(resp).await?;
        let obj: UsersBy = convert_from_json(&json_str)?;

        Ok(obj)
    }

    async fn users_timelines_home(&self, id: &str, since_id: &str)
        -> Result<Timeline, String>
    {
        let url = format!(URL_USERS_TIMELINES_HOME!(), id);
        let param = KeyValue::from([
            ("since_id".into(), since_id.into()),
            ("expansions".into(), "author_id".into()),
        ]);
        let resp = self.http_oauth_get(
            &url,
            &param).await;
        let json_str = process_response(resp).await?;
        let obj: Timeline = convert_from_json(&json_str)?;

        Ok(obj)
    }

    async fn users_tweets(&self, id: &str) -> Result<Timeline, String> {
        let url = format!(URL_USERS_TWEET!(), id);
        let param = KeyValue::from([
            // retweets and/or replies
            ("exclude".into(), "retweets".into()),
            // default=10, min=5, max=100
            ("max_results".into(), "100".into()),
        ]);
        let resp = self.http_oauth_get(
            &url,
            &param).await;
        let json_str = process_response(resp).await?;
        let obj: Timeline = convert_from_json(&json_str)?;

        Ok(obj)
    }

    async fn tweets_post(&self, param: TweetParam) -> Result<TweetResponse, String> {
        let resp = self.http_oauth_post(
            URL_TWEETS,
            &KeyValue::new(),
            param).await;
        let json_str = process_response(resp).await?;
        let obj: TweetResponse = convert_from_json(&json_str)?;

        Ok(obj)
    }

    async fn http_oauth_get(&self, base_url: &str, query_param: &KeyValue)
        -> Result<reqwest::Response, reqwest::Error>
    {
        let cf = &self.config;
        let mut oauth_param = create_oauth_field(
            &cf.consumer_key, &cf.access_token);
        let signature = create_signature(
            "GET", base_url,
            &oauth_param, query_param, &KeyValue::new(),
            &cf.consumer_secret, &cf.access_secret);
        oauth_param.insert("oauth_signature".into(), signature);

        let (oauth_k,oauth_v) = create_http_oauth_header(&oauth_param);

        let client = reqwest::Client::new();
        let req = client
            .get(base_url)
            .query(&query_param)
            .header(oauth_k, oauth_v);
        let res = req.send().await?;

        Ok(res)
    }

    async fn http_oauth_post<T>(
        &self,
        base_url: &str,
        query_param: &KeyValue,
        body_param: T)
        -> Result<reqwest::Response, reqwest::Error>
        where T: Serialize
    {
        let json_str = serde_json::to_string(&body_param).unwrap();

        let cf = &self.config;
        let mut oauth_param = create_oauth_field(
            &cf.consumer_key, &cf.access_token);
        let signature = create_signature(
            "POST", base_url,
            &oauth_param, query_param, &KeyValue::new(),
            &cf.consumer_secret, &cf.access_secret);
        oauth_param.insert("oauth_signature".into(), signature);

        let (oauth_k,oauth_v) = create_http_oauth_header(&oauth_param);

        debug!("POST: {}", json_str);
        let client = reqwest::Client::new();
        let req = client
            .post(base_url)
            .query(query_param)
            .header(oauth_k, oauth_v)
            .header("Content-type", "application/json")
            .body(json_str);
        let res = req.send().await?;

        Ok(res)
    }

}

impl SystemModule for Twitter {
    fn on_start(&self, ctrl: &Control) {
        info!("[twitter] on_start");
        if self.config.enabled {
            if self.config.debug_exec_once {
                ctrl.spawn_oneshot_task(
                    "tw_check",
                    Twitter::twitter_task_entry);
            }
            else {
                ctrl.spawn_periodic_task(
                    "tw_check",
                    &self.wakeup_list,
                    Twitter::twitter_task_entry);
            }
        }
    }
}

fn convert_from_json<'a, T>(json_str: &'a str) -> Result<T, String>
    where T: Deserialize<'a>
{
    match serde_json::from_str::<T>(json_str) {
        Ok(result) => Ok(result),
        Err(e) => {
            Err(format!("{}: {}", e, json_str))
        },
    }
}

async fn process_response(result: Result<reqwest::Response, reqwest::Error>)
-> Result<String, String>
{
    match result {
        Ok(resp) => {
            let status = resp.status();
            let text = resp.text().await;
            let text = match text {
                Ok(text) => text,
                Err(e) => return Err(e.to_string())
            };
            if status.is_success() {
                Ok(text)
            }
            else {
                Err(format!("HTTP error {} {}", status, text))
            }
        },
        Err(e) => Err(e.to_string()),
    }
}

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
    let rnd32: [u8; 32] = rng.gen();
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

/// HTTP header に設定する (key, value) を文字列として生成して返す。
///
/// Authorization: OAuth key1="value1", key2="value2", ..., keyN="valueN"
fn create_http_oauth_header(oauth_param: &KeyValue) -> (String, String) {
    let mut oauth_value = "OAuth ".to_string();
    {
        let v: Vec<_> = oauth_param.iter()
            .map(|(k, v)|
                format!(r#"{}="{}""#, net::percent_encode(k), net::percent_encode(v)))
            .collect();
        oauth_value.push_str(&v.join(", "));
    }

    ("Authorization".into(), oauth_value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tweet_pattern_match() {
        assert!(Twitter::pattern_match("あいうえお", "あいうえお"));
        assert!(Twitter::pattern_match("^あいうえお", "あいうえお"));
        assert!(Twitter::pattern_match("あいうえお$", "あいうえお"));
        assert!(Twitter::pattern_match("^あいうえお$", "あいうえお"));

        assert!(Twitter::pattern_match("あいう", "あいうえお"));
        assert!(Twitter::pattern_match("^あいう", "あいうえお"));
        assert!(!Twitter::pattern_match("あいう$", "あいうえお"));
        assert!(!Twitter::pattern_match("^あいう$", "あいうえお"));

        assert!(Twitter::pattern_match("うえお", "あいうえお"));
        assert!(!Twitter::pattern_match("^うえお", "あいうえお"));
        assert!(Twitter::pattern_match("うえお$", "あいうえお"));
        assert!(!Twitter::pattern_match("^うえお$", "あいうえお"));

        assert!(Twitter::pattern_match("いうえ", "あいうえお"));
        assert!(!Twitter::pattern_match("^いうえ", "あいうえお"));
        assert!(!Twitter::pattern_match("いうえ$", "あいうえお"));
        assert!(!Twitter::pattern_match("^いうえ$", "あいうえお"));

        assert!(!Twitter::pattern_match("", "あいうえお"));
        assert!(!Twitter::pattern_match("^", "あいうえお"));
        assert!(!Twitter::pattern_match("$", "あいうえお"));
        assert!(!Twitter::pattern_match("^$", "あいうえお"));
    }

    // https://developer.twitter.com/en/docs/authentication/oauth-1-0a/creating-a-signature
    #[test]
    fn twitter_sample_signature() {
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
