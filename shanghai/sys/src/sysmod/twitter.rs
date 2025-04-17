//! Twitter 機能。

use super::SystemModule;
use crate::sysmod::openai::InputItem;
use crate::sysmod::openai::Role;
use crate::taskserver::Control;
use crate::{config, taskserver};
use utils::graphics::FontRenderer;
use utils::netutil;

use anyhow::Result;
use base64::{Engine as _, engine::general_purpose};
use chrono::NaiveTime;
use log::warn;
use log::{debug, info};
use rand::Rng;
use reqwest::multipart;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const LONG_TWEET_FONT_SIZE: u32 = 16;
const LONG_TWEET_IMAGE_WIDTH: u32 = 640;
const LONG_TWEET_FGCOLOR: (u8, u8, u8) = (255, 255, 255);
const LONG_TWEET_BGCOLOR: (u8, u8, u8) = (0, 0, 0);

const TIMEOUT: Duration = Duration::from_secs(20);

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
struct Mention {
    start: u32,
    end: u32,
    username: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct HashTag {
    start: u32,
    end: u32,
    tag: String,
}

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
struct Entities {
    #[serde(default)]
    mentions: Vec<Mention>,
    #[serde(default)]
    hashtags: Vec<HashTag>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Includes {
    #[serde(default)]
    users: Vec<User>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Tweet {
    id: String,
    text: String,
    author_id: Option<String>,
    edit_history_tweet_ids: Vec<String>,
    /// tweet.fields=entities
    #[serde(default)]
    entities: Entities,
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
    /// expansions=author_id
    includes: Option<Includes>,
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

/// Twitter 設定データ。toml 設定に対応する。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TwitterConfig {
    /// タイムラインの定期確認を有効にする。
    tlcheck_enabled: bool,
    /// 起動時に1回だけタイムライン確認タスクを起動する。デバッグ用。
    debug_exec_once: bool,
    /// ツイートを実際にはせずにログにのみ出力する。
    fake_tweet: bool,
    /// Twitter API のアカウント情報。
    consumer_key: String,
    /// Twitter API のアカウント情報。
    consumer_secret: String,
    /// Twitter API のアカウント情報。
    access_token: String,
    /// Twitter API のアカウント情報。
    access_secret: String,
    /// OpenAI API 応答を起動するハッシュタグ。
    ai_hashtag: String,
    /// 長文ツイートの画像化に使う ttf ファイルへのパス。
    /// 空文字列にすると機能を無効化する。
    ///
    /// Debian 環境の例\
    /// `sudo apt install fonts-ipafont`\
    /// /usr/share/fonts/truetype/fonts-japanese-gothic.ttf
    font_file: String,
    // タイムラインチェックルール。
    #[serde(default)]
    tlcheck: TimelineCheck,
    /// OpenAI プロンプト。
    #[serde(default)]
    prompt: TwitterPrompt,
}

impl Default for TwitterConfig {
    fn default() -> Self {
        Self {
            tlcheck_enabled: false,
            debug_exec_once: false,
            fake_tweet: true,
            consumer_key: "".to_string(),
            consumer_secret: "".to_string(),
            access_token: "".to_string(),
            access_secret: "".to_string(),
            ai_hashtag: "DollsAI".to_string(),
            font_file: "".to_string(),
            tlcheck: Default::default(),
            prompt: Default::default(),
        }
    }
}

/// Twitter 応答設定データの要素。
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct TimelineCheckRule {
    /// 対象とするユーザ名 (Screen Name) のリスト。
    pub user_names: Vec<String>,
    /// マッチパターンと応答のリスト。
    ///
    /// 前者は検索する文字列の配列。どれか1つにマッチしたら応答を行う。
    /// _^_ で始まる場合、文頭 (行頭ではない) にマッチする。
    /// _$_ で終わる場合、文末 (行末ではない) にマッチする。
    ///
    /// 後者は応答候補の文字列配列。
    /// この中からランダムに1つが選ばれ応答する。
    pub patterns: Vec<(Vec<String>, Vec<String>)>,
}

/// Twitter 応答設定データ。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineCheck {
    /// タイムラインチェックのルール。[TimelineCheckRule] のリスト。
    pub rules: Vec<TimelineCheckRule>,
}

/// [TimelineCheck] のデフォルト値。
const DEFAULT_TLCHECK_TOML: &str =
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/res/tlcheck.toml"));
impl Default for TimelineCheck {
    fn default() -> Self {
        toml::from_str(DEFAULT_TLCHECK_TOML).unwrap()
    }
}

/// OpenAI プロンプト設定。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TwitterPrompt {
    pub pre: Vec<String>,
}

/// [TwitterPrompt] のデフォルト値。
const DEFAULT_PROMPT_TOML: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/res/openai_twitter.toml"
));
impl Default for TwitterPrompt {
    fn default() -> Self {
        toml::from_str(DEFAULT_PROMPT_TOML).unwrap()
    }
}

pub struct Twitter {
    config: TwitterConfig,

    wakeup_list: Vec<NaiveTime>,

    font: Option<FontRenderer>,

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

struct Reply {
    to_tw_id: String,
    to_user_id: String,
    text: String,
    post_image_if_long: bool,
}

impl Twitter {
    pub fn new(wakeup_list: Vec<NaiveTime>) -> Result<Self> {
        info!("[twitter] initialize");

        let config = config::get(|cfg| cfg.twitter.clone());

        let font = if !config.font_file.is_empty() {
            let ttf_bin = fs::read(&config.font_file)?;
            Some(FontRenderer::new(ttf_bin)?)
        } else {
            None
        };

        Ok(Twitter {
            config,
            wakeup_list,
            font,
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
        let rules = self.config.tlcheck.rules.clone();
        for rule in rules.iter() {
            self.resolve_ids(&rule.user_names).await?;
        }
        info!(
            "[tw-check] user id cache size: {}",
            self.username_user_cache.len()
        );

        // 以降メイン処理

        // 自分の最終ツイート以降のタイムラインを得る (リツイートは除く)
        let tl = self.users_timelines_home(&me.id, &since_id).await?;
        info!("{} tweets fetched", tl.data.len());

        // 全リプライを Vec として得る
        let mut reply_buf = self.create_reply_list(&tl, &me);
        // 全 AI リプライを得て追加
        reply_buf.append(&mut self.create_ai_reply_list(ctrl, &tl, &me).await);

        // バッファしたリプライを実行
        for Reply {
            to_tw_id,
            to_user_id,
            text,
            post_image_if_long,
        } in reply_buf
        {
            // since_id 更新用データ
            // tweet id を数値比較のため文字列から変換する
            // (リプライ先 ID + 1) の max をとる
            let cur: u64 = self.tl_check_since_id.as_ref().unwrap().parse().unwrap();
            let next: u64 = to_tw_id.parse().unwrap();
            let max = cur.max(next);

            let name = self.get_username_from_id(&to_user_id).unwrap();
            info!("reply to: {}", name);

            // post_image_if_long が有効で文字数オーバーの場合、画像にして投稿する
            if self.font.is_some() && post_image_if_long && text.chars().count() > TWEET_LEN_MAX {
                let pngbin = self.font.as_ref().unwrap().draw_multiline_text(
                    LONG_TWEET_FGCOLOR,
                    LONG_TWEET_BGCOLOR,
                    &text,
                    LONG_TWEET_FONT_SIZE,
                    LONG_TWEET_IMAGE_WIDTH,
                );
                let media_id = self.media_upload(pngbin).await?;
                self.tweet_custom("", Some(&to_tw_id), &[media_id]).await?;
            } else {
                self.tweet_custom(&text, Some(&to_tw_id), &[]).await?;
            }

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

    /// 全リプライを生成する
    fn create_reply_list(&self, tl: &Timeline, me: &User) -> Vec<Reply> {
        let mut reply_buf = Vec::new();

        for rule in self.config.tlcheck.rules.iter() {
            // 自分のツイートには反応しない
            let tliter = tl
                .data
                .iter()
                // author_id が存在する場合のみ
                .filter(|tw| tw.author_id.is_some())
                // 自分のツイートには反応しない
                .filter(|tw| *tw.author_id.as_ref().unwrap() != me.id)
                // 特定ハッシュタグを含むものは除外 (別関数で返答する)
                .filter(|tw| {
                    !tw.entities
                        .hashtags
                        .iter()
                        .any(|v| v.tag == self.config.ai_hashtag)
                });

            for tw in tliter {
                // author_id が user_names リストに含まれているものでフィルタ
                let user_match = rule.user_names.iter().any(|user_name| {
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
                for (pats, msgs) in rule.patterns.iter() {
                    // 配列内のすべてのパターンを満たす
                    let match_hit = pats.iter().all(|pat| Self::pattern_match(pat, &tw.text));
                    if match_hit {
                        info!("FIND: {:?}", tw);
                        // 配列からリプライをランダムに1つ選ぶ
                        let rnd_idx = rand::rng().random_range(0..msgs.len());
                        reply_buf.push(Reply {
                            to_tw_id: tw.id.clone(),
                            to_user_id: tw.author_id.as_ref().unwrap().clone(),
                            text: msgs[rnd_idx].clone(),
                            post_image_if_long: false,
                        });
                        // 複数種類では反応しない
                        // 反応は1回のみ
                        break;
                    }
                }
            }
        }

        reply_buf
    }

    /// 全 AI リプライを生成する
    async fn create_ai_reply_list(&self, ctrl: &Control, tl: &Timeline, me: &User) -> Vec<Reply> {
        let mut reply_buf = Vec::new();

        let tliter = tl
            .data
            .iter()
            // author_id が存在する場合のみ
            .filter(|tw| tw.author_id.is_some())
            // 自分のツイートには反応しない
            .filter(|tw| *tw.author_id.as_ref().unwrap() != me.id)
            // 自分がメンションされている場合のみ
            .filter(|tw| {
                tw.entities
                    .mentions
                    .iter()
                    .any(|v| v.username == me.username)
            })
            // 設定で指定されたハッシュタグを含む場合のみ対象
            .filter(|tw| {
                tw.entities
                    .hashtags
                    .iter()
                    .any(|v| v.tag == self.config.ai_hashtag)
            });

        for tw in tliter {
            info!("FIND (AI): {:?}", tw);

            let user = Self::resolve_user(
                tw.author_id.as_ref().unwrap(),
                &tl.includes.as_ref().unwrap().users,
            );
            if user.is_none() {
                warn!("User {} is not found", tw.author_id.as_ref().unwrap());
                continue;
            }

            // 設定からプロローグ分の入力メッセージを生成する
            let system_msgs: Vec<_> = self
                .config
                .prompt
                .pre
                .iter()
                .map(|text| {
                    let text = text.replace("${user}", &user.unwrap().name);
                    InputItem::Message {
                        role: Role::Developer,
                        content: text,
                    }
                })
                .collect();

            let mut main_msg = String::new();
            // メンションおよびハッシュタグ部分を削除する
            for (ind, ch) in tw.text.chars().enumerate() {
                let ind = ind as u32;
                let mut deleted = false;
                for m in tw.entities.mentions.iter() {
                    if (m.start..m.end).contains(&ind) {
                        deleted = true;
                        break;
                    }
                }
                for h in tw.entities.hashtags.iter() {
                    if (h.start..h.end).contains(&ind) {
                        deleted = true;
                        break;
                    }
                }
                if !deleted {
                    main_msg.push(ch);
                }
            }

            // 最後にツイートの本文を追加
            let mut msgs = system_msgs.clone();
            msgs.push(InputItem::Message {
                role: Role::User,
                content: main_msg,
            });

            // 結果に追加する
            // エラーはログのみ出して追加をしない
            {
                let mut ai = ctrl.sysmods().openai.lock().await;
                match ai.chat(None, msgs).await {
                    Ok(resp) => reply_buf.push(Reply {
                        to_tw_id: tw.id.clone(),
                        to_user_id: tw.author_id.as_ref().unwrap().clone(),
                        text: resp.output_text(),
                        post_image_if_long: true,
                    }),
                    Err(e) => {
                        warn!("AI chat error: {e}");
                    }
                }
            }
        }

        reply_buf
    }

    fn resolve_user<'a>(id: &str, users: &'a [User]) -> Option<&'a User> {
        users.iter().find(|&user| user.id == id)
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
        self.tweet_custom(text, None, &[]).await
    }

    /// メディア付きツイート。
    /// 中身は [Self::tweet_raw]。
    pub async fn tweet_custom(
        &mut self,
        text: &str,
        reply_to: Option<&str>,
        media_ids: &[u64],
    ) -> Result<()> {
        let reply = reply_to.map(|id| TweetParamReply {
            in_reply_to_tweet_id: id.to_string(),
        });

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
            reply,
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
        let json_str = netutil::check_http_resp(resp).await?;
        let obj: UploadResponseData = netutil::convert_from_json(&json_str)?;
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
                    panic!("parse error {e:?}");
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
                "cannot resolved (account suspended?): {rest:?}"
            );

            start += LIMIT_USERS_BY;
        }
        assert_eq!(self.username_user_cache.len(), self.id_username_cache.len());

        Ok(())
    }

    async fn users_me(&self) -> Result<UsersMe> {
        let resp = self.http_oauth_get(URL_USERS_ME, &KeyValue::new()).await?;
        let json_str = netutil::check_http_resp(resp).await?;
        let obj: UsersMe = netutil::convert_from_json(&json_str)?;

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
        let json_str = netutil::check_http_resp(resp).await?;
        let obj: UsersBy = netutil::convert_from_json(&json_str)?;

        Ok(obj)
    }

    async fn users_timelines_home(&self, id: &str, since_id: &str) -> Result<Timeline> {
        let url = format!(URL_USERS_TIMELINES_HOME!(), id);
        let param = KeyValue::from([
            ("since_id".to_string(), since_id.to_string()),
            ("exclude".to_string(), "retweets".to_string()),
            ("expansions".to_string(), "author_id".to_string()),
            ("tweet.fields".to_string(), "entities".to_string()),
        ]);
        let resp = self.http_oauth_get(&url, &param).await?;
        let json_str = netutil::check_http_resp(resp).await?;
        debug!("{json_str}");
        let obj: Timeline = netutil::convert_from_json(&json_str)?;

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
        let json_str = netutil::check_http_resp(resp).await?;
        let obj: Timeline = netutil::convert_from_json(&json_str)?;

        Ok(obj)
    }

    async fn tweets_post(&self, param: TweetParam) -> Result<TweetResponse> {
        let resp = self
            .http_oauth_post_json(URL_TWEETS, &KeyValue::new(), &param)
            .await?;
        let json_str = netutil::check_http_resp(resp).await?;
        let obj: TweetResponse = netutil::convert_from_json(&json_str)?;

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
            .timeout(TIMEOUT)
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
            .timeout(TIMEOUT)
            .query(query_param)
            .header(oauth_k, oauth_v)
    }
}

impl SystemModule for Twitter {
    fn on_start(&mut self, ctrl: &Control) {
        info!("[twitter] on_start");
        if self.config.tlcheck_enabled {
            if self.config.debug_exec_once {
                taskserver::spawn_oneshot_task(ctrl, "tw-check", Twitter::twitter_task_entry);
            } else {
                taskserver::spawn_periodic_task(
                    ctrl,
                    "tw-check",
                    &self.wakeup_list,
                    Twitter::twitter_task_entry,
                );
            }
        }
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
    let mut rng = rand::rng();
    let rnd32: [u8; 32] = rng.random();
    let rnd32_str = general_purpose::STANDARD.encode(rnd32);
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
            let old = param.insert(netutil::percent_encode(k), netutil::percent_encode(v));
            if old.is_some() {
                panic!("duplicate key: {k}");
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
    signature_base_string.push_str(&netutil::percent_encode(base_url));
    signature_base_string.push('&');
    signature_base_string.push_str(&netutil::percent_encode(&parameter_string));

    // "Getting a signing key"
    // 署名鍵は consumer_secret と token_secret をエスケープして & でつなぐだけ
    let mut signing_key = "".to_string();
    signing_key.push_str(consumer_secret);
    signing_key.push('&');
    signing_key.push_str(token_secret);

    // "Calculating the signature"
    // HMAC SHA1
    let result = netutil::hmac_sha1(signing_key.as_bytes(), signature_base_string.as_bytes());

    // base64 encode したものを署名として "oauth_signature" に設定する
    general_purpose::STANDARD.encode(result.into_bytes())
}

/// HTTP header に設定する (key, value) を文字列として生成して返す。
///
/// Authorization: OAuth key1="value1", key2="value2", ..., keyN="valueN"
fn create_http_oauth_header(oauth_param: &KeyValue) -> (String, String) {
    let mut oauth_value = "OAuth ".to_string();
    {
        let v: Vec<_> = oauth_param
            .iter()
            .map(|(k, v)| {
                format!(
                    r#"{}="{}""#,
                    netutil::percent_encode(k),
                    netutil::percent_encode(v)
                )
            })
            .collect();
        oauth_value.push_str(&v.join(", "));
    }

    ("Authorization".into(), oauth_value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_default_toml() {
        // should not panic
        let obj: TimelineCheck = Default::default();
        assert_ne!(obj.rules.len(), 0);

        let obj: TwitterPrompt = Default::default();
        assert_ne!(obj.pre.len(), 0);
    }

    #[test]
    fn truncate_tweet_text() {
        // 20 chars * 7
        let from1 = "あいうえおかきくけこ0123456789".repeat(7);
        let to1 = Twitter::truncate_tweet_text(&from1).to_string();
        assert_eq!(from1.chars().count(), TWEET_LEN_MAX);
        assert_eq!(from1, to1);

        let from2 = format!("{from1}あ");
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
