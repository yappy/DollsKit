//! LINE API。

use super::SystemModule;
use super::openai::{InputContent, ParameterType};
use super::openai::{
    ParameterElement,
    chat_history::ChatHistory,
    function::{self, BasicContext, FuncArgs, FunctionTable},
};
use crate::config;
use crate::sysmod::openai::{Function, Parameters, function::FUNCTION_TOKEN};
use crate::taskserver::{self, Control};

use anyhow::{Result, anyhow, bail, ensure};
use log::info;
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use std::vec;
use std::{
    collections::HashMap,
    fmt::Debug,
    sync::Arc,
    time::{Duration, Instant},
};

/// LINE API タイムアウト。
const TIMEOUT: Duration = Duration::from_secs(30);
/// [Message::Text] の最大文字数。
/// mention 関連でのずれが少し怖いので余裕を持たせる。
const MSG_SPLIT_LEN: usize = 5000 - 128;

/// Discord 設定データ。toml 設定に対応する。
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct LineConfig {
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
    pub instructions: Vec<String>,
    /// 個々のメッセージの直前に一度ずつ与えらえるシステムメッセージ。
    pub each: Vec<String>,
    /// 会話履歴をクリアするまでの時間。
    pub history_timeout_min: u32,
    /// OpenAI API タイムアウト時のメッセージ。
    pub timeout_msg: String,
    /// OpenAI API レートリミットエラーのメッセージ。
    pub ratelimit_msg: String,
    /// OpenAI API クレジット枯渇エラーのメッセージ。
    pub quota_msg: String,
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
    /// userId, groupIdm or roomId
    pub reply_to: String,
}

/// LINE システムモジュール。
///
/// [Option] は遅延初期化。
pub struct Line {
    /// 設定データ。
    pub config: LineConfig,
    /// HTTP クライアント。
    client: reqwest::Client,

    pub image_buffer: HashMap<String, Vec<InputContent>>,
    /// ai コマンドの会話履歴。
    pub chat_history: Option<ChatHistory>,
    /// [Self::image_buffer] [Self::chat_history] の有効期限。
    pub history_timeout: Option<Instant>,
    /// OpenAI function 機能テーブル
    pub func_table: Option<FunctionTable<FunctionContext>>,
}

impl Line {
    /// コンストラクタ。
    pub fn new() -> Result<Self> {
        info!("[line] initialize");
        let config = config::get(|cfg| cfg.line.clone());
        let client = Client::builder().timeout(TIMEOUT).build()?;

        Ok(Self {
            config,
            client,
            image_buffer: Default::default(),
            chat_history: None,
            history_timeout: None,
            func_table: None,
        })
    }

    async fn init_openai(&mut self, ctrl: &Control) {
        if self.chat_history.is_some() && self.func_table.is_some() {
            return;
        }

        // トークン上限を算出
        // Function 定義 + 前文 + (使用可能上限) + 出力
        let (model_info, reserved) = {
            let openai = ctrl.sysmods().openai.lock().await;

            (
                openai.model_info_offline(),
                openai.get_output_reserved_token(),
            )
        };

        let mut chat_history = ChatHistory::new(model_info.name);
        assert!(chat_history.get_total_limit() == model_info.context_window);
        let pre_token: usize = self
            .config
            .prompt
            .instructions
            .iter()
            .map(|text| chat_history.token_count(text))
            .sum();
        let reserved = FUNCTION_TOKEN + pre_token + reserved;
        chat_history.reserve_tokens(reserved);
        info!("[line] OpenAI token limit");
        info!("[line] {:6} total", model_info.context_window);
        info!("[line] {reserved:6} reserved");
        info!("[line] {:6} chat history", chat_history.usage().1);

        let mut func_table = FunctionTable::new(Arc::clone(ctrl), Some("line"));
        func_table.register_basic_functions();
        register_draw_picture(&mut func_table);

        let _ = self.chat_history.insert(chat_history);
        let _ = self.func_table.insert(func_table);
    }

    pub async fn chat_history(&mut self, ctrl: &Control) -> &ChatHistory {
        self.init_openai(ctrl).await;
        self.chat_history.as_ref().unwrap()
    }

    pub async fn chat_history_mut(&mut self, ctrl: &Control) -> &mut ChatHistory {
        self.init_openai(ctrl).await;
        self.chat_history.as_mut().unwrap()
    }

    pub async fn func_table(&mut self, ctrl: &Control) -> &FunctionTable<FunctionContext> {
        self.init_openai(ctrl).await;
        self.func_table.as_ref().unwrap()
    }

    /*
    pub async fn func_table_mut(&mut self, ctrl: &Control) -> &mut FunctionTable<FunctionContext> {
        self.init_openai(ctrl).await;
        self.func_table.as_mut().unwrap()
    }
    */
}

impl SystemModule for Line {
    fn on_start(&mut self, _ctrl: &Control) {
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

fn url_content(message_id: &str) -> String {
    format!("https://api-data.line.me/v2/bot/message/{message_id}/content")
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

    /// コンテンツを取得する。
    ///
    /// <https://developers.line.biz/ja/reference/messaging-api/#get-content>
    ///
    /// Webhookで受信したメッセージIDを使って、ユーザーが送信した画像、動画、音声、
    /// およびファイルを取得するエンドポイントです。
    /// このエンドポイントは、Webhookイベントオブジェクトの
    /// contentProvider.typeプロパティがlineの場合にのみ利用できます。
    /// ユーザーからデータサイズが大きい動画または音声が送られた場合に、
    /// コンテンツのバイナリデータを取得できるようになるまで時間がかかるときがあります。
    /// バイナリデータの準備中にコンテンツを取得しようとすると、
    /// ステータスコード202が返されバイナリデータは取得できません。
    /// バイナリデータが取得できるかどうかは、
    /// 動画または音声の取得準備の状況を確認するエンドポイントで確認できます。
    /// なお、ユーザーが送信したコンテンツは、
    /// 縮小などの変換が内部的に行われる場合があります。
    pub async fn get_content(&self, message_id: &str) -> Result<Vec<u8>> {
        let bin = loop {
            let (status, bin) = self.get_auth_bin(&url_content(message_id)).await?;
            match status {
                StatusCode::OK => {
                    info!("OK ({} bytes)", bin.len());
                    break bin;
                }
                StatusCode::ACCEPTED => {
                    // 202 Accepted
                    info!("202: Not ready yet");
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
                _ => {
                    bail!("Invalid status: {status}");
                }
            }
        };

        Ok(bin)
    }

    pub fn postpone_timeout(&mut self) {
        let now = Instant::now();
        let timeout = now + Duration::from_secs(self.config.prompt.history_timeout_min as u64 * 60);
        self.history_timeout = Some(timeout);
    }

    /// [Self::image_buffer] [Self::chat_history] にタイムアウトを適用する。
    pub async fn check_history_timeout(&mut self, ctrl: &Control) {
        let now = Instant::now();

        if let Some(timeout) = self.history_timeout {
            if now > timeout {
                self.image_buffer.clear();
                self.chat_history_mut(ctrl).await.clear();
                self.history_timeout = None;
            }
        }
    }

    /// [Self::reply_multi] のシンプル版。
    /// 文字列が長すぎるならば分割して送信する。
    pub async fn reply(&self, reply_token: &str, text: &str) -> Result<ReplyResp> {
        let texts = [text];

        self.reply_multi(reply_token, &texts).await
    }

    /// <https://developers.line.biz/ja/reference/messaging-api/#send-reply-message>
    ///
    /// <https://developers.line.biz/ja/docs/messaging-api/text-character-count/>
    ///
    /// `texts` のそれぞれについて、長すぎるならば分割し、
    /// 文字列メッセージ配列として送信する。
    /// 配列の最大サイズは 5。
    pub async fn reply_multi(&self, reply_token: &str, texts: &[&str]) -> Result<ReplyResp> {
        let mut messages = Vec::new();
        for text in texts {
            ensure!(!text.is_empty(), "text must not be empty");
            let splitted = split_message(text);
            messages.extend(splitted.iter().map(|&chunk| Message::Text {
                text: chunk.to_string(),
            }));
        }
        ensure!(messages.len() <= 5, "text too long: {}", texts.len());

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
    async fn check_resp_json<'a, T>(resp: reqwest::Response) -> Result<T>
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

        Self::check_resp_json(resp).await
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

        Self::check_resp_json(resp).await
    }

    async fn get_auth_bin(&self, url: &str) -> Result<(StatusCode, Vec<u8>)> {
        info!("[line] GET {url}");
        let token = &self.config.token;

        let resp = self
            .client
            .get(url)
            .header("Authorization", format!("Bearer {token}"))
            .send()
            .await?;

        let status = resp.status();
        if status.is_success() {
            let body = resp.bytes().await?.to_vec();

            Ok((status, body))
        } else {
            let body = resp.text().await?;

            match serde_json::from_str::<ErrorResp>(&body) {
                Ok(obj) => bail!("{status}: {:?}", obj),
                Err(json_err) => bail!("{status} - {json_err}: {body}"),
            }
        }
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
        "keywords".to_string(),
        ParameterElement {
            type_: vec![ParameterType::String],
            description: Some("Keywords for drawing. They must be in English.".to_string()),
            ..Default::default()
        },
    );
    func_table.register_function(
        Function {
            name: "draw".to_string(),
            description: Some("Draw a picture".to_string()),
            parameters: Parameters {
                properties,
                required: vec!["keywords".to_string()],
                ..Default::default()
            },
            ..Default::default()
        },
        move |bctx, ctx, args| Box::pin(draw_picture(bctx, ctx, args)),
    );
}

async fn draw_picture(
    bctx: Arc<BasicContext>,
    ctx: FunctionContext,
    args: &FuncArgs,
) -> Result<String> {
    let keywords = function::get_arg_str(args, "keywords")?.to_string();

    let ctrl = bctx.ctrl.clone();
    taskserver::spawn_oneshot_fn(&ctrl, "line_draw_picture", async move {
        let url = {
            let mut ai = bctx.ctrl.sysmods().openai.lock().await;

            ai.generate_image(&keywords, 1)
                .await?
                .pop()
                .ok_or_else(|| anyhow!("parse error"))?
        };
        {
            let line = bctx.ctrl.sysmods().line.lock().await;
            line.push_image_message(&ctx.reply_to, &url).await?;
        }
        Ok(())
    });

    Ok("Accepted. The result will be automatially posted later. Assistant should not draw for now.".to_string())
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
