use super::SystemModule;
use crate::sys::config;
use crate::sys::taskserver::Control;

use anyhow::{anyhow, bail, Result};
use log::warn;
use log::{info};
use serde::{Deserialize, Serialize};

/// OpenAI API
///
/// https://platform.openai.com/docs/api-reference/chat
const URL_CHAT: &str = "https://api.openai.com/v1/chat/completions";
const MODEL: &str = "gpt-3.5-turbo";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Message {
    /// "system", "user", or "assistant"
    role: String,
    content: String,
}

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
struct ChatRequest {
    model: String,
    messages: Vec<Message>,
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
    enabled: bool,
    api_key: String,
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

    pub async fn chat(&self, msgs: Vec<Message>) -> Result<String> {
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
        let res = client
            .post(URL_CHAT)
            .header("Authorization", format!("Bearer {key}"))
            .json(&body)
            .send()
            .await?
            .text()
            .await?;

        Ok(res)
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

    #[tokio::test]
    #[ignore]
    // cargo test openai -- --ignore --nocapture
    async fn openai() {
        config::add_config(&std::fs::read_to_string("config.json").unwrap()).unwrap();

        let ai = OpenAi::new().unwrap();
        let msgs = vec![
            Message {
                role: "system".to_string(),
                content: "あなたの名前は上海人形で、あなたはやっぴー(yappy)の人形です。あなたはやっぴー家の優秀なアシスタントです。".to_string(),
            },
            Message {
                role: "system".to_string(),
                content: "やっぴーさんは男性で、ホワイト企業に勤めています。".to_string(),
            },
            Message {
                role: "user".to_string(),
                content: "こんにちは。システムメッセージによりあなたの知っている情報を教えてください。".to_string(),
            },
        ];
        let res = ai.chat(msgs).await.unwrap();
        println!("{res}");
    }
}
