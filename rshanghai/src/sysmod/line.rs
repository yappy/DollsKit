//! LINE API。
use super::SystemModule;
use crate::sys::{config, taskserver::Control};

use anyhow::{bail, Result};
use log::info;
use serde::{Deserialize, Serialize};
use std::{fmt::Debug, time::Duration};

const TIMEOUT: Duration = Duration::from_secs(15);

/// Discord 設定データ。toml 設定に対応する。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineConfig {
    /// 機能を有効化するなら true。
    enabled: bool,
    /// アクセストークン。Developer Portal で入手できる。
    token: String,
    // OpenAI プロンプト。
    #[serde(default)]
    pub prompt: LinePrompt,
}

impl Default for LineConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            token: "".to_string(),
            prompt: Default::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinePrompt {
    /// 最初に一度だけ与えられるシステムメッセージ。
    pub pre: Vec<String>,
    /// OpenAI API タイムアウト時のメッセージ。
    pub timeout_msg: String,
    /// OpenAI API エラー時のメッセージ。
    pub error_msg: String,
}

/// [LinePrompt] のデフォルト値。
const DEFAULT_TOML: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/src/res/openai_line.toml"
));
impl Default for LinePrompt {
    fn default() -> Self {
        toml::from_str(DEFAULT_TOML).unwrap()
    }
}

/// LINE システムモジュール。
pub struct Line {
    /// 設定データ。
    pub config: LineConfig,
    client: reqwest::Client,
}

impl Line {
    /// コンストラクタ。
    pub fn new() -> Result<Self> {
        info!("[line] initialize");

        let config = config::get(|cfg| cfg.line.clone());
        let client = reqwest::Client::builder().timeout(TIMEOUT).build()?;

        Ok(Self { config, client })
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

#[serde_with::skip_serializing_none]
#[derive(Debug, Serialize)]
struct ReplyReq {
    #[serde(rename = "replyToken")]
    reply_token: String,
    /// len = 1..=5
    messages: Vec<Message>,
    #[serde(rename = "notificationDisabled")]
    notification_disabled: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ReplyResp {
    #[serde(rename = "sentMessages")]
    sent_messages: Vec<SentMessage>,
}

#[derive(Debug, Serialize, Deserialize)]
struct SentMessage {
    id: String,
    #[serde(rename = "quoteToken")]
    quote_token: Option<String>,
}

#[serde_with::skip_serializing_none]
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
enum Message {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "image")]
    Image {
        /// url len <= 5000
        /// protocol = https (>= TLS 1.2)
        /// format = jpeg | png
        /// size <= 10 MB
        #[serde(rename = "originalContentUrl")]
        original_content_url: String,
        /// url len <= 5000
        /// protocol = https (>= TLS 1.2)
        /// format = jpeg | png
        /// size <= 1 MB
        #[serde(rename = "previewImageUrl")]
        preview_image_url: String,
    },
}

const URL_REPLY: &str = "https://api.line.me/v2/bot/message/reply";

impl Line {
    /// <https://developers.line.biz/ja/reference/messaging-api/#send-reply-message>
    pub async fn reply(&self, reply_token: &str, text: &str) -> Result<()> {
        // TODO: check len and split
        let messages = vec![Message::Text {
            text: text.to_string(),
        }];
        let req = ReplyReq {
            reply_token: reply_token.to_string(),
            messages,
            notification_disabled: None,
        };
        let resp = self.post_auth_json(URL_REPLY, &req).await?;
        info!("{:?}", resp);

        Ok(())
    }

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

    async fn post_auth_json<T>(&self, url: &str, body: &T) -> Result<ReplyResp>
    where
        T: Serialize + Debug,
    {
        info!("[line] POST {:?}", body);
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
