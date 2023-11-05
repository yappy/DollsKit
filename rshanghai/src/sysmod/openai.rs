//! OpenAI API.

pub mod function;

use std::collections::HashMap;
use std::time::Duration;

use super::SystemModule;
use crate::sys::config;
use crate::sys::taskserver::Control;
use crate::utils::netutil;

use anyhow::{anyhow, bail, Result};
use log::info;
use log::warn;
use serde::{Deserialize, Serialize};

const CONN_TIMEOUT: Duration = Duration::from_secs(10);
const TIMEOUT: Duration = Duration::from_secs(60);

/// <https://platform.openai.com/docs/api-reference/chat/create>
const URL_CHAT: &str = "https://api.openai.com/v1/chat/completions";
const URL_IMAGE_GEN: &str = "https://api.openai.com/v1/images/generations";

/// <https://platform.openai.com/docs/models>
///
/// <https://openai.com/pricing>
///
/// (name, max_tokens)
//pub const MODEL: (&str, usize) = ("gpt-3.5-turbo", 4097);
pub const MODEL: (&str, usize) = ("gpt-3.5-turbo-16k", 16385);
/// トークン数制限のうち出力用に予約する割合。
const OUTPUT_RESERVED_RATIO: f32 = 0.2;
/// トークン数制限のうち出力用に予約する数。
pub const OUTPUT_RESERVED_TOKEN: usize = (MODEL.1 as f32 * OUTPUT_RESERVED_RATIO) as usize;

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct ChatMessage {
    /// "system", "user", "assistant", or "function"
    pub role: Role,
    /// Required if role is "function"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Required even if None (null)
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function_call: Option<FunctionCall>,
}

#[derive(Default, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    #[default]
    System,
    User,
    Assistant,
    Function,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FunctionCall {
    pub name: String,
    pub arguments: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Choice {
    pub index: u32,
    pub message: ChatMessage,
    pub finish_reason: String,
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
pub struct ParameterElement {
    /// e.g. "string"
    #[serde(rename = "type")]
    pub type_: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(rename = "enum")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enum_: Option<Vec<String>>,
}

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct Parameters {
    /// "object"
    #[serde(rename = "type")]
    pub type_: String,
    pub properties: HashMap<String, ParameterElement>,
    pub required: Vec<String>,
}

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct Function {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub parameters: Parameters,
}

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,

    #[serde(skip_serializing_if = "Option::is_none")]
    function_call: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    functions: Option<Vec<Function>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    n: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stop: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    user: Option<String>,
}

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
struct ImageGenRequest {
    /// A text description of the desired image(s).
    /// The maximum length is 1000 characters.
    prompt: String,
    /// The number of images to generate. Must be between 1 and 10.
    /// Defaults to 1
    #[serde(skip_serializing_if = "Option::is_none")]
    n: Option<u8>,
    /// The format in which the generated images are returned.
    /// Must be one of url or b64_json.
    /// Defaults to url
    #[serde(skip_serializing_if = "Option::is_none")]
    response_format: Option<String>,
    /// The size of the generated images. Must be one of 256x256, 512x512, or 1024x1024.
    /// Defaults to 1024x1024
    #[serde(skip_serializing_if = "Option::is_none")]
    size: Option<ImageSize>,
    /// A unique identifier representing your end-user,
    /// which can help OpenAI to monitor and detect abuse. Learn more.
    #[serde(skip_serializing_if = "Option::is_none")]
    user: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum ResponseFormat {
    Url,
    B64Json,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum ImageSize {
    #[serde(rename = "256x256")]
    X256,
    #[serde(rename = "512x512")]
    X512,
    #[serde(rename = "1024x1024")]
    X1024,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct ImageGenResponse {
    created: u64,
    data: Vec<Image>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Image {
    b64_json: Option<String>,
    url: Option<String>,
}

/// OpenAI 設定データ。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAiConfig {
    /// OpenAI API 利用を有効にする。
    enabled: bool,
    /// OpenAI API のキー。
    api_key: String,
}

impl Default for OpenAiConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            api_key: "".to_string(),
        }
    }
}

pub struct OpenAi {
    config: OpenAiConfig,
    client: reqwest::Client,
}

impl OpenAi {
    pub fn new() -> Result<Self> {
        info!("[openai] initialize");

        let config = config::get(|cfg| cfg.openai.clone());

        let client = reqwest::Client::builder()
            .connect_timeout(CONN_TIMEOUT)
            .timeout(TIMEOUT)
            .build()?;

        Ok(OpenAi { config, client })
    }

    /// エラーチェインの中から [reqwest] のタイムアウトエラーを探す。
    pub fn is_timeout(err: &anyhow::Error) -> bool {
        for cause in err.chain() {
            if let Some(req_err) = cause.downcast_ref::<reqwest::Error>() {
                if req_err.is_timeout() {
                    return true;
                }
            }
        }

        false
    }

    pub async fn chat(&self, msgs: Vec<ChatMessage>) -> Result<String> {
        let key = &self.config.api_key;
        let body = ChatRequest {
            model: MODEL.0.to_string(),
            messages: msgs,
            ..Default::default()
        };

        info!("[openai] chat request: {:?}", body);
        if !self.config.enabled {
            warn!("[openai] skip because openai feature is disabled");
            bail!("openai is disabled");
        }

        let resp = self
            .client
            .post(URL_CHAT)
            .header("Authorization", format!("Bearer {key}"))
            .json(&body)
            .send()
            .await?;

        let json_str = netutil::check_http_resp(resp).await?;
        let resp_msg: ChatResponse = netutil::convert_from_json(&json_str)?;

        // 最初のを選ぶ
        let choice0 = resp_msg.choices.get(0).ok_or(anyhow!("choices is empty"))?;
        let text = choice0
            .message
            .content
            .as_ref()
            .ok_or(anyhow!("message content is empty"))?
            .clone();

        Ok(text)
    }

    pub async fn chat_with_function(
        &self,
        msgs: &[ChatMessage],
        funcs: &[Function],
    ) -> Result<ChatMessage> {
        let key = &self.config.api_key;
        let body = ChatRequest {
            model: MODEL.0.to_string(),
            messages: msgs.to_vec(),
            functions: Some(funcs.to_vec()),
            ..Default::default()
        };

        info!("[openai] chat request with function: {:?}", body);
        if !self.config.enabled {
            warn!("[openai] skip because openai feature is disabled");
            bail!("openai is disabled");
        }

        let resp = self
            .client
            .post(URL_CHAT)
            .header("Authorization", format!("Bearer {key}"))
            .json(&body)
            .send()
            .await?;

        let json_str = netutil::check_http_resp(resp).await?;
        let resp_msg: ChatResponse = netutil::convert_from_json(&json_str)?;

        // 最初のを選ぶ
        let msg = &resp_msg
            .choices
            .get(0)
            .ok_or(anyhow!("choices is empty"))?
            .message;

        Ok(msg.clone())
    }

    pub async fn generate_image(&self, prompt: &str, n: u8) -> Result<Vec<String>> {
        let key = &self.config.api_key;
        let body = ImageGenRequest {
            prompt: prompt.to_string(),
            n: Some(n),
            size: Some(ImageSize::X256),
            ..Default::default()
        };

        info!("[openai] image gen request: {:?}", body);
        if !self.config.enabled {
            warn!("[openai] skip because openai feature is disabled");
            bail!("openai is disabled");
        }

        let resp = self
            .client
            .post(URL_IMAGE_GEN)
            .header("Authorization", format!("Bearer {key}"))
            .json(&body)
            .send()
            .await?;

        let json_str = netutil::check_http_resp(resp).await?;
        let resp: ImageGenResponse = netutil::convert_from_json(&json_str)?;

        let mut result = Vec::new();
        for img in resp.data.iter() {
            let url = img.url.as_ref().ok_or_else(|| anyhow!("url is required"))?;
            result.push(url.to_string());
        }

        Ok(result)
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
    use crate::utils::netutil::HttpStatusError;
    use serial_test::serial;

    #[tokio::test]
    #[serial(openai)]
    #[ignore]
    // cargo test openai -- --ignored --nocapture
    async fn openai() {
        let src = std::fs::read_to_string("config.toml").unwrap();
        let _unset = config::set(toml::from_str(&src).unwrap());

        let ai = OpenAi::new().unwrap();
        let msgs = vec![
            ChatMessage {
                role: Role::System,
                content: Some("あなたの名前は上海人形で、あなたはやっぴー(yappy)の人形です。あなたはやっぴー家の優秀なアシスタントです。".to_string()),
                ..Default::default()
            },
            ChatMessage {
                role: Role::System,
                content: Some("やっぴーさんは男性で、ホワイト企業に勤めています。yappyという名前で呼ばれることもあります。".to_string()),
                ..Default::default()
            },
            ChatMessage {
                role: Role::User,
                content: Some("こんにちは。システムメッセージから教えられた、あなたの知っている情報を教えてください。".to_string()),
                ..Default::default()
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

    #[tokio::test]
    #[serial(openai)]
    #[ignore]
    // cargo test image_gen -- --ignored --nocapture
    async fn image_gen() -> Result<()> {
        let src = std::fs::read_to_string("config.toml").unwrap();
        let _unset = config::set(toml::from_str(&src).unwrap());

        let ai = OpenAi::new().unwrap();
        let res = ai
            .generate_image("Rasberry Pi の上に乗っている管理人形", 1)
            .await?;
        assert_eq!(1, res.len());

        Ok(())
    }
}
