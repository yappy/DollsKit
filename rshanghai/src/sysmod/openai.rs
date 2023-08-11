//! OpenAI API.

use super::SystemModule;
use crate::sys::config;
use crate::sys::netutil;
use crate::sys::taskserver::Control;

use anyhow::{anyhow, bail, Result};
use log::info;
use log::warn;
use serde::{Deserialize, Serialize};

/// <https://platform.openai.com/docs/api-reference/chat/create>
const URL_CHAT: &str = "https://api.openai.com/v1/chat/completions";
const MODEL: &str = "gpt-3.5-turbo";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChatMessage {
    /// "system", "user", or "assistant"
    pub role: String,
    pub content: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Choice {
    pub message: ChatMessage,
    pub finish_reason: String,
    pub index: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct ChatResponse {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub usage: Usage,
    pub choices: Vec<Choice>,
}

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    n: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stop: Option<Vec<String>>,
}

/// OpenAI 設定データ。
#[derive(Clone, Serialize, Deserialize)]
pub struct OpenAiConfig {
    /// OpenAI API 利用を有効にする。
    enabled: bool,
    /// OpenAI API のキー。
    api_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAiPromptDiscord {
    /// 最初に一度だけ与えられるシステムメッセージ。
    pub first: Vec<String>,
    /// 個々のメッセージの直前に一度ずつ与えらえるシステムメッセージ。
    pub each: Vec<String>,
    pub history_max: u32,
    pub history_timeout_min: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAiPrompt {
    /// role="system" で与えられる。
    /// ${user} は発言者の名前で置換される。
    pub twitter: Vec<String>,
    /// role="system" で与えられる。
    /// ${user} は発言者の名前で置換される。
    pub discord: OpenAiPromptDiscord,
}

pub struct OpenAi {
    config: OpenAiConfig,
}

impl OpenAi {
    pub fn new() -> Result<Self> {
        info!("[openai] initialize");

        let jsobj =
            config::get_object(&["openai"]).map_or(Err(anyhow!("Config not found: openai")), Ok)?;
        let config: OpenAiConfig = serde_json::from_value(jsobj)?;

        Ok(OpenAi { config })
    }

    pub async fn chat(&self, msgs: Vec<ChatMessage>) -> Result<String> {
        let key = &self.config.api_key;
        let body = ChatRequest {
            model: MODEL.to_string(),
            messages: msgs,
            ..Default::default()
        };

        info!("[openai] chat request: {:?}", body);
        if !self.config.enabled {
            warn!("[openai] skip because openai feature is disabled");
            bail!("openai is disabled");
        }

        let client = reqwest::Client::new();
        let resp = client
            .post(URL_CHAT)
            .header("Authorization", format!("Bearer {key}"))
            .json(&body)
            .send()
            .await?;

        let json_str = netutil::check_http_resp(resp).await?;
        let resp_msg: ChatResponse = netutil::convert_from_json(&json_str)?;

        // 複数候補が返ってくることがあるらしいが、よく分からないので最初のを選ぶ
        let text = resp_msg
            .choices
            .get(0)
            .ok_or(anyhow!("choices is empty"))?
            .message
            .content
            .clone();

        Ok(text)
    }
}

impl SystemModule for OpenAi {
    fn on_start(&self, _ctrl: &Control) {
        info!("[openai] on_start");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sys::netutil::HttpStatusError;

    #[tokio::test]
    #[ignore]
    // cargo test openai -- --ignored --nocapture
    async fn openai() {
        config::add_config(&std::fs::read_to_string("config.json").unwrap()).unwrap();

        let ai = OpenAi::new().unwrap();
        let msgs = vec![
            ChatMessage {
                role: "system".to_string(),
                content: "あなたの名前は上海人形で、あなたはやっぴー(yappy)の人形です。あなたはやっぴー家の優秀なアシスタントです。".to_string(),
            },
            ChatMessage {
                role: "system".to_string(),
                content: "やっぴーさんは男性で、ホワイト企業に勤めています。yappyという名前で呼ばれることもあります。".to_string(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: "こんにちは。システムメッセージから教えられた、あなたの知っている情報を教えてください。".to_string(),
            },
        ];
        let resp = match ai.chat(msgs).await {
            Ok(text) => text,
            Err(err) => {
                // HTTP status が得られるタイプのエラーのみ許容する
                let err = err.downcast_ref::<HttpStatusError>().unwrap();
                err.to_string()
            }
        };
        println!("{resp}");
    }
}
