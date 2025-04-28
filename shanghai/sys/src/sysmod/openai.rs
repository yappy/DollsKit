//! OpenAI API.

mod basicfuncs;
pub mod chat_history;
pub mod function;

use std::collections::HashMap;
use std::io::Cursor;
use std::sync::LazyLock;
use std::time::{Duration, Instant, SystemTime};

use super::SystemModule;
use crate::config;
use crate::taskserver::Control;
use base64::{Engine, engine::general_purpose};
use utils::netutil::{self, HttpStatusError};

use anyhow::{Context, ensure};
use anyhow::{Result, anyhow, bail};
use chrono::TimeZone;
use log::warn;
use log::{debug, info};
use reqwest::Response;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

/// HTTP 通信のタイムアウト。
/// これを設定しないと無限待ちになる危険性がある。
const CONN_TIMEOUT: Duration = Duration::from_secs(10);
/// AI 応答待ちのタイムアウト。
const TIMEOUT: Duration = Duration::from_secs(60);
/// モデル情報更新間隔。
/// 24 時間に一度更新する。
const MODEL_INFO_UPDATE_INTERVAL: Duration = Duration::from_secs(24 * 3600);

/// <https://platform.openai.com/docs/api-reference/models/retrieve>
fn url_model(model: &str) -> String {
    format!("https://api.openai.com/v1/models/{model}")
}
const URL_RESPONSE: &str = "https://api.openai.com/v1/responses";
const URL_IMAGE_GEN: &str = "https://api.openai.com/v1/images/generations";
const URL_AUDIO_SPEECH: &str = "https://api.openai.com/v1/audio/speech";

/// [OfflineModelInfo] + [OnlineModelInfo]
#[derive(Debug, Clone, Serialize)]
pub struct ModelInfo {
    #[serde(flatten)]
    offline: OfflineModelInfo,
    #[serde(flatten)]
    online: OnlineModelInfo,
}

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

/// [MODEL_LIST] からモデル名で [OfflineModelInfo] を検索する。
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

/// モデル情報。
/// API から得られるデータ。時々でよいので再取得する必要がある。
#[derive(Debug, Clone, Serialize)]
pub struct CachedModelInfo {
    last_update: SystemTime,
    info: OnlineModelInfo,
}

/// OpenAI API JSON 定義。
/// モデル情報。
#[derive(Default, Clone, Debug, Serialize, Deserialize)]
struct Model {
    /// The model identifier, which can be referenced in the API endpoints.
    id: String,
    /// The Unix timestamp (in seconds) when the model was created.
    created: u64,
    /// The object type, which is always "model".
    object: String,
    ///The organization that owns the model.
    owned_by: String,
}

/// [Model] から必要なもののみ抜き出して読めるデータに変換したもの。
#[derive(Default, Clone, Debug, Serialize)]
pub struct OnlineModelInfo {
    created: String,
}

impl OnlineModelInfo {
    fn from(model: Model) -> Self {
        let dt_str = chrono::Local
            .timestamp_opt(model.created as i64, 0)
            .single()
            .map_or_else(|| "?".into(), |dt| dt.to_rfc3339());

        Self { created: dt_str }
    }
}

/// Rate Limit 情報。
///
/// HTTP レスポンスヘッダに含まれる。
#[derive(Debug, Clone, Copy)]
struct RateLimit {
    timestamp: Instant,
    limit_requests: u32,
    limit_tokens: u32,
    remaining_requests: u32,
    remaining_tokens: u32,
    reset_requests: Duration,
    reset_tokens: Duration,
}

#[derive(Debug, Clone, Copy)]
pub struct ExpectedRateLimit {
    pub limit_requests: u32,
    pub limit_tokens: u32,
    pub remaining_requests: u32,
    pub remaining_tokens: u32,
}

impl RateLimit {
    fn from(resp: &reqwest::Response) -> Result<Self> {
        let timestamp = Instant::now();
        let headers = resp.headers();

        let to_u32 = |key| -> Result<u32> {
            let s = headers
                .get(key)
                .ok_or_else(|| anyhow!("not found: {key}"))?
                .to_str()?;

            s.parse::<u32>()
                .with_context(|| format!("parse u32 failed: {s}"))
        };
        let to_secs_f64 = |key| -> Result<f64> {
            let s = headers
                .get(key)
                .ok_or_else(|| anyhow!("not found: {key}"))?
                .to_str()?;

            Self::to_secs_f64(s).with_context(|| format!("parse f64 secs failed: {s}"))
        };

        let limit_requests = to_u32("x-ratelimit-limit-requests")?;
        let limit_tokens = to_u32("x-ratelimit-limit-tokens")?;
        let remaining_requests = to_u32("x-ratelimit-remaining-requests")?;
        let remaining_tokens = to_u32("x-ratelimit-remaining-tokens")?;
        let reset_requests = to_secs_f64("x-ratelimit-reset-requests")?;
        let reset_tokens = to_secs_f64("x-ratelimit-reset-tokens")?;

        Ok(Self {
            timestamp,
            limit_requests,
            limit_tokens,
            remaining_requests,
            remaining_tokens,
            reset_requests: Duration::from_secs_f64(reset_requests),
            reset_tokens: Duration::from_secs_f64(reset_tokens),
        })
    }

    /// サンプルには "6m0s" と書かれているが、
    /// 実際には "30.828s" のように小数が来ている。
    /// また、"120ms" とかもある。
    fn to_secs_f64(s: &str) -> Result<f64> {
        let mut sum = 0.0;

        let unit_to_scale = |unit: &str| -> Result<f64> {
            let scale = match unit {
                "ns" => 0.000_000_001,
                "us" => 0.000_001,
                "ms" => 0.001,
                "s" => 1.0,
                "m" => 60.0,
                "h" => 3600.0,
                "d" => 86400.0,
                _ => bail!("unknown unit: {unit}"),
            };
            Ok(scale)
        };

        let mut numbuf = String::new();
        let mut unitbuf = String::new();
        for c in s.chars() {
            match c {
                '0'..='9' | '.' => {
                    if !unitbuf.is_empty() {
                        let num = numbuf.parse::<f64>()?;
                        let scale = unit_to_scale(&unitbuf)?;
                        sum += num * scale;
                        numbuf.clear();
                        unitbuf.clear();
                    }
                    numbuf.push(c);
                }
                _ => {
                    unitbuf.push(c);
                }
            };
        }
        if !unitbuf.is_empty() {
            let num = numbuf.parse::<f64>()?;
            let scale = unit_to_scale(&unitbuf)?;
            sum += num * scale;
            numbuf.clear();
            unitbuf.clear();
        }
        ensure!(numbuf.is_empty(), "unexpected format: {}", s);

        Ok(sum)
    }

    /// 情報のタイムスタンプと現在時刻から、現在の推測値を計算する。
    fn calc_expected_current(&self) -> ExpectedRateLimit {
        let now = Instant::now();
        let elapsed_secs = (now - self.timestamp).as_secs_f64();
        debug!("{self:?}");
        debug!("elapsed_secs = {elapsed_secs}");

        let remaining_requests = if elapsed_secs >= self.reset_requests.as_secs_f64() {
            self.limit_requests
        } else {
            let vreq = (self.limit_requests - self.remaining_requests) as f64
                / self.reset_requests.as_secs_f64();
            let remaining_requests = self.remaining_requests as f64 + vreq * elapsed_secs;

            (remaining_requests as u32).min(self.limit_requests)
        };

        let remaining_tokens = if elapsed_secs >= self.reset_tokens.as_secs_f64() {
            self.limit_tokens
        } else {
            let vtok = (self.limit_tokens - self.remaining_tokens) as f64
                / self.reset_tokens.as_secs_f64();
            let remaining_tokens = self.remaining_tokens as f64 + vtok * elapsed_secs;

            (remaining_tokens as u32).min(self.limit_tokens)
        };

        ExpectedRateLimit {
            limit_requests: self.limit_requests,
            limit_tokens: self.limit_tokens,
            remaining_requests,
            remaining_tokens,
        }
    }
}

/// OpenAI API JSON 定義。
/// 入力エレメント。
///
/// <https://platform.openai.com/docs/api-reference/responses/create>
#[derive(Clone, Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum InputItem {
    /// Input Message or Output Message.
    Message {
        /// Input か Output かは role で決まる。
        ///
        /// * Input : Role = user | developer
        /// * Output: Role = assistant
        ///
        /// Output では id, status, type が Required になっているが多分嘘。
        role: Role,
        /// string | array
        /// image やファイルも含める場合はオブジェクト配列にする必要がある。
        content: Vec<InputContent>,
    },
    /// The results of a web search tool call.
    WebSearchCall(WebSearchCall),
    /// A tool call to run a function.
    FunctionCall {
        /// The unique ID of the function tool call generated by the model.
        call_id: String,
        /// The name of the function to run.
        name: String,
        /// A JSON string of the arguments to pass to the function.
        arguments: String,
    },
    /// The output of a function tool call.
    FunctionCallOutput { call_id: String, output: String },
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum InputContent {
    InputText {
        /// The text input to the model.
        text: String,
    },
    InputImage {
        image_url: String,
        /// The detail level of the image to be sent to the model.
        /// One of high, low, or auto. Defaults to auto.
        detail: InputImageDetail,
    },
}

#[derive(Clone, Default, Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum InputImageDetail {
    #[default]
    Auto,
    High,
    Low,
}

/// OpenAI API JSON 定義。
/// 入力/出力テキストのロール。
#[derive(Default, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    #[default]
    Developer,
    User,
    Assistant,
}

#[skip_serializing_none]
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Tool {
    /// This tool searches the web for relevant results to use in a response.
    /// Learn more about the web search tool.
    WebSearchPreview {
        /// High level guidance for the amount of context window space to use
        /// for the search. One of low, medium, or high. medium is the default.
        search_context_size: Option<SearchContextSize>,
        /// Approximate location parameters for the search.
        user_location: Option<UserLocation>,
    },
    /// A tool that searches for relevant content from uploaded files.
    /// Learn more about the file search tool.
    FileSearch {
        vector_store_ids: Vec<String>,
    },

    Function(Function),
}

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SearchContextSize {
    Low,
    #[default]
    Medium,
    High,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum UserLocation {
    Approximate {
        /// Free text input for the city of the user, e.g. San Francisco.
        #[serde(skip_serializing_if = "Option::is_none")]
        city: Option<String>,
        /// The two-letter ISO country code of the user, e.g. US.
        #[serde(skip_serializing_if = "Option::is_none")]
        country: Option<String>,
        /// Free text input for the region of the user, e.g. California.
        #[serde(skip_serializing_if = "Option::is_none")]
        region: Option<String>,
        /// The IANA timezone of the user, e.g. America/Los_Angeles.
        #[serde(skip_serializing_if = "Option::is_none")]
        timezone: Option<String>,
    },
}

impl Default for UserLocation {
    fn default() -> Self {
        Self::Approximate {
            city: None,
            country: Some("JP".to_string()),
            region: None,
            timezone: Some("Asia/Tokyo".to_string()),
        }
    }
}

/// OpenAI API JSON 定義。
/// function 定義。
#[skip_serializing_none]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Function {
    /// The name of the function to call.
    pub name: String,
    /// A description of the function.
    /// Used by the model to determine whether or not to call the function.
    pub description: Option<String>,
    /// A JSON schema object describing the parameters of the function.
    pub parameters: Parameters,
    // Whether to enforce strict parameter validation. Default true.
    pub strict: bool,
}

impl Default for Function {
    /// フィールドの説明にはデフォルト true とあるが、
    /// function call の例ではデフォルト false かのように書かれているので
    /// 明示的に true にしておく。
    fn default() -> Self {
        Self {
            name: Default::default(),
            description: Default::default(),
            parameters: Default::default(),
            strict: true,
        }
    }
}

/// OpenAI API JSON 定義。
/// function パラメータ定義 (JSON Schema)。
///
/// <https://json-schema.org/understanding-json-schema>
/// <https://platform.openai.com/docs/guides/structured-outputs?api-mode=responses#supported-schemas>
#[skip_serializing_none]
#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct ParameterElement {
    #[serde(rename = "type")]
    pub type_: Vec<ParameterType>,
    pub description: Option<String>,
    #[serde(rename = "enum")]
    pub enum_: Option<Vec<String>>,
    // Not supported on strict mode
    //pub minumum: Option<i64>,
    //pub maximum: Option<i64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ParameterType {
    Null,
    Boolean,
    Integer,
    Number,
    String,
}

/// OpenAI API JSON 定義。
/// function パラメータ定義。
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Parameters {
    /// "object"
    #[serde(rename = "type")]
    pub type_: String,
    pub properties: HashMap<String, ParameterElement>,
    pub required: Vec<String>,
    #[serde(rename = "additionalProperties")]
    pub additional_properties: bool,
}

impl Default for Parameters {
    fn default() -> Self {
        Self {
            type_: "object".to_string(),
            properties: Default::default(),
            required: Default::default(),
            additional_properties: false,
        }
    }
}

/// OpenAI API JSON 定義。
/// Response API リクエスト。
#[skip_serializing_none]
#[derive(Default, Clone, Debug, Serialize)]
pub struct ResponseRequest {
    /// Model ID used to generate the response, like gpt-4o or o1.
    /// OpenAI offers a wide range of models with different capabilities,
    /// performance characteristics, and price points.
    /// Refer to the model guide to browse and compare available models.
    model: String,

    /// Inserts a system (or developer) message as the first item
    /// in the model's context.
    /// When using along with previous_response_id,
    /// the instructions from a previous response will not be
    /// carried over to the next response.
    /// This makes it simple to swap out system (or developer) messages
    /// in new responses.
    instructions: Option<String>,

    /// Text, image, or file inputs to the model, used to generate a response.
    /// (string or Array)
    input: Vec<InputItem>,

    /// An array of tools the model may call while generating a response.
    /// You can specify which tool to use by setting the tool_choice parameter.
    ///
    /// The two categories of tools you can provide the model are:
    /// * Built-in tools
    ///   * Tools that are provided by OpenAI that extend
    ///     the model's capabilities, like web search or file search.
    ///     Learn more about built-in tools.
    /// * Function calls (custom tools)
    ///   * Functions that are defined by you,
    ///     enabling the model to call your own code.
    ///     Learn more about function calling.
    tools: Option<Vec<Tool>>,

    /// Specify additional output data to include in the model response.
    /// Currently supported values are:
    /// * file_search_call.results:
    ///   * Include the search results of the file search tool call.
    /// * message.input_image.image_url:
    ///   * Include image urls from the input message.
    /// * computer_call_output.output.image_url:
    ///   *Include image urls from the computer call output.
    include: Option<Vec<String>>,

    /// An upper bound for the number of tokens that can be generated
    /// for a response, including visible output tokens and reasoning tokens.
    max_output_tokens: Option<u64>,

    /// The unique ID of the previous response to the model.
    /// Use this to create multi-turn conversations.
    /// Learn more about conversation state.
    previous_response_id: Option<String>,

    /// What sampling temperature to use, between 0 and 2.
    /// Higher values like 0.8 will make the output more random,
    /// while lower values like 0.2 will make it more focused and deterministic.
    /// We generally recommend altering this or top_p but not both.
    /// (Defaults to 1)
    temperature: Option<f32>,

    /// An alternative to sampling with temperature, called nucleus sampling,
    /// where the model considers the results of the tokens
    /// with top_p probability mass.
    /// So 0.1 means only the tokens comprising
    /// the top 10% probability mass are considered.
    /// We generally recommend altering this or temperature but not both.
    /// (Defaults to 1)
    top_p: Option<f32>,

    /// A unique identifier representing your end-user,
    /// which can help OpenAI to monitor and detect abuse.
    user: Option<String>,
}

#[allow(dead_code)]
#[derive(Clone, Debug, Deserialize)]
pub struct ResponseObject {
    id: String,
    created_at: u64,
    error: Option<ErrorObject>,
    instructions: Option<String>,
    max_output_tokens: Option<u64>,
    model: String,
    output: Vec<OutputElement>,
    previous_response_id: Option<String>,
    usage: Usage,
    user: Option<String>,
}

impl ResponseObject {
    /// OpenAI SDK 互換。
    /// [Self::output] の出力リストのうち、文字列であるものを連結して返す。
    ///
    /// 存在しない場合は空文字列になる。
    pub fn output_text(&self) -> String {
        let mut buf = String::new();
        for elem in self.output.iter() {
            if let OutputElement::Message { content, .. } = elem {
                for cont in content.iter() {
                    if let OutputContent::OutputText { text } = cont {
                        buf.push_str(text);
                    }
                }
            }
        }

        buf
    }

    /// [Self::output] の出力リストのうち、FunctionCall であるもののみを走査する。
    pub fn web_search_iter(&self) -> impl Iterator<Item = &WebSearchCall> {
        self.output.iter().filter_map(|elem| match elem {
            OutputElement::WebSearchCall(wsc) => Some(wsc),
            _ => None,
        })
    }

    /// [Self::output] の出力リストのうち、FunctionCall であるもののみを走査する。
    pub fn func_call_iter(&self) -> impl Iterator<Item = &FunctionCall> {
        self.output.iter().filter_map(|elem| match elem {
            OutputElement::FunctionCall(fc) => Some(fc),
            _ => None,
        })
    }
}

#[allow(dead_code)]
#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum OutputElement {
    /// An output message from the model.
    Message {
        /// The unique ID of the output message.
        id: String,
        /// The role of the output message. Always assistant.
        role: Role,
        /// The content of the output message.
        content: Vec<OutputContent>,
    },
    FunctionCall(FunctionCall),
    /// The results of a web search tool call.
    /// See the web search guide for more information.
    WebSearchCall(WebSearchCall),
}

#[allow(dead_code)]
#[derive(Clone, Debug, Deserialize)]
pub struct FunctionCall {
    pub id: String,
    pub call_id: String,
    pub name: String,
    pub arguments: String,
    pub status: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WebSearchCall {
    /// The unique ID of the web search tool call.
    pub id: String,
    /// The status of the web search tool call.
    pub status: String,
}

#[allow(dead_code)]
#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum OutputContent {
    /// A text output from the model.
    OutputText {
        /// The text output from the model.
        text: String,
        // The annotations of the text output.
        // annotations: Vec<()>,
    },
    /// A refusal from the model.
    Refusal {
        /// The refusal explanationfrom the model.
        refusal: String,
    },
}

#[allow(dead_code)]
#[derive(Default, Clone, Debug, Deserialize)]
struct ErrorObject {
    /// The error code for the response.
    code: String,
    ///A human-readable description of the error.
    message: String,
}

#[allow(dead_code)]
#[derive(Default, Clone, Debug, Deserialize)]
struct Usage {
    input_tokens: u32,
    input_tokens_details: InputTokensDetails,
    output_tokens: u32,
    output_tokens_details: OutputTokensDetails,
    total_tokens: u32,
}

#[allow(dead_code)]
#[derive(Default, Clone, Debug, Deserialize)]
struct InputTokensDetails {
    cached_tokens: u32,
}

#[allow(dead_code)]
#[derive(Default, Clone, Debug, Deserialize)]
struct OutputTokensDetails {
    reasoning_tokens: u32,
}

/// OpenAI API JSON 定義。
/// 画像生成リクエスト。
///
/// <https://platform.openai.com/docs/api-reference/images>
#[skip_serializing_none]
#[derive(Default, Clone, Debug, Serialize, Deserialize)]
struct ImageGenRequest {
    /// A text description of the desired image(s).
    /// The maximum length is 1000 characters.
    prompt: String,
    /// The number of images to generate. Must be between 1 and 10.
    /// Defaults to 1
    n: Option<u8>,
    /// The format in which the generated images are returned.
    /// Must be one of url or b64_json.
    /// Defaults to url
    response_format: Option<String>,
    /// The size of the generated images. Must be one of 256x256, 512x512, or 1024x1024.
    /// Defaults to 1024x1024
    size: Option<ImageSize>,
    /// A unique identifier representing your end-user,
    /// which can help OpenAI to monitor and detect abuse. Learn more.
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
#[skip_serializing_none]
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
    response_format: Option<SpeechFormat>,
    /// The speed of the generated audio.
    /// Select a value from 0.25 to 4.0. 1.0 is the default.
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
    model_info_online: Option<CachedModelInfo>,

    rate_limit: Option<RateLimit>,
}

/// 特別な案内をすべきかもしれないエラー。
///
/// <https://platform.openai.com/docs/guides/error-codes>
pub enum OpenAiErrorKind {
    /// その他。
    Fatal,
    /// HTTP リクエストがタイムアウト。
    Timeout,
    /// 429: 短時間に使いすぎ。
    RateLimit,
    /// 429: 課金が必要。
    QuotaExceeded,
    /// その他の HTTP Error
    HttpError(u16),
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
            rate_limit: None,
        })
    }

    /*
    pub fn model_name(&self) -> &str {
        self.model_name
    }
    */

    pub async fn model_info(&mut self) -> Result<ModelInfo> {
        let offline = self.model_info_offline();
        let online = self.model_info_online().await?;

        Ok(ModelInfo { offline, online })
    }

    pub fn model_info_offline(&self) -> OfflineModelInfo {
        self.model_info_offline
    }

    pub async fn model_info_online(&mut self) -> Result<OnlineModelInfo> {
        let cur = &self.model_info_online;
        let update = if let Some(info) = cur {
            let now = SystemTime::now();
            let elapsed = now.duration_since(info.last_update).unwrap_or_default();

            elapsed > MODEL_INFO_UPDATE_INTERVAL
        } else {
            true
        };

        if update {
            info!("[openai] update model info");
            let info = self.get_online_model_info().await?;
            let info = OnlineModelInfo::from(info);
            let newval = CachedModelInfo {
                last_update: SystemTime::now(),
                info: info.clone(),
            };
            let _ = self.model_info_online.insert(newval);

            Ok(info)
        } else {
            info!("[openai] skip to update model info");
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

    async fn get_online_model_info(&self) -> Result<Model> {
        let key = &self.config.api_key;
        let model = self.model_name;

        info!("[openai] model request");
        self.check_enabled()?;

        let resp = self
            .client
            .get(url_model(model))
            .header("Authorization", format!("Bearer {key}"))
            .send()
            .await?;

        let json_str = netutil::check_http_resp(resp).await?;

        netutil::convert_from_json::<Model>(&json_str)
    }

    /// 設定で無効になっていたら警告をログに出しつつ [Err] を返す。
    fn check_enabled(&self) -> Result<()> {
        if !self.config.enabled {
            warn!("[openai] skip because openai feature is disabled");
            bail!("openai is disabled");
        }

        Ok(())
    }

    /// エラーチェインの中から特定のエラーを探す。
    pub fn error_kind(err: &anyhow::Error) -> OpenAiErrorKind {
        for cause in err.chain() {
            if let Some(req_err) = cause.downcast_ref::<reqwest::Error>() {
                if req_err.is_timeout() {
                    return OpenAiErrorKind::Timeout;
                }
            }
            if let Some(http_err) = cause.downcast_ref::<HttpStatusError>() {
                // 429: Too Many Requests
                if http_err.status == 429 {
                    let msg = http_err.body.to_ascii_lowercase();
                    if msg.contains("rate") && msg.contains("limit") {
                        return OpenAiErrorKind::RateLimit;
                    } else if msg.contains("quota") && msg.contains("billing") {
                        return OpenAiErrorKind::QuotaExceeded;
                    }
                } else {
                    return OpenAiErrorKind::HttpError(http_err.status);
                }
            }
        }

        OpenAiErrorKind::Fatal
    }

    fn log_header(resp: &reqwest::Response, key: &str) {
        if let Some(value) = resp.headers().get(key) {
            info!("[openai] {key}: {value:?}");
        } else {
            info!("[openai] not found: {key}");
        }
    }

    /// JSON を POST して [Response] を返す。
    /// 成功しても HTTP ステータスコードは失敗かもしれない。
    ///
    /// HTTP Header に付与されるメタ情報をログに記録する。
    /// レートリミット情報は後で参照できるように保存する。
    ///
    /// <https://platform.openai.com/docs/api-reference/debugging-requests>
    async fn post_json(
        &mut self,
        url: &str,
        body: &(impl Serialize + std::fmt::Debug),
    ) -> Result<Response> {
        let key = &self.config.api_key;

        info!("[openai] post_json: {url}");
        info!("[openai] {}", serde_json::to_string(body).unwrap());
        self.check_enabled()?;

        let resp = self
            .client
            .post(url)
            .header("Authorization", format!("Bearer {key}"))
            .json(body)
            .send()
            .await?;
        // HTTP POST レスポンス取得まで成功
        // ステータスコードは失敗かもしれない

        // API メタ情報および Rate Limit 情報をログに記録
        Self::log_header(&resp, "x-request-id");
        Self::log_header(&resp, "openai-organization");
        Self::log_header(&resp, "openai-processing-ms");
        Self::log_header(&resp, "openai-version");
        // ドキュメントにはないが、公式ライブラリが使っている
        // これに従うのがおすすめなのかもしれない
        Self::log_header(&resp, "x-should-retry");

        // レートリミット情報を最新に更新
        // 失敗しても警告のみ
        match RateLimit::from(&resp) {
            Ok(rate_limit) => {
                info!("[openai] rate limit: {:?}", rate_limit);
                self.rate_limit = Some(rate_limit);
            }
            Err(err) => {
                warn!("[openai] could not get rate limit: {err:#}");
            }
        }

        Ok(resp)
    }

    /// [Self::post_json] の結果を文字列として返す。
    /// HTTP エラーも含めてエラーにする。
    async fn post_json_text(
        &mut self,
        url: &str,
        body: &(impl Serialize + std::fmt::Debug),
    ) -> Result<String> {
        let resp = self.post_json(url, body).await?;
        let text = netutil::check_http_resp(resp).await?;
        info!("{text}");

        Ok(text)
    }

    /// [Self::post_json] の結果をバイナリとして返す。
    /// HTTP エラーも含めてエラーにする。
    async fn post_json_bin(
        &mut self,
        url: &str,
        body: &(impl Serialize + std::fmt::Debug),
    ) -> Result<Vec<u8>> {
        let resp = self.post_json(url, body).await?;
        let bin = netutil::check_http_resp_bin(resp).await?;
        info!("[openai] binary received: size={}", bin.len());

        Ok(bin)
    }

    pub fn get_expected_rate_limit(&self) -> Option<ExpectedRateLimit> {
        self.rate_limit
            .as_ref()
            .map(|rate_limit| rate_limit.calc_expected_current())
    }

    /// OpenAI Reponse API を使用する。
    pub async fn chat(
        &mut self,
        instructions: Option<&str>,
        input: Vec<InputItem>,
    ) -> Result<ResponseObject> {
        self.chat_with_tools(instructions, input, &[]).await
    }

    /// OpenAI Reponse API を使用する。
    pub async fn chat_with_tools(
        &mut self,
        instructions: Option<&str>,
        input: Vec<InputItem>,
        tools: &[Tool],
    ) -> Result<ResponseObject> {
        info!("[openai] chat request");

        let instructions = instructions.map(|s| s.to_string());

        let body = ResponseRequest {
            model: self.model_name.to_string(),
            instructions,
            input,
            tools: Some(tools.to_vec()),
            ..Default::default()
        };

        let json_str = self.post_json_text(URL_RESPONSE, &body).await?;
        let resp: ResponseObject = netutil::convert_from_json(&json_str)?;

        Ok(resp)
    }

    /// OpenAI Image Input に適した形式に変換する。
    ///
    /// <https://platform.openai.com/docs/guides/images-vision?api-mode=responses#image-input-requirements>
    ///
    /// Supported file types
    /// * PNG (.png)
    /// * JPEG (.jpeg and .jpg)
    /// * WEBP (.webp)
    /// * Non-animated GIF (.gif)
    ///
    /// Size limits
    /// * Up to 20MB per image
    /// * Low-resolution: 512px x 512px
    /// * High-resolution: 768px (short side) x 2000px (long side)
    ///
    /// image/png;base64 文字列として保持する。
    /// detail=Low だと OpenAI 側で 512x512 まで縮小されるらしいが、
    /// こちらのメモリ消費と送信時のネットワーク帯域が無駄なので
    /// ここで 512 まで縮小してからエンコードする。
    pub fn to_image_input(bin: &[u8]) -> Result<InputContent> {
        const SIZE_LIMIT: u32 = 512;

        let mut img: image::DynamicImage =
            image::load_from_memory(bin).context("Load image error")?;
        // 縦か横が制限を超えている場合はアスペクト比を保ちながらリサイズする
        if img.width() > SIZE_LIMIT || img.height() > SIZE_LIMIT {
            img = img.resize(SIZE_LIMIT, SIZE_LIMIT, image::imageops::FilterType::Nearest);
        }

        // png 形式としてメモリに書き出し
        let mut output = Cursor::new(vec![]);
        img.write_to(&mut output, image::ImageFormat::Png)
            .context("Convert image error")?;
        let dst = output.into_inner();

        // base64 エンコードして json object に変換
        let base64 = general_purpose::STANDARD.encode(&dst);
        let image_url = format!("data:image/png;base64,{base64}");
        let input = InputContent::InputImage {
            image_url,
            detail: InputImageDetail::Low,
        };

        Ok(input)
    }

    /// OpenAI Image Generation API を使用する。
    pub async fn generate_image(&mut self, prompt: &str, n: u8) -> Result<Vec<String>> {
        info!("[openai] image gen request");

        let body = ImageGenRequest {
            prompt: prompt.to_string(),
            n: Some(n),
            size: Some(ImageSize::X256),
            ..Default::default()
        };

        let json_str = self.post_json_text(URL_IMAGE_GEN, &body).await?;
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
        &mut self,
        model: SpeechModel,
        input: &str,
        voice: SpeechVoice,
        response_format: Option<SpeechFormat>,
        speed: Option<f32>,
    ) -> Result<Vec<u8>> {
        info!("[openai] create speech request");

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

        let body = SpeechRequest {
            model,
            input: input.to_string(),
            voice,
            response_format,
            speed,
        };

        let bin = self.post_json_bin(URL_AUDIO_SPEECH, &body).await?;

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
    use serial_test::serial;
    use utils::netutil::HttpStatusError;

    #[test]
    fn test_parse_resettime() {
        const EPS: f64 = 1e-10;

        let s = "6m0s";
        let v = RateLimit::to_secs_f64(s).unwrap();
        assert_eq!(360.0, v);

        let s = "30.828s";
        let v = RateLimit::to_secs_f64(s).unwrap();
        assert!((30.828 - v).abs() < EPS);

        let s = "1h2m3s";
        let v = RateLimit::to_secs_f64(s).unwrap();
        assert_eq!((3600 + 120 + 3) as f64, v);

        let s = "120ms";
        let v = RateLimit::to_secs_f64(s).unwrap();
        assert!((0.120 - v).abs() < EPS);
    }

    #[tokio::test]
    #[serial(openai)]
    #[ignore]
    // cargo test simple_assistant -- --ignored --nocapture
    async fn simple_assistant() {
        let src = std::fs::read_to_string("../config.toml").unwrap();
        let _unset = config::set(toml::from_str(&src).unwrap());

        let mut ai = OpenAi::new().unwrap();
        let inst = concat!(
            "あなたの名前は上海人形で、あなたはやっぴーさんの人形です。あなたはやっぴー家の優秀なアシスタントです。",
            "やっぴーさんは男性で、ホワイト企業に勤めています。yappyという名前で呼ばれることもあります。"
        );
        let input = vec![InputItem::Message {
            role: Role::User,
            content: vec![InputContent::InputText {
                text: "こんにちは。あなたの知っている情報を教えてください。".to_string(),
            }],
        }];
        match ai.chat(Some(inst), input).await {
            Ok(resp) => {
                println!("{resp:?}");
                println!("{}", resp.output_text());
            }
            Err(err) => {
                // HTTP status が得られるタイプのエラーのみ許容する
                let err = err.downcast_ref::<HttpStatusError>().unwrap();
                println!("{err:#?}");
            }
        };
    }

    #[tokio::test]
    #[serial(openai)]
    #[ignore]
    // cargo test web_search -- --ignored --nocapture
    async fn web_search() {
        let src = std::fs::read_to_string("../config.toml").unwrap();
        let _unset = config::set(toml::from_str(&src).unwrap());

        let mut ai = OpenAi::new().unwrap();
        let input = vec![InputItem::Message {
            role: Role::User,
            content: vec![InputContent::InputText {
                text: "今日の最新ニュースを教えてください。1つだけでいいです。".to_string(),
            }],
        }];
        let tools = [Tool::WebSearchPreview {
            search_context_size: Some(SearchContextSize::Low),
            user_location: Some(UserLocation::Approximate {
                city: None,
                country: Some("JP".to_string()),
                region: None,
                timezone: Some("Asia/Tokyo".to_string()),
            }),
        }];
        println!("{}", serde_json::to_string(&tools).unwrap());
        match ai.chat_with_tools(None, input, &tools).await {
            Ok(resp) => {
                println!("{resp:?}");
                println!("{}", resp.output_text());
            }
            Err(err) => {
                // HTTP status が得られるタイプのエラーのみ許容する
                let err = err.downcast_ref::<HttpStatusError>().unwrap();
                println!("{err:#?}");
            }
        };
    }

    #[tokio::test]
    #[serial(openai)]
    #[ignore]
    // cargo test image_gen -- --ignored --nocapture
    async fn image_gen() -> Result<()> {
        let src = std::fs::read_to_string("../config.toml").unwrap();
        let _unset = config::set(toml::from_str(&src).unwrap());

        let mut ai = OpenAi::new().unwrap();
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
        let src = std::fs::read_to_string("../config.toml").unwrap();
        let _unset = config::set(toml::from_str(&src).unwrap());

        let mut ai = OpenAi::new().unwrap();
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
