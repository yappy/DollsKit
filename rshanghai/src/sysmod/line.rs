//! LINE API。
use super::{
    openai::{
        function::{FuncArgs, FuncBodyAsync, FunctionTable},
        ParameterElement,
    },
    SystemModule,
};
use crate::{
    sys::{config, taskserver::Control},
    sysmod::openai::{self, function::FUNCTION_TOKEN, Function, Parameters},
    utils::chat_history::{self, ChatHistory},
};

use anyhow::{anyhow, bail, ensure, Result};
use log::info;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fmt::Debug,
    time::{Duration, Instant},
};

/// LINE API タイムアウト。
const TIMEOUT: Duration = Duration::from_secs(15);
/// [Message::Text] の最大文字数。
/// mention 関連でのずれが少し怖いので余裕を持たせる。
const MSG_SPLIT_LEN: usize = 5000 - 128;

/// Discord 設定データ。toml 設定に対応する。
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct LineConfig {
    // TODO
    /// 機能を有効化するなら true。
    enabled: bool,
    /// アクセストークン。Developer Portal で入手できる。
    token: String,
    /// チャネルシークレット。
    pub channel_secret: String,
    /// LINE ID から名前への固定マップ。
    pub id_name_map: HashMap<String, String>,
    // OpenAI プロンプト。
    #[serde(default)]
    pub prompt: LinePrompt,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinePrompt {
    /// 最初に一度だけ与えられるシステムメッセージ。
    pub pre: Vec<String>,
    /// 個々のメッセージの直前に一度ずつ与えらえるシステムメッセージ。
    pub each: Vec<String>,
    /// 会話履歴をクリアするまでの時間。
    pub history_timeout_min: u32,
    /// OpenAI API タイムアウト時のメッセージ。
    pub timeout_msg: String,
    /// OpenAI API エラー時のメッセージ。
    pub error_msg: String,
}

/// [LinePrompt] のデフォルト値。
const DEFAULT_TOML: &str =
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/res/openai_line.toml"));
impl Default for LinePrompt {
    fn default() -> Self {
        toml::from_str(DEFAULT_TOML).unwrap()
    }
}

pub struct FunctionContext {
    pub ctrl: Control,
    /// userId, groupIdm or roomId
    pub reply_to: String,
}

/// LINE システムモジュール。
pub struct Line {
    /// 設定データ。
    pub config: LineConfig,
    /// HTTP クライアント。
    client: reqwest::Client,

    /// ai コマンドの会話履歴。
    pub chat_history: ChatHistory,
    /// [Self::chat_history] の有効期限。
    pub chat_timeout: Option<Instant>,
    /// OpenAI function 機能テーブル
    pub func_table: FunctionTable<FunctionContext>,
}

impl Line {
    /// コンストラクタ。
    pub fn new() -> Result<Self> {
        info!("[line] initialize");

        let config = config::get(|cfg| cfg.line.clone());

        // トークン上限を算出
        let total_limit = openai::MODEL.1;
        let pre_token: usize = config
            .prompt
            .pre
            .iter()
            .map(|text| chat_history::token_count(text))
            .sum();
        assert!(FUNCTION_TOKEN + pre_token + openai::OUTPUT_RESERVED_TOKEN < total_limit);
        let chat_history = ChatHistory::new(
            total_limit - FUNCTION_TOKEN - pre_token - openai::OUTPUT_RESERVED_TOKEN,
        );

        let mut func_table = FunctionTable::new();
        func_table.register_basic_functions();
        register_draw_picture(&mut func_table);

        let client = Client::builder().timeout(TIMEOUT).build()?;

        Ok(Self {
            config,
            client,
            chat_history,
            chat_timeout: None,
            func_table,
        })
    }
}

impl SystemModule for Line {
    fn on_start(&self, _ctrl: &Control) {
        info!("[line] on_start");
    }
}

////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
struct ErrorResp {
    message: String,
    details: Option<Vec<Detail>>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Detail {
    message: Option<String>,
    property: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileResp {
    pub display_name: String,
    pub user_id: String,
    pub language: Option<String>,
    pub picture_url: Option<String>,
    pub status_message: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde_with::skip_serializing_none]
struct ReplyReq {
    reply_token: String,
    /// len = 1..=5
    messages: Vec<Message>,
    notification_disabled: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReplyResp {
    sent_messages: Vec<SentMessage>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde_with::skip_serializing_none]
struct PushReq {
    to: String,
    /// len = 1..=5
    messages: Vec<Message>,
    notification_disabled: Option<bool>,
    custom_aggregation_units: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PushResp {
    sent_messages: Vec<SentMessage>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SentMessage {
    id: String,
    quote_token: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(tag = "type")]
#[serde_with::skip_serializing_none]
enum Message {
    #[serde(rename_all = "camelCase")]
    Text { text: String },
    #[serde(rename_all = "camelCase")]
    Image {
        /// url len <= 5000
        /// protocol = https (>= TLS 1.2)
        /// format = jpeg | png
        /// size <= 10 MB
        original_content_url: String,
        /// url len <= 5000
        /// protocol = https (>= TLS 1.2)
        /// format = jpeg | png
        /// size <= 1 MB
        preview_image_url: String,
    },
}

fn url_profile(user_id: &str) -> String {
    format!("https://api.line.me/v2/bot/profile/{user_id}")
}
fn url_group_profile(group_id: &str, user_id: &str) -> String {
    format!("https://api.line.me/v2/bot/group/{group_id}/member/{user_id}")
}

const URL_REPLY: &str = "https://api.line.me/v2/bot/message/reply";
const URL_PUSH: &str = "https://api.line.me/v2/bot/message/push";

impl Line {
    pub async fn get_profile(&self, user_id: &str) -> Result<ProfileResp> {
        self.get_auth_json(&url_profile(user_id)).await
    }

    pub async fn get_group_profile(&self, group_id: &str, user_id: &str) -> Result<ProfileResp> {
        self.get_auth_json(&url_group_profile(group_id, user_id))
            .await
    }

    /// [Self::chat_history] にタイムアウトを適用する。
    pub fn check_history_timeout(&mut self) {
        let now = Instant::now();

        if let Some(timeout) = self.chat_timeout {
            if now > timeout {
                self.chat_history.clear();
                self.chat_timeout = None;
            }
        }
    }

    /// <https://developers.line.biz/ja/reference/messaging-api/#send-reply-message>
    ///
    /// <https://developers.line.biz/ja/docs/messaging-api/text-character-count/>
    pub async fn reply(&self, reply_token: &str, text: &str) -> Result<ReplyResp> {
        ensure!(!text.is_empty(), "text must not be empty");

        let messages: Vec<_> = split_message(text)
            .iter()
            .map(|&chunk| Message::Text {
                text: chunk.to_string(),
            })
            .collect();
        ensure!(messages.len() <= 5, "text too long: {}", text.len());

        let req = ReplyReq {
            reply_token: reply_token.to_string(),
            messages,
            notification_disabled: None,
        };
        let resp = self.post_auth_json(URL_REPLY, &req).await?;
        info!("{:?}", resp);

        Ok(resp)
    }

    /// <https://developers.line.biz/ja/reference/messaging-api/#send-push-message>
    ///
    /// <https://developers.line.biz/ja/docs/messaging-api/text-character-count/>
    #[allow(unused)]
    pub async fn push_message(&self, to: &str, text: &str) -> Result<ReplyResp> {
        ensure!(!text.is_empty(), "text must not be empty");

        let messages: Vec<_> = split_message(text)
            .iter()
            .map(|&chunk| Message::Text {
                text: chunk.to_string(),
            })
            .collect();
        ensure!(messages.len() <= 5, "text too long: {}", text.len());

        let req = PushReq {
            to: to.to_string(),
            messages,
            notification_disabled: None,
            custom_aggregation_units: None,
        };
        let resp = self.post_auth_json(URL_PUSH, &req).await?;
        info!("{:?}", resp);

        Ok(resp)
    }

    /// <https://developers.line.biz/ja/reference/messaging-api/#send-push-message>
    pub async fn push_image_message(&self, to: &str, url: &str) -> Result<ReplyResp> {
        let messages = vec![Message::Image {
            original_content_url: url.to_string(),
            preview_image_url: url.to_string(),
        }];

        let req = PushReq {
            to: to.to_string(),
            messages,
            notification_disabled: None,
            custom_aggregation_units: None,
        };
        let resp = self.post_auth_json(URL_PUSH, &req).await?;
        info!("{:?}", resp);

        Ok(resp)
    }

    /// レスポンスの内容を確認しながら json に変換する。
    ///
    /// * HTTP Status が成功の場合
    ///   * Response body JSON を T に変換できればそれを返す。
    ///   * 変換に失敗したらエラー。
    /// * HTTP Status が失敗の場合
    ///   * Response body JSON を [ErrorResp] にパースできればその [Debug] を
    ///     メッセージとしてエラーを返す。
    ///   * 変換に失敗した場合、JSON ソースをメッセージとしてエラーを返す。
    async fn check_resp<'a, T>(resp: reqwest::Response) -> Result<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        // https://qiita.com/kzee/items/d01e6f3e00dfadb9a00b
        let status = resp.status();
        let body = resp.text().await?;

        if status.is_success() {
            Ok(serde_json::from_reader::<_, T>(body.as_bytes())?)
        } else {
            match serde_json::from_str::<ErrorResp>(&body) {
                Ok(obj) => bail!("{status}: {:?}", obj),
                Err(json_err) => bail!("{status} - {json_err}: {body}"),
            }
        }
    }

    async fn get_auth_json<'a, T>(&self, url: &str) -> Result<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        info!("[line] GET {url}");
        let token = &self.config.token;
        let resp = self
            .client
            .get(url)
            .header("Authorization", format!("Bearer {token}"))
            .send()
            .await?;

        Self::check_resp(resp).await
    }

    async fn post_auth_json<T, R>(&self, url: &str, body: &T) -> Result<R>
    where
        T: Serialize + Debug,
        R: for<'de> Deserialize<'de>,
    {
        info!("[line] POST {url} {:?}", body);
        let token = &self.config.token;
        let resp = self
            .client
            .post(url)
            .header("Authorization", format!("Bearer {token}"))
            .json(body)
            .send()
            .await?;

        Self::check_resp(resp).await
    }
}

fn split_message(text: &str) -> Vec<&str> {
    // UTF-16 5000 文字で分割
    let mut result = Vec::new();
    // 左端
    let mut s = 0;
    // utf-16 文字数
    let mut len = 0;
    for (ind, c) in text.char_indices() {
        // 1 or 2
        let clen = c.len_utf16();
        // 超えそうなら [s, ind) の部分文字列を出力
        if len + clen > MSG_SPLIT_LEN {
            result.push(&text[s..ind]);
            s = ind;
            len = 0;
        }
        len += clen;
    }
    if len > 0 {
        result.push(&text[s..]);
    }

    result
}

fn register_draw_picture(func_table: &mut FunctionTable<FunctionContext>) {
    let mut properties = HashMap::new();
    properties.insert(
        "prompt".to_string(),
        ParameterElement {
            type_: "string".to_string(),
            description: Some("Prompt string that will be used for drawing".to_string()),
            ..Default::default()
        },
    );
    func_table.register_function(
        Function {
            name: "draw".to_string(),
            description: Some("Draw a picture".to_string()),
            parameters: Parameters {
                type_: "object".to_string(),
                properties,
                required: vec!["prompt".to_string()],
            },
        },
        "draw",
        Box::new(draw_picture_sync),
    );
}

fn draw_picture_sync(ctx: FunctionContext, args: &FuncArgs) -> FuncBodyAsync {
    Box::pin(draw_picture(ctx, args))
}

async fn draw_picture(ctx: FunctionContext, _args: &FuncArgs) -> Result<String> {
    let ctrl = ctx.ctrl.clone();
    ctrl.spawn_oneshot_fn("line_draw_picture", async move {
        let url = {
            let ai = ctx.ctrl.sysmods().openai.lock().await;

            ai.generate_image("test", 1)
                .await?
                .pop()
                .ok_or_else(|| anyhow!("parse error"))?
        };
        {
            let line = ctx.ctrl.sysmods().line.lock().await;
            line.push_image_message(&ctx.reply_to, &url).await?;
        }
        Ok(())
    });

    Ok("Accepted. Will be posted later. Wait a moment.".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_long_message() {
        let mut src = String::new();
        assert!(split_message(&src).is_empty());

        for i in 0..MSG_SPLIT_LEN {
            let a = 'A' as u32;
            src.push(char::from_u32(a + (i as u32 % 26)).unwrap());
        }
        let res = split_message(&src);
        assert_eq!(1, res.len());
        assert_eq!(src, res[0]);

        src.push('0');
        let res = split_message(&src);
        assert_eq!(2, res.len());
        assert_eq!(&src[..MSG_SPLIT_LEN], res[0]);
        assert_eq!(&src[MSG_SPLIT_LEN..], res[1]);

        src.pop();
        src.pop();
        src.push('😀');
        let res = split_message(&src);
        assert_eq!(2, res.len());
        assert_eq!(&src[..MSG_SPLIT_LEN - 1], res[0]);
        assert_eq!(&src[MSG_SPLIT_LEN - 1..], res[1]);
    }
}
