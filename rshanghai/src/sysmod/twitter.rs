use super::SystemModule;
use crate::sys::config;
use crate::sys::net;
use crate::sys::taskserver::Control;

use anyhow::{anyhow, bail, Context, Result};
use chrono::NaiveTime;
use log::warn;
use log::{debug, info};
use rand::Rng;
use reqwest::multipart;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::time::{SystemTime, UNIX_EPOCH};

// Twitter API v2
pub const TWEET_LEN_MAX: usize = 140;
pub const LIMIT_PHOTO_COUNT: usize = 4;
pub const LIMIT_PHOTO_SIZE: usize = 5_000_000;

const URL_USERS_ME: &str = "https://api.twitter.com/2/users/me";
const URL_USERS_BY: &str = "https://api.twitter.com/2/users/by";
const LIMIT_USERS_BY: usize = 100;

macro_rules! URL_USERS_TIMELINES_HOME {
    () => {
        "https://api.twitter.com/2/users/{}/timelines/reverse_chronological"
    };
}
macro_rules! URL_USERS_TWEET {
    () => {
        "https://api.twitter.com/2/users/{}/tweets"
    };
}

const URL_TWEETS: &str = "https://api.twitter.com/2/tweets";

const URL_UPLOAD: &str = "https://upload.twitter.com/1.1/media/upload.json";

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
    /// [Self::result_count] = 0 だと存在しない
    newest_id: Option<String>,
    /// [Self::result_count] = 0 だと存在しない
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
struct Media {
    #[serde(skip_serializing_if = "Option::is_none")]
    media_ids: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tagged_user_ids: Option<Vec<String>>,
}

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
struct TweetParam {
    #[serde(skip_serializing_if = "Option::is_none")]
    poll: Option<TweetParamPoll>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reply: Option<TweetParamReply>,
    /// 本文。media.media_ids が無いなら必須。
    #[serde(skip_serializing_if = "Option::is_none")]
    text: Option<String>,
    /// 添付メディアデータ。
    #[serde(skip_serializing_if = "Option::is_none")]
    media: Option<Media>,
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

#[derive(Clone, Debug, Serialize, Deserialize)]
struct UploadResponseData {
    media_id: u64,
    size: u64,
    expires_after_secs: u64,
}

#[derive(Clone, Serialize, Deserialize)]
struct TwitterConfig {
    tlcheck_enabled: bool,
    debug_exec_once: bool,
    fake_tweet: bool,
    consumer_key: String,
    consumer_secret: String,
    access_token: String,
    access_secret: String,
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
    username_user_cache: HashMap<String, User>,
    /// ID -> screen name のマップ。
    id_username_cache: HashMap<String, String>,
}

impl Twitter {
    pub fn new(wakeup_list: Vec<NaiveTime>) -> Result<Self> {
        info!("[twitter] initialize");

        let jsobj = config::get_object(&["twitter"])
            .map_or(Err(anyhow!("Config not found: twitter")), Ok)?;
        let config: TwitterConfig = serde_json::from_value(jsobj)?;

        let jsobj = config::get_object(&["tw_contents"])
            .map_or(Err(anyhow!("Config not found: tw_contents")), Ok)?;
        let contents: TwitterContents = serde_json::from_value(jsobj)?;

        Ok(Twitter {
            config,
            contents,
            wakeup_list,
            tl_check_since_id: None,
            my_user_cache: None,
            username_user_cache: HashMap::new(),
            id_username_cache: HashMap::new(),
        })
    }

    /// Twitter 巡回タスク。
    async fn twitter_task(&mut self, ctrl: &Control) -> Result<()> {
        // 自分の ID
        let me = self.get_my_id().await?;
        info!("[tw-check] user_me: {:?}", me);

        // チェック開始 ID
        let since_id = self.get_since_id().await?;
        info!("[tw-check] since_id: {}", since_id);

        // 設定ファイル中の全 user name (screen name) から ID を得る
        info!("[tw-check] get all user info from screen name");
        // borrow checker (E0502) が手強すぎて勝てないので諦めてコピーを取る
        let tlc_list = self.contents.timeline_check.clone();
        for tlcheck in tlc_list.iter() {
            self.resolve_ids(&tlcheck.user_names).await?;
        }
        info!(
            "[tw-check] user id cache size: {}",
            self.username_user_cache.len()
        );

        // 以降メイン処理

        // 自分の最終ツイート以降のタイムラインを得る (リツイートは除く)
        let tl = self.users_timelines_home(&me.id, &since_id).await?;
        info!("{} tweets fetched", tl.data.len());

        // 反応設定のブロックごとに全ツイートを走査する
        let mut reply_buf = vec![];
        for ch in self.contents.timeline_check.iter() {
            // 自分のツイートには反応しない
            let tliter = tl.data.iter().filter(|tw| tw.id != me.id);

            for tw in tliter {
                // author_id が user_names リストに含まれているものでフィルタ
                let user_match = ch.user_names.iter().any(|user_name| {
                    let user = self.get_user_from_username(user_name);
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
                    let match_hit = pats.iter().all(|pat| Self::pattern_match(pat, &tw.text));
                    if match_hit {
                        info!("FIND: {:?}", tw);
                        // 配列からリプライをランダムに1つ選ぶ
                        let rnd_idx = rand::thread_rng().gen_range(0..msgs.len());
                        // リプライツイート (id, text) を一旦バッファする
                        // E0502 回避
                        reply_buf.push((
                            tw.id.clone(),
                            tw.author_id.as_ref().unwrap().clone(),
                            msgs[rnd_idx].clone(),
                        ));
                        // 複数種類では反応しない
                        // 反応は1回のみ
                        break;
                    }
                }
            }
        }

        // バッファしたリプライを実行
        for (tw_id, user_id, text) in reply_buf {
            // since_id 更新用データ
            // tweet id を数値比較のため文字列から変換する
            // (リプライ先 ID + 1) の max をとる
            let cur: u64 = self.tl_check_since_id.as_ref().unwrap().parse().unwrap();
            let next: u64 = tw_id.parse().unwrap();
            let max = cur.max(next);

            let name = self.get_username_from_id(&user_id).unwrap();
            info!("reply to: {}", name);

            let param = TweetParam {
                reply: Some(TweetParamReply {
                    in_reply_to_tweet_id: tw_id,
                }),
                text: Some(text),
                ..Default::default()
            };
            self.tweet_raw(param).await?;

            // 成功したら since_id を更新する
            self.tl_check_since_id = Some(max.to_string());
        }

        // TODO: vote test
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
    #[allow(clippy::bool_to_int_with_if)]
    fn pattern_match(pat: &str, text: &str) -> bool {
        let count = pat.chars().count();
        if count == 0 {
            return false;
        }
        let match_start = pat.starts_with('^');
        let match_end = pat.ends_with('$');
        let begin = pat
            .char_indices()
            // clippy::bool_to_int_with_if
            .nth(if match_start { 1 } else { 0 })
            .unwrap_or((0, '\0'))
            .0;
        let end = pat
            .char_indices()
            .nth(if match_end { count - 1 } else { count })
            .unwrap_or((pat.len(), '\0'))
            .0;
        let pat = &pat[begin..end];
        if pat.is_empty() {
            return false;
        }

        if match_start && match_end {
            text == pat
        } else if match_start {
            text.starts_with(pat)
        } else if match_end {
            text.ends_with(pat)
        } else {
            text.contains(pat)
        }
    }

    /// 自分のツイートリストを得て最終ツイート ID を得る(キャッシュ付き)。
    async fn get_since_id(&mut self) -> Result<String> {
        let me = self.get_my_id().await?;
        if self.tl_check_since_id.is_none() {
            let usertw = self.users_tweets(&me.id).await?;
            // API は成功したが最新 ID が得られなかった場合は "1" を設定する
            self.tl_check_since_id = Some(usertw.meta.newest_id.unwrap_or_else(|| "1".into()));
        }

        Ok(self.tl_check_since_id.clone().unwrap())
    }

    /// シンプルなツイート。
    /// 中身は [Self::tweet_raw]。
    pub async fn tweet(&mut self, text: &str) -> Result<()> {
        self.tweet_with_media(text, &[]).await
    }

    /// メディア付きツイート。
    /// 中身は [Self::tweet_raw]。
    pub async fn tweet_with_media(&mut self, text: &str, media_ids: &[u64]) -> Result<()> {
        let media_ids = if media_ids.is_empty() {
            None
        } else {
            let media_ids: Vec<_> = media_ids.iter().map(|id| id.to_string()).collect();
            Some(media_ids)
        };
        let media = media_ids.map(|media_ids| Media {
            media_ids: Some(media_ids),
            ..Default::default()
        });

        let param = TweetParam {
            text: Some(text.to_string()),
            media,
            ..Default::default()
        };

        self.tweet_raw(param).await
    }

    /// [TwitterConfig::fake_tweet] 設定に対応したツイート。
    async fn tweet_raw(&mut self, mut param: TweetParam) -> Result<()> {
        // tl_check_since_id が None なら自分の最新ツイート ID を取得して設定する
        self.get_since_id().await?;

        // 140 字チェック
        if let Some(ref text) = param.text {
            let len = text.chars().count();
            if len > TWEET_LEN_MAX {
                warn!("tweet length > {}: {}", TWEET_LEN_MAX, len);
                warn!("before: {}", text);
                let text = Self::truncate_tweet_text(text).to_string();
                warn!("after : {}", text);
                param.text = Some(text);
            }
        }

        if !self.config.fake_tweet {
            // real tweet!
            self.tweets_post(param).await?;

            Ok(())
        } else {
            info!("fake tweet: {:?}", param);

            Ok(())
        }
    }

    /// 140 字に切り詰める
    fn truncate_tweet_text(text: &str) -> &str {
        // 141 文字目の最初のバイトインデックスを得る
        let lastc = text.char_indices().nth(TWEET_LEN_MAX);

        match lastc {
            // 0 からそれを含まないバイトまで
            Some((ind, _)) => &text[0..ind],
            // 存在しないなら文字列全体を返す
            None => text,
        }
    }

    /// <https://developer.twitter.com/en/docs/twitter-api/v1/media/upload-media/api-reference/post-media-upload>
    /// <https://developer.twitter.com/en/docs/twitter-api/v1/media/upload-media/uploading-media/media-best-practices>
    pub async fn media_upload<T: Into<reqwest::Body>>(&self, bin: T) -> Result<u64> {
        if self.config.fake_tweet {
            info!("fake upload");

            return Ok(0);
        }

        info!("upload");
        let part = multipart::Part::stream(bin);
        let form = multipart::Form::new().part("media", part);

        let resp = self
            .http_oauth_post_multipart(URL_UPLOAD, &BTreeMap::new(), form)
            .await?;
        let json_str = process_response(resp).await?;
        let obj: UploadResponseData = convert_from_json(&json_str)?;
        info!("upload OK: media_id={}", obj.media_id);

        Ok(obj.media_id)
    }

    /// エントリ関数。[Self::twitter_task] を呼ぶ。
    ///
    /// [Control] 内の [Twitter] オブジェクトを lock するので
    /// [Self::twitter_task] は排他実行となる。
    async fn twitter_task_entry(ctrl: Control) -> Result<()> {
        let mut twitter = ctrl.sysmods().twitter.lock().await;
        twitter.twitter_task(&ctrl).await
    }

    /// 自身の Twitter ID を返す。
    /// [Self::users_me] の キャッシュ付きバージョン。
    async fn get_my_id(&mut self) -> Result<User> {
        if let Some(user) = &self.my_user_cache {
            Ok(user.clone())
        } else {
            Ok(self.users_me().await?.data)
        }
    }

    fn get_user_from_username(&self, name: &String) -> Option<&User> {
        self.username_user_cache.get(name)
    }

    fn get_username_from_id(&self, id: &String) -> Option<&String> {
        self.id_username_cache.get(id)
    }

    /// user name (screen name) から id を取得する。
    /// id -> user name のマップも同時に作成する。
    ///
    /// 結果は [Self::username_user_cache], [Self::id_username_cache] に入れる。
    /// 凍結等で取得できない可能性があり、その場合はエラーを出しながら続行するよりは
    /// panic でユーザに知らせる。
    async fn resolve_ids(&mut self, user_names: &[String]) -> Result<()> {
        // name_user_cache にないユーザ名を集める
        let unknown_users: Vec<_> = user_names
            .iter()
            .filter_map(|user| {
                if !self.username_user_cache.contains_key(user) {
                    Some(user.clone())
                } else {
                    None
                }
            })
            .collect();

        // LIMIT_USERS_BY 個ずつ GET リクエストしてハッシュテーブルにキャッシュする
        let mut start = 0_usize;
        while start < unknown_users.len() {
            let end = std::cmp::min(unknown_users.len(), start + LIMIT_USERS_BY);
            let request_users = &unknown_users[start..end];
            let mut rest: BTreeSet<_> = request_users.iter().collect();

            // suspend user のみでリクエストすると
            // {data: {...}} でなく {error: {...}}
            // が返ってきて API は 200 で成功するがパースに失敗する
            // やや汚いが panic してユーザリストの見直すよう促す
            let result = self.users_by(request_users).await;
            if let Err(e) = result {
                if e.is::<serde_json::Error>() {
                    panic!("parse error {:?}", e);
                } else {
                    return Err(e);
                }
            }

            for user in result?.data.iter() {
                info!(
                    "[twitter] resolve username: {} => {}",
                    user.username, user.id
                );
                self.username_user_cache
                    .insert(user.username.clone(), user.clone());
                self.id_username_cache
                    .insert(user.id.clone(), user.username.clone());
                let removed = rest.remove(&user.username);
                assert!(removed);
            }
            assert!(
                rest.is_empty(),
                "cannot resolved (account suspended?): {:?}",
                rest
            );

            start += LIMIT_USERS_BY;
        }
        assert_eq!(self.username_user_cache.len(), self.id_username_cache.len());

        Ok(())
    }

    async fn users_me(&self) -> Result<UsersMe> {
        let resp = self.http_oauth_get(URL_USERS_ME, &KeyValue::new()).await?;
        let json_str = process_response(resp).await?;
        let obj: UsersMe = convert_from_json(&json_str)?;

        Ok(obj)
    }

    async fn users_by(&self, users: &[String]) -> Result<UsersBy> {
        if !(1..LIMIT_USERS_BY).contains(&users.len()) {
            panic!("{} limit over: {}", URL_USERS_BY, users.len());
        }
        let users_str = users.join(",");
        let resp = self
            .http_oauth_get(
                URL_USERS_BY,
                &BTreeMap::from([("usernames".into(), users_str)]),
            )
            .await?;
        let json_str = process_response(resp).await?;
        let obj: UsersBy = convert_from_json(&json_str)?;

        Ok(obj)
    }

    async fn users_timelines_home(&self, id: &str, since_id: &str) -> Result<Timeline> {
        let url = format!(URL_USERS_TIMELINES_HOME!(), id);
        let param = KeyValue::from([
            ("since_id".into(), since_id.into()),
            ("exclude".into(), "retweets".into()),
            ("expansions".into(), "author_id".into()),
        ]);
        let resp = self.http_oauth_get(&url, &param).await?;
        let json_str = process_response(resp).await?;
        let obj: Timeline = convert_from_json(&json_str)?;

        Ok(obj)
    }

    async fn users_tweets(&self, id: &str) -> Result<Timeline> {
        let url = format!(URL_USERS_TWEET!(), id);
        let param = KeyValue::from([
            // retweets and/or replies
            ("exclude".into(), "retweets".into()),
            // default=10, min=5, max=100
            ("max_results".into(), "100".into()),
        ]);
        let resp = self.http_oauth_get(&url, &param).await?;
        let json_str = process_response(resp).await?;
        let obj: Timeline = convert_from_json(&json_str)?;

        Ok(obj)
    }

    async fn tweets_post(&self, param: TweetParam) -> Result<TweetResponse> {
        let resp = self
            .http_oauth_post_json(URL_TWEETS, &KeyValue::new(), &param)
            .await?;
        let json_str = process_response(resp).await?;
        let obj: TweetResponse = convert_from_json(&json_str)?;

        Ok(obj)
    }

    async fn http_oauth_get(
        &self,
        base_url: &str,
        query_param: &KeyValue,
    ) -> Result<reqwest::Response> {
        let cf = &self.config;
        let mut oauth_param = create_oauth_field(&cf.consumer_key, &cf.access_token);
        let signature = create_signature(
            "GET",
            base_url,
            &oauth_param,
            query_param,
            &KeyValue::new(),
            &cf.consumer_secret,
            &cf.access_secret,
        );
        oauth_param.insert("oauth_signature".into(), signature);

        let (oauth_k, oauth_v) = create_http_oauth_header(&oauth_param);

        let client = reqwest::Client::new();
        let req = client
            .get(base_url)
            .query(&query_param)
            .header(oauth_k, oauth_v);
        let res = req.send().await?;

        Ok(res)
    }

    async fn http_oauth_post_json<T: Serialize>(
        &self,
        base_url: &str,
        query_param: &KeyValue,
        body_param: &T,
    ) -> Result<reqwest::Response> {
        let json_str = serde_json::to_string(body_param).unwrap();
        debug!("POST: {}", json_str);

        let client = reqwest::Client::new();
        let req = self
            .http_oauth_post(&client, base_url, query_param)
            .header("Content-type", "application/json")
            .body(json_str);
        let resp = req.send().await?;

        Ok(resp)
    }

    async fn http_oauth_post_multipart(
        &self,
        base_url: &str,
        query_param: &KeyValue,
        body: multipart::Form,
    ) -> Result<reqwest::Response> {
        let client = reqwest::Client::new();
        let req = self
            .http_oauth_post(&client, base_url, query_param)
            .multipart(body);
        let resp = req.send().await?;

        Ok(resp)
    }

    fn http_oauth_post(
        &self,
        client: &reqwest::Client,
        base_url: &str,
        query_param: &KeyValue,
    ) -> reqwest::RequestBuilder {
        let cf = &self.config;
        let mut oauth_param = create_oauth_field(&cf.consumer_key, &cf.access_token);
        let signature = create_signature(
            "POST",
            base_url,
            &oauth_param,
            query_param,
            &KeyValue::new(),
            &cf.consumer_secret,
            &cf.access_secret,
        );
        oauth_param.insert("oauth_signature".into(), signature);

        let (oauth_k, oauth_v) = create_http_oauth_header(&oauth_param);

        client
            .post(base_url)
            .query(query_param)
            .header(oauth_k, oauth_v)
    }
}

impl SystemModule for Twitter {
    fn on_start(&self, ctrl: &Control) {
        info!("[twitter] on_start");
        if self.config.tlcheck_enabled {
            if self.config.debug_exec_once {
                ctrl.spawn_oneshot_task("tw-check", Twitter::twitter_task_entry);
            } else {
                ctrl.spawn_periodic_task(
                    "tw-check",
                    &self.wakeup_list,
                    Twitter::twitter_task_entry,
                );
            }
        }
    }
}

/// 文字列を JSON としてパースし、T 型に変換する。
///
/// 変換エラーが発生した場合はエラーにソース文字列を付加する。
fn convert_from_json<'a, T>(json_str: &'a str) -> Result<T>
where
    T: Deserialize<'a>,
{
    let obj = serde_json::from_str::<T>(json_str).with_context(|| json_str.to_string())?;

    Ok(obj)
}

/// HTTP status が成功 (200 台) でなければ Err に変換する。
/// 成功ならば response body を文字列に変換して返す。
async fn process_response(resp: reqwest::Response) -> Result<String> {
    let status = resp.status();
    let text = resp.text().await?;
    if status.is_success() {
        Ok(text)
    } else {
        bail!("HTTP error {} {}", status, text);
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
    http_method: &str,
    base_url: &str,
    oauth_param: &KeyValue,
    query_param: &KeyValue,
    body_param: &KeyValue,
    consumer_secret: &str,
    token_secret: &str,
) -> String {
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
    let encode_add = |param: &mut KeyValue, src: &KeyValue| {
        for (k, v) in src.iter() {
            let old = param.insert(net::percent_encode(k), net::percent_encode(v));
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
        } else {
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
    let result = net::hmac_sha1(signing_key.as_bytes(), signature_base_string.as_bytes());

    // base64 encode したものを署名として "oauth_signature" に設定する
    base64::encode(result.into_bytes())
}

/// HTTP header に設定する (key, value) を文字列として生成して返す。
///
/// Authorization: OAuth key1="value1", key2="value2", ..., keyN="valueN"
fn create_http_oauth_header(oauth_param: &KeyValue) -> (String, String) {
    let mut oauth_value = "OAuth ".to_string();
    {
        let v: Vec<_> = oauth_param
            .iter()
            .map(|(k, v)| format!(r#"{}="{}""#, net::percent_encode(k), net::percent_encode(v)))
            .collect();
        oauth_value.push_str(&v.join(", "));
    }

    ("Authorization".into(), oauth_value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_tweet_text() {
        // 20 chars * 7
        let from1 = "あいうえおかきくけこ0123456789".repeat(7);
        let to1 = Twitter::truncate_tweet_text(&from1).to_string();
        assert_eq!(from1.chars().count(), TWEET_LEN_MAX);
        assert_eq!(from1, to1);

        let from2 = format!("{}あ", from1);
        let to2 = Twitter::truncate_tweet_text(&from2).to_string();
        assert_eq!(from2.chars().count(), TWEET_LEN_MAX + 1);
        assert_eq!(from1, to2);
    }

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
        oauth_param.insert(
            "oauth_nonce".into(),
            "kYjzVBB8Y0ZFabxSWbWovY3uYSQ2pTgmZeNu2VS4cg".into(),
        );
        oauth_param.insert("oauth_signature_method".into(), "HMAC-SHA1".into());
        oauth_param.insert("oauth_timestamp".into(), "1318622958".into());
        oauth_param.insert(
            "oauth_token".into(),
            "370773112-GmHxMAgYyLbNEtIKZeRNFsMKPR9EyMZeS9weJAEb".into(),
        );
        oauth_param.insert("oauth_version".into(), "1.0".into());

        let mut query_param = KeyValue::new();
        query_param.insert("include_entities".into(), "true".into());

        let mut body_param = KeyValue::new();
        body_param.insert(
            "status".into(),
            "Hello Ladies + Gentlemen, a signed OAuth request!".into(),
        );

        // This is just an example in the Twitter API document
        // Not a real secret key
        let consumer_secret = "kAcSOqF21Fu85e7zjz7ZN2U4ZRhfV3WpwPAoE3Z7kBw";
        let token_secret = "LswwdoUaIvS8ltyTt5jkRh4J50vUPVVHtR2YPi5kE";

        let result = create_signature(
            method,
            url,
            &oauth_param,
            &query_param,
            &body_param,
            consumer_secret,
            token_secret,
        );

        assert_eq!(result, "hCtSmYh+iHYCEqBWrE7C7hYmtUk=");
    }
}
