//! OpenAI API.

mod basicfuncs;
pub mod function;

use std::collections::HashMap;
use std::sync::LazyLock;
use std::time::{Duration, SystemTime};

use super::SystemModule;
use crate::sys::config;
use crate::sys::taskserver::Control;
use crate::utils::netutil;

use anyhow::ensure;
use anyhow::{Result, anyhow, bail};
use log::info;
use log::warn;
use serde::{Deserialize, Serialize};

/// HTTP 通信のタイムアウト。
/// これを設定しないと無限待ちになる危険性がある。
const CONN_TIMEOUT: Duration = Duration::from_secs(10);
/// AI 応答待ちのタイムアウト。
const TIMEOUT: Duration = Duration::from_secs(60);
/// モデル情報更新間隔。
/// 24 時間に一度更新する。
const MODEL_INFO_UPDATE_INTERVAL: Duration = Duration::from_secs(24 * 3600);

/// <https://platform.openai.com/docs/api-reference/chat/create>
fn url_model(model: &str) -> String {
    format!("https://api.openai.com/v1/models{model}")
}
const URL_CHAT: &str = "https://api.openai.com/v1/chat/completions";
const URL_IMAGE_GEN: &str = "https://api.openai.com/v1/images/generations";
const URL_AUDIO_SPEECH: &str = "https://api.openai.com/v1/audio/speech";

/// モデル情報。
/// API からは得られない、ドキュメントにのみある情報。
#[derive(Debug, Clone, Copy, Serialize)]
pub struct OfflineModelInfo {
    pub name: &'static str,
    /// 総トークン数制限。入力と出力その他コントロールトークン全てを合わせた値。
    pub context_window: usize,
    /// 最大出力トークン数。
    pub max_output_tokens: usize,
}

/// モデル情報。
/// API から得られるデータ。時々でよいので再取得する必要がある。
#[derive(Debug, Clone, Serialize)]
pub struct OnlineModelInfo {
    last_update: SystemTime,
    info: Model,
}

/// [OfflineModelInfo] + (Online) [Model]。
#[derive(Debug, Clone, Serialize)]
pub struct ModelInfo {
    pub offline: OfflineModelInfo,
    pub online: Model,
}

/// モデル情報。一番上がデフォルト。
///
/// <https://platform.openai.com/docs/models>
///
/// <https://openai.com/pricing>
const MODEL_LIST: &[OfflineModelInfo] = &[
    OfflineModelInfo {
        name: "gpt-4o-mini",
        context_window: 128000,
        max_output_tokens: 4096,
    },
    OfflineModelInfo {
        name: "gpt-4o",
        context_window: 128000,
        max_output_tokens: 4096,
    },
    OfflineModelInfo {
        name: "gpt-4",
        context_window: 8192,
        max_output_tokens: 8192,
    },
    OfflineModelInfo {
        name: "gpt-4-turbo",
        context_window: 128000,
        max_output_tokens: 4096,
    },
];

/// `max_output_tokens` をギリギリまで攻めると危ないので、少し余裕を持たせる。
const MAX_OUTPUT_TOKENS_FACTOR: f32 = 1.05;

/// `context_window` のうち出力用に予約する割合 (まともに決まっていない場合用)。
/// `max_output_tokens` が意味をなしていない gpt-4 で適当に決めるための値。
const OUTPUT_RESERVED_RATIO: f32 = 0.2;

/// [MODEL_LIST] からモデル名で [ModelInfo] を検索する。
///
/// HashMap で検索する。
fn get_offline_model_info(model: &str) -> Result<&OfflineModelInfo> {
    static MAP: LazyLock<HashMap<&str, &OfflineModelInfo>> = LazyLock::new(|| {
        let mut map = HashMap::new();
        for info in MODEL_LIST.iter() {
            map.insert(info.name, info);
        }

        map
    });

    MAP.get(model)
        .copied()
        .ok_or_else(|| anyhow!("Model not found: {model}"))
}

/// OpenAI API JSON 定義。
/// モデル情報。
#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct Model {
    /// The model identifier, which can be referenced in the API endpoints.
    id: String,
    /// The Unix timestamp (in seconds) when the model was created.
    created: u64,
    /// The object type, which is always "model".
    object: String,
    ///The organization that owns the model.
    owned_by: String,
}

/// OpenAI API JSON 定義。
/// 会話メッセージ。
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

/// OpenAI API JSON 定義。
/// [ChatMessage] に設定されるロール。
#[derive(Default, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    #[default]
    System,
    User,
    Assistant,
    Function,
}

/// OpenAI API JSON 定義。
/// function 呼び出し。
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FunctionCall {
    pub name: String,
    pub arguments: String,
}

/// OpenAI API JSON 定義。
/// トークン消費量。
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

/// OpenAI API JSON 定義。
/// 応答案。
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Choice {
    pub index: u32,
    pub message: ChatMessage,
    pub finish_reason: String,
}

/// OpenAI API JSON 定義。
/// 会話応答データ。
#[derive(Clone, Debug, Serialize, Deserialize)]
struct ChatResponse {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub usage: Usage,
    pub choices: Vec<Choice>,
}

/// OpenAI API JSON 定義。
/// function パラメータ定義 (JSON Schema)。
///
/// <https://json-schema.org/understanding-json-schema>
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minumum: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maximum: Option<i64>,
}

/// OpenAI API JSON 定義。
/// function パラメータ定義。
#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct Parameters {
    /// "object"
    #[serde(rename = "type")]
    pub type_: String,
    pub properties: HashMap<String, ParameterElement>,
    pub required: Vec<String>,
}

/// OpenAI API JSON 定義。
/// function 定義。
#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct Function {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub parameters: Parameters,
}

/// OpenAI API JSON 定義。
/// 会話リクエスト。
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

/// OpenAI API JSON 定義。
/// 画像生成リクエスト。
///
/// <https://platform.openai.com/docs/api-reference/images>
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

/// OpenAI API JSON 定義。
/// 画像生成のフォーマット。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum ResponseFormat {
    Url,
    B64Json,
}

/// OpenAI API JSON 定義。
/// 画像生成のサイズ。
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

/// OpenAI API JSON 定義。
/// 画像生成レスポンス。
#[derive(Clone, Debug, Serialize, Deserialize)]
struct ImageGenResponse {
    created: u64,
    data: Vec<Image>,
}

/// OpenAI API JSON 定義。
/// 画像データ。
#[derive(Clone, Debug, Serialize, Deserialize)]
struct Image {
    b64_json: Option<String>,
    url: Option<String>,
}

/// OpenAI API JSON 定義。
/// 音声生成リクエスト。
///
///<https://platform.openai.com/docs/api-reference/audio/createSpeech>
#[derive(Default, Clone, Debug, Serialize, Deserialize)]
struct SpeechRequest {
    /// One of the available TTS models: tts-1 or tts-1-hd
    model: SpeechModel,
    /// The text to generate audio for.
    /// The maximum length is 4096 characters.
    input: String,
    /// The voice to use when generating the audio.
    /// Supported voices are alloy, echo, fable, onyx, nova, and shimmer.
    /// Previews of the voices are available in the Text to speech guide.
    voice: SpeechVoice,
    /// The format to audio in.
    /// Supported formats are mp3, opus, aac, flac, wav, and pcm.
    /// Defaults to mp3
    #[serde(skip_serializing_if = "Option::is_none")]
    response_format: Option<SpeechFormat>,
    /// The speed of the generated audio.
    /// Select a value from 0.25 to 4.0. 1.0 is the default.
    #[serde(skip_serializing_if = "Option::is_none")]
    speed: Option<f32>,
}

#[derive(Default, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpeechModel {
    /// The latest text to speech model, optimized for speed.
    #[serde(rename = "tts-1")]
    #[default]
    Tts1,
    /// The latest text to speech model, optimized for quality.
    #[serde(rename = "tts-1-hd")]
    Tts1Hd,
}

pub const SPEECH_INPUT_MAX: usize = 4096;

#[derive(Default, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpeechVoice {
    #[default]
    Alloy,
    Echo,
    Fable,
    Onyx,
    Nova,
    Shimmer,
}

#[derive(Default, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpeechFormat {
    #[default]
    Mp3,
    OpuS,
    Aac,
    Flac,
    Wav,
    Pcm,
}

pub const SPEECH_SPEED_MIN: f32 = 0.25;
pub const SPEECH_SPEED_MAX: f32 = 4.0;

/// OpenAI 設定データ。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAiConfig {
    /// OpenAI API 利用を有効にする。
    enabled: bool,
    /// OpenAI API のキー。
    api_key: String,
    /// 使用するモデル名。
    /// [MODEL_LIST] から選択。
    pub model: String,
    /// ストレージディレクトリ。
    /// 空文字列だと機能を無効にする。
    pub storage_dir: String,
}

impl Default for OpenAiConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            api_key: "".to_string(),
            model: MODEL_LIST.first().unwrap().name.to_string(),
            storage_dir: "./aimemory".to_string(),
        }
    }
}

/// OpenAI システムモジュール。
pub struct OpenAi {
    config: OpenAiConfig,
    client: reqwest::Client,
    model_name: &'static str,
    model_info_offline: OfflineModelInfo,
    model_info_online: Option<OnlineModelInfo>,
}

impl OpenAi {
    /// コンストラクタ。
    pub fn new() -> Result<Self> {
        info!("[openai] initialize");

        let config = config::get(|cfg| cfg.openai.clone());

        info!("[openai] OpenAI model list START");
        for info in MODEL_LIST.iter() {
            info!(
                "[openai] name: \"{}\", context_window: {}",
                info.name, info.context_window
            );
        }
        info!("[openai] OpenAI model list END");

        let info = get_offline_model_info(&config.model)?;
        info!(
            "[openai] selected: model: {}, token_limit: {}",
            info.name, info.context_window
        );

        if !config.storage_dir.is_empty() {
            info!("[openai] mkdir: {}", config.storage_dir);
            std::fs::create_dir_all(&config.storage_dir)?;
        }

        let client = reqwest::Client::builder()
            .connect_timeout(CONN_TIMEOUT)
            .timeout(TIMEOUT)
            .build()?;

        Ok(OpenAi {
            config: config.clone(),
            client,
            model_name: info.name,
            model_info_offline: *info,
            model_info_online: None,
        })
    }

    pub fn model_name(&self) -> &str {
        self.model_name
    }

    pub fn model_info_offline(&self) -> OfflineModelInfo {
        self.model_info_offline
    }

    pub async fn model_info_online(&mut self) -> Result<Model> {
        let cur = &self.model_info_online;
        let update = if let Some(info) = cur {
            let now = SystemTime::now();
            let elapsed = now.duration_since(info.last_update).unwrap_or_default();

            elapsed > MODEL_INFO_UPDATE_INTERVAL
        } else {
            true
        };

        if update {
            let info = self.get_online_model_info().await?;
            let newval = OnlineModelInfo {
                last_update: SystemTime::now(),
                info: info.clone(),
            };
            self.model_info_online.insert(newval);

            Ok(info)
        } else {
            Ok(cur.as_ref().unwrap().info.clone())
        }
    }

    /// 出力用に予約するトークン数を計算する。
    /// 基本的に max_output_tokens に余裕を持たせた値を使うが、
    /// それが意味をなしていない旧モデルの場合は context_window のうち一定割合とする。
    pub fn get_output_reserved_token(&self) -> usize {
        let info = self.model_info_offline();
        let v1 = (info.max_output_tokens as f32 * MAX_OUTPUT_TOKENS_FACTOR) as usize;
        let v2 = (info.context_window as f32 * OUTPUT_RESERVED_RATIO) as usize;

        v1.min(v2)
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

    async fn get_online_model_info(&self) -> Result<Model> {
        let key = &self.config.api_key;
        let model = self.model_name;

        info!("[openai] model request");
        self.check_enabled();

        let resp = self
            .client
            .get(url_model(model))
            .header("Authorization", format!("Bearer {key}"))
            .send()
            .await?;

        let json_str = netutil::check_http_resp(resp).await?;

        netutil::convert_from_json::<Model>(&json_str)
    }

    fn check_enabled(&self) -> Result<()> {
        if !self.config.enabled {
            warn!("[openai] skip because openai feature is disabled");
            bail!("openai is disabled");
        }

        Ok(())
    }

    /// OpenAI Chat API を使用する。
    pub async fn chat(&self, msgs: Vec<ChatMessage>) -> Result<String> {
        let key = &self.config.api_key;
        let body = ChatRequest {
            model: self.model_name.to_string(),
            messages: msgs,
            ..Default::default()
        };

        info!("[openai] chat request: {:?}", body);
        self.check_enabled();

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
        let choice0 = resp_msg
            .choices
            .first()
            .ok_or(anyhow!("choices is empty"))?;
        let text = choice0
            .message
            .content
            .as_ref()
            .ok_or(anyhow!("message content is empty"))?
            .clone();

        Ok(text)
    }

    /// OpenAI Chat API を fcuntion call 機能付きで使用する。
    pub async fn chat_with_function(
        &self,
        msgs: &[ChatMessage],
        funcs: &[Function],
    ) -> Result<ChatMessage> {
        let key = &self.config.api_key;
        let body = ChatRequest {
            model: self.model_name.to_string(),
            messages: msgs.to_vec(),
            functions: Some(funcs.to_vec()),
            ..Default::default()
        };

        info!("[openai] chat request with function: {:?}", body);
        self.check_enabled();

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
            .first()
            .ok_or(anyhow!("choices is empty"))?
            .message;

        Ok(msg.clone())
    }

    /// OpenAI Image Generation API を使用する。
    pub async fn generate_image(&self, prompt: &str, n: u8) -> Result<Vec<String>> {
        let key = &self.config.api_key;
        let body = ImageGenRequest {
            prompt: prompt.to_string(),
            n: Some(n),
            size: Some(ImageSize::X256),
            ..Default::default()
        };

        info!(
            "[openai] image gen request: {}",
            serde_json::to_string(&body)?
        );
        self.check_enabled();

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
        info!("[openai] image gen OK: {:?}", result);

        Ok(result)
    }

    /// OpenAI Create Speech API を使用する。
    pub async fn text_to_speech(
        &self,
        model: SpeechModel,
        input: &str,
        voice: SpeechVoice,
        response_format: Option<SpeechFormat>,
        speed: Option<f32>,
    ) -> Result<Vec<u8>> {
        ensure!(
            input.len() <= SPEECH_INPUT_MAX,
            "input length limit is {SPEECH_INPUT_MAX} characters"
        );
        if let Some(speed) = speed {
            ensure!(
                (SPEECH_SPEED_MIN..=SPEECH_SPEED_MAX).contains(&speed),
                "speed must be {SPEECH_SPEED_MIN} .. {SPEECH_SPEED_MAX}"
            );
        }

        let key = &self.config.api_key;
        let body = SpeechRequest {
            model,
            input: input.to_string(),
            voice,
            response_format,
            speed,
        };

        info!(
            "[openai] create speech request: {}",
            serde_json::to_string(&body)?
        );
        self.check_enabled();

        let resp = self
            .client
            .post(URL_AUDIO_SPEECH)
            .header("Authorization", format!("Bearer {key}"))
            .json(&body)
            .send()
            .await?;

        let bin = netutil::check_http_resp_bin(resp).await?;

        Ok(bin)
    }
}

impl SystemModule for OpenAi {
    fn on_start(&mut self, _ctrl: &Control) {
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
    // cargo test assistant -- --ignored --nocapture
    async fn assistant() {
        let src = std::fs::read_to_string("config.toml").unwrap();
        let _unset = config::set(toml::from_str(&src).unwrap());

        let ai = OpenAi::new().unwrap();
        let msgs = vec![
            ChatMessage {
                role: Role::System,
                content: Some("あなたの名前は上海人形で、あなたはやっぴーさんの人形です。あなたはやっぴー家の優秀なアシスタントです。".to_string()),
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

    #[tokio::test]
    #[serial(openai)]
    #[ignore]
    // cargo test test_to_sppech -- --ignored --nocapture
    async fn test_to_sppech() -> Result<()> {
        let src = std::fs::read_to_string("config.toml").unwrap();
        let _unset = config::set(toml::from_str(&src).unwrap());

        let ai = OpenAi::new().unwrap();
        let res = ai
            .text_to_speech(
                SpeechModel::Tts1,
                "こんにちは、かんりにんぎょうです。",
                SpeechVoice::Nova,
                Some(SpeechFormat::Mp3),
                Some(1.0),
            )
            .await?;

        assert!(!res.is_empty());
        let size = res.len();
        const PATH: &str = "speech.mp3";
        std::fs::write(PATH, res)?;
        println!("Wrote to: {PATH} ({} bytes)", size);

        Ok(())
    }
}
