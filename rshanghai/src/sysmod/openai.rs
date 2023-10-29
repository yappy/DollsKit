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
const TIMEOUT: Duration = Duration::from_secs(40);

/// <https://platform.openai.com/docs/api-reference/chat/create>
const URL_CHAT: &str = "https://api.openai.com/v1/chat/completions";

/// <https://platform.openai.com/docs/models>
///
/// <https://openai.com/pricing>
///
/// (name, max_tokens)
//pub const MODEL: (&str, usize) = ("gpt-3.5-turbo", 4097);
pub const MODEL: (&str, usize) = ("gpt-3.5-turbo-16k", 16385);

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FunctionCall {
    pub name: String,
    pub arguments: String,
}

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct ChatMessage {
    /// "system", "user", "assistant", or "function"
    pub role: String,
    /// Required if role is "function"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Required even if None (null)
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function_call: Option<FunctionCall>,
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
}

impl SystemModule for OpenAi {
    fn on_start(&self, _ctrl: &Control) {
        info!("[openai] on_start");
    }
}

#[cfg(test)]
mod tests {
    use super::function::*;
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
                role: "system".to_string(),
                content: Some("あなたの名前は上海人形で、あなたはやっぴー(yappy)の人形です。あなたはやっぴー家の優秀なアシスタントです。".to_string()),
                ..Default::default()
            },
            ChatMessage {
                role: "system".to_string(),
                content: Some("やっぴーさんは男性で、ホワイト企業に勤めています。yappyという名前で呼ばれることもあります。".to_string()),
                ..Default::default()
            },
            ChatMessage {
                role: "user".to_string(),
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
    // cargo test chat_function -- --ignored --nocapture
    async fn chat_function() {
        let src = std::fs::read_to_string("config.toml").unwrap();
        let _unset = config::set(toml::from_str(&src).unwrap());

        let mut func_table = FunctionTable::new();
        func_table.register_all_functions();

        let ai = OpenAi::new().unwrap();
        let mut msgs = vec![ChatMessage {
            role: "user".to_string(),
            content: Some("こんにちは。今は何時でしょうか？".to_string()),
            ..Default::default()
        }];
        let funcs = func_table.function_list();

        // function call が返ってくる
        println!("{:?}\n", msgs);
        let reply = match ai.chat_with_function(&msgs, funcs).await {
            Ok(msgs) => msgs,
            Err(err) => {
                println!("{err}");
                // HTTP status が得られるタイプのエラーのみ許容する
                let _err = err.downcast_ref::<HttpStatusError>().unwrap();
                return;
            }
        };
        println!("{:?}\n", reply);

        assert!(reply.role == "assistant");
        assert!(reply.content.is_none());
        assert!(reply.function_call.as_ref().unwrap().name == "get_current_datetime");

        let func_name = &reply.function_call.as_ref().unwrap().name;
        let func_args = &reply.function_call.as_ref().unwrap().arguments;
        let func_res = func_table.call(func_name, func_args).await;

        // function call と呼び出し結果を対話ログに追加
        msgs.push(reply);
        msgs.push(func_res);

        // function の結果を使った応答が返ってくる
        println!("{:?}\n", msgs);
        let reply = match ai.chat_with_function(&msgs, funcs).await {
            Ok(msgs) => msgs,
            Err(err) => {
                println!("{err}");
                // HTTP status が得られるタイプのエラーのみ許容する
                let _err = err.downcast_ref::<HttpStatusError>().unwrap();
                return;
            }
        };
        println!("{:?}", reply);

        assert!(reply.role == "assistant");
        assert!(reply.content.is_some());
        assert!(reply.function_call.is_none());
    }
}
