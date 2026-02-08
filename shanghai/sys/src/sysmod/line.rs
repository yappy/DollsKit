//! LINE API。

#![allow(clippy::identity_op)]

use super::SystemModule;
use super::openai::{InputContent, ParameterType};
use super::openai::{
    ParameterElement,
    chat_history::ChatHistory,
    function::{self, BasicContext, FuncArgs, FunctionTable},
};
use crate::config;
use crate::sysmod::openai::{Function, Parameters, function::FUNCTION_TOKEN};
use crate::sysmod::openai::{OpenAi, OpenAiErrorKind, Role, SearchContextSize, Tool, UserLocation};
use crate::taskserver::{self, Control};

use actix_web::http::header::ContentType;
use anyhow::{Context, Result, anyhow, bail, ensure};
use base64::{Engine, engine::general_purpose};
use log::{error, info, warn};
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use std::vec;
use std::{collections::HashMap, fmt::Debug, sync::Arc, time::Instant};
use utils::netutil;

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
    /// 特権ユーザ LINE ID リスト。
    pub privileged_user_ids: Vec<String>,
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

/// OpenAI function 呼び出し時のコンテキスト情報。
pub struct FunctionContext {
    /// userId, groupIdm or roomId
    pub reply_to: String,
    /// 特権ユーザの発言なら true
    pub privileged: bool,
}

/// LINE システムモジュール。
///
/// [Option] は遅延初期化。
pub struct Line {
    /// 設定データ。
    pub config: LineConfig,
    /// HTTP クライアント。
    client: reqwest::Client,
    /// タスクキューの送信口。
    tx: tokio::sync::mpsc::Sender<WebhookEvent>,
    /// タスクキューの受信口。
    /// コンストラクタで生成、設定されるが、
    /// タスクスレッド起動時にその中へ move される。
    rx: Option<tokio::sync::mpsc::Receiver<WebhookEvent>>,

    /// 受信した画像リストを display_name をキーとしてバッファする。
    pub image_buffer: HashMap<String, Vec<InputContent>>,
    /// ai コマンドの会話履歴。
    pub chat_history: Option<ChatHistory>,
    /// [Self::image_buffer] [Self::chat_history] の有効期限。
    pub history_timeout: Option<Instant>,
    /// OpenAI function 機能テーブル
    pub func_table: Option<FunctionTable<FunctionContext>>,
}

impl Line {
    const TASKQUEUE_SIZE: usize = 8;

    /// コンストラクタ。
    pub fn new() -> Result<Self> {
        info!("[line] initialize");
        let config = config::get(|cfg| cfg.line.clone());
        let client = Client::builder().timeout(TIMEOUT).build()?;
        let (tx, rx) = tokio::sync::mpsc::channel(Self::TASKQUEUE_SIZE);

        Ok(Self {
            config,
            client,
            tx,
            rx: Some(rx),
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
        register_camera(&mut func_table);
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
}

/// WebHook Request 処理タスク。
async fn line_worker(
    ctrl: Control,
    mut rx: tokio::sync::mpsc::Receiver<WebhookEvent>,
) -> Result<()> {
    loop {
        tokio::select! {
        _ = ctrl.wait_cancel_rx() => {
            info!("[line-worker] cancel");
            break;
        }
        ev = rx.recv() => {
            if let Some(ev) = ev {
                    info!("[line-worker] process webhook event start");
                    if let Err(why) = process_webhook_event(&ctrl, ev).await {
                        error!("[line-worker] process webhook event error: {why:#?}");
                    } else {
                        info!("[line-worker] process webhook event ok");
                    }
                } else {
                    info!("[line-worker] sender closed");
                    break;
                }
            }
        }
    }

    let rest = rx.len();
    if rest > 0 {
        warn!("[line-worker] {} remaining task(s) will be dropped", rest);
    }
    // drop rx (残りタスクがあっても捨てる)

    Ok(())
}

async fn process_webhook_event(ctrl: &Control, ev: WebhookEvent) -> Result<()> {
    match &ev.body {
        WebhookEventBody::Message {
            reply_token,
            message,
        } => match message {
            RecvMessage::Text { text, .. } => {
                info!("[line-worker] Receive text message: {text}");
                on_text_message(ctrl, &ev.common.source, reply_token, text).await
            }
            RecvMessage::Image {
                id,
                content_provider,
                ..
            } => {
                info!("[line-worker] Receive image message");
                on_image_message(ctrl, &ev.common.source, id, reply_token, content_provider).await
            }
            other => {
                info!("[line-worker] Ignore message type: {other:?}");
                Ok(())
            }
        },
        other => {
            info!("[line-worker] Ignore event: {other:?}");
            Ok(())
        }
    }
}

impl SystemModule for Line {
    fn on_start(&mut self, ctrl: &Control) {
        info!("[line] on_start");

        let rx = self.rx.take().unwrap();
        let ctrl_for_worker = ctrl.clone();
        taskserver::spawn_oneshot_fn(ctrl, "line-worker", async move {
            line_worker(ctrl_for_worker, rx).await
        });
    }
}

////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Serialize, Deserialize)]
struct WebHookRequest {
    destination: String,
    // may be empty
    events: Vec<WebhookEvent>,
}

#[derive(Debug, Serialize, Deserialize)]
struct WebhookEvent {
    #[serde(flatten)]
    common: WebhookEventCommon,
    #[serde(flatten)]
    body: WebhookEventBody,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WebhookEventCommon {
    /// "active" or "standby"
    mode: String,
    timestamp: u64,
    source: Option<Source>,
    webhook_event_id: String,
    delivery_context: DeliveryContext,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
enum Source {
    User {
        user_id: String,
    },
    Group {
        group_id: String,
        user_id: Option<String>,
    },
    Room {
        room_id: String,
        user_id: Option<String>,
    },
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeliveryContext {
    is_redelivery: bool,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
enum WebhookEventBody {
    Message {
        reply_token: String,
        message: RecvMessage,
    },
    Unsend {
        message_id: String,
    },
    Follow {
        reply_token: String,
    },
    Unfollow,
    Join {
        reply_token: String,
    },
    Leave,
    MemberJoined {
        joined: Members,
        reply_token: String,
    },
    MemberLeft {
        left: Members,
        reply_token: String,
    },
    #[serde(other)]
    Other,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
struct Members {
    /// type = "user"
    members: Vec<Source>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
enum RecvMessage {
    /// 送信元から送られたテキストを含むメッセージオブジェクトです。
    Text {
        /// メッセージID
        id: String,
        /// メッセージの引用トークン。
        /// 詳しくは、『Messaging APIドキュメント』の「引用トークンを取得する」を
        /// 参照してください。
        quote_token: String,
        /// メッセージのテキスト
        text: String,
        // emojis
        /// メンションの情報を含むオブジェクト。
        /// textプロパティにメンションが含まれる場合のみ、
        /// メッセージイベントに含まれます。
        mention: Option<Mention>,
        /// 引用されたメッセージのメッセージID。
        /// 過去のメッセージを引用している場合にのみ含まれます。
        quoted_message_id: Option<String>,
    },
    /// 送信元から送られた画像を含むメッセージオブジェクトです。
    Image {
        /// メッセージID
        id: String,
        /// メッセージの引用トークン。
        /// 詳しくは、『Messaging APIドキュメント』の「引用トークンを取得する」を
        /// 参照してください。
        quote_token: String,
        /// 画像ファイルの提供元。
        content_provider: ContentProvider,
    },
    #[serde(other)]
    Other,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
struct Mention {
    mentionees: Vec<Mentionee>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
struct Mentionee {
    index: usize,
    length: usize,
    #[serde(flatten)]
    target: MentioneeTarget,
    quoted_message_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
enum MentioneeTarget {
    User { user_id: Option<String> },
    All,
}

/// 画像ファイルの提供元。
#[derive(Debug, Serialize, Deserialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
enum ContentProvider {
    /// LINEユーザーが画像ファイルを送信しました。
    /// 画像ファイルのバイナリデータは、メッセージIDを指定してコンテンツを取得する
    /// エンドポイントを使用することで取得できます。
    Line,
    /// 画像ファイルのURLは `contentProvider.originalContentUrl`
    /// プロパティに含まれます。
    /// なお、画像ファイルの提供元がexternalの場合、
    /// 画像ファイルのバイナリデータはコンテンツを取得するエンドポイントで
    /// 取得できません。
    External {
        /// 画像ファイルのURL。
        /// contentProvider.typeがexternalの場合にのみ含まれます。
        /// 画像ファイルが置かれているサーバーは、
        /// LINEヤフー株式会社が提供しているものではありません。
        original_content_url: String,
        /// プレビュー画像のURL。
        /// contentProvider.typeがexternalの場合にのみ含まれます。
        /// プレビュー画像が置かれているサーバーは、
        /// LINEヤフー株式会社が提供しているものではありません。
        preview_image_url: String,
        // image_set
    },
}

// web サーバからの呼び出し部分
impl Line {
    /// 署名検証。
    pub fn verify_signature(&self, signature: &str, body: &str) -> Result<()> {
        let channel_secret = self.config.channel_secret.as_str();

        verify_signature(signature, channel_secret, body)
    }

    /// 署名検証後の POST request 処理本体。
    pub fn process_post(&self, json_body: &str) -> Result<()> {
        // JSON parse
        let req = serde_json::from_str::<WebHookRequest>(json_body).inspect_err(|err| {
            error!("[line] Json parse error: {err}");
            error!("[line] {json_body}");
        })?;
        info!("{req:?}");

        // WebhookEvent が最大5個入っている
        for ev in req.events {
            // mode == "active" の時のみ処理
            if ev.common.mode != "active" {
                info!(
                    "[line] Ignore event because mode is not active: {}",
                    ev.common.mode
                );
                continue;
            }
            // 処理タスクキューへディスパッチ
            // キューがいっぱいの場合は警告だけログに出して成功扱いにする
            if self.tx.try_send(ev).is_err() {
                warn!("[line] Task queue full, drop message");
            }
        }

        Ok(())
    }
}

/// 署名検証。
fn verify_signature(signature: &str, channel_secret: &str, body: &str) -> Result<()> {
    let key = channel_secret.as_bytes();
    let data = body.as_bytes();
    let expected = general_purpose::STANDARD.decode(signature)?;

    utils::netutil::hmac_sha256_verify(key, data, &expected)
}

async fn on_text_message(
    ctrl: &Control,
    src: &Option<Source>,
    reply_token: &str,
    text: &str,
) -> Result<()> {
    ensure!(src.is_some(), "Field 'source' is required");
    let src = src.as_ref().unwrap();

    let (prompt, privileged) = {
        let mut line = ctrl.sysmods().line.lock().await;
        let prompt = line.config.prompt.clone();

        // source フィールドからプロフィール情報を取得
        let (display_name, privileged) = source_to_display_name_and_permission(&line, src).await?;

        // タイムアウト処理
        line.check_history_timeout(ctrl).await;

        // 画像バッファから同一ユーザに対する画像リストを抽出
        let imgs = line.image_buffer.remove(&display_name).unwrap_or_default();

        // 今回の発言をヒストリに追加 (システムメッセージ + 本体)
        let sysmsg = prompt.each.join("").replace("${user}", &display_name);
        line.chat_history_mut(ctrl)
            .await
            .push_input_message(Role::Developer, &sysmsg)?;
        line.chat_history_mut(ctrl)
            .await
            .push_input_and_images(Role::User, text, imgs)?;

        (prompt, privileged)
    };

    // システムメッセージ
    let inst = prompt.instructions.join("");
    // ツール (function + built-in tools)
    let mut tools = vec![];
    // web search
    tools.push(Tool::WebSearchPreview {
        search_context_size: Some(SearchContextSize::Medium),
        user_location: Some(UserLocation::default()),
    });
    // function
    {
        let mut line = ctrl.sysmods().line.lock().await;
        for f in line.func_table(ctrl).await.function_list() {
            tools.push(Tool::Function(f.clone()));
        }
    }

    // function 用コンテキスト情報
    let reply_to = match src {
        Source::User { user_id } => user_id,
        Source::Group {
            group_id,
            user_id: _,
        } => group_id,
        Source::Room {
            room_id: _,
            user_id: _,
        } => bail!("Source::Room is not supported"),
    };

    let mut func_trace = String::new();
    let reply_msg = loop {
        let mut line = ctrl.sysmods().line.lock().await;

        let resp = {
            let mut ai = ctrl.sysmods().openai.lock().await;

            // ヒストリの中身全部を追加
            let input = Vec::from_iter(line.chat_history(ctrl).await.iter().cloned());
            // ChatGPT API
            ai.chat_with_tools(Some(&inst), input, &tools).await
        };

        match resp {
            Ok(resp) => {
                // function 呼び出しがあれば履歴に追加
                for fc in resp.func_call_iter() {
                    let call_id = &fc.call_id;
                    let func_name = &fc.name;
                    let func_args = &fc.arguments;

                    // 関数に渡すコンテキスト情報 (LINE reply_to ID)
                    let ctx = FunctionContext {
                        reply_to: reply_to.clone(),
                        privileged,
                    };
                    // call function
                    let func_out = line
                        .func_table(ctrl)
                        .await
                        .call(ctx, func_name, func_args)
                        .await;
                    // debug trace
                    if line.func_table(ctrl).await.debug_mode() {
                        if !func_trace.is_empty() {
                            func_trace.push('\n');
                        }
                        func_trace += &format!(
                            "function call: {func_name}\nparameters: {func_args}\nresult: {func_out}"
                        );
                    }
                    // function の結果を履歴に追加
                    line.chat_history_mut(ctrl)
                        .await
                        .push_function(call_id, func_name, func_args, &func_out)?;
                }
                // アシスタント応答と web search があれば履歴に追加
                let text = resp.output_text();
                let text = if text.is_empty() { None } else { Some(text) };
                line.chat_history_mut(ctrl)
                    .await
                    .push_output_and_tools(text.as_deref(), resp.web_search_iter().cloned())?;

                if let Some(text) = text {
                    break Ok(text);
                }
            }
            Err(err) => {
                // エラーが発生した
                error!("{err:#?}");
                break Err(err);
            }
        }
    };

    // LINE へ返信
    {
        let mut line = ctrl.sysmods().line.lock().await;

        let mut msgs: Vec<&str> = Vec::new();
        if !func_trace.is_empty() {
            msgs.push(&func_trace);
        }
        match reply_msg {
            Ok(reply_msg) => {
                msgs.push(&reply_msg);
                for msg in msgs.iter() {
                    info!("[line] openai reply: {msg}");
                }
                line.reply_multi(reply_token, &msgs).await?;

                // タイムアウト延長
                line.postpone_timeout();
            }
            Err(err) => {
                error!("[line] openai error: {err:#?}");
                let errmsg = match OpenAi::error_kind(&err) {
                    OpenAiErrorKind::Timeout => prompt.timeout_msg,
                    OpenAiErrorKind::RateLimit => prompt.ratelimit_msg,
                    OpenAiErrorKind::QuotaExceeded => prompt.quota_msg,
                    _ => prompt.error_msg,
                };
                msgs.push(&errmsg);
                for msg in msgs.iter() {
                    info!("[line] openai reply: {msg}");
                }
                line.reply_multi(reply_token, &msgs).await?;
            }
        }
    }

    Ok(())
}

async fn on_image_message(
    ctrl: &Control,
    src: &Option<Source>,
    id: &str,
    _reply_token: &str,
    content_provider: &ContentProvider,
) -> Result<()> {
    ensure!(src.is_some(), "Field 'source' is required");
    let src = src.as_ref().unwrap();

    let mut line = ctrl.sysmods().line.lock().await;

    // source フィールドからプロフィール情報を取得
    let (display_name, _privileged) = source_to_display_name_and_permission(&line, src).await?;

    // 画像を取得
    let bin = match content_provider {
        ContentProvider::Line => line.get_content(id).await?,
        ContentProvider::External {
            original_content_url,
            preview_image_url: _,
        } => {
            const CONN_TIMEOUT: Duration = Duration::from_secs(30);
            const TIMEOUT: Duration = Duration::from_secs(30);

            let client = reqwest::ClientBuilder::new()
                .connect_timeout(CONN_TIMEOUT)
                .timeout(TIMEOUT)
                .build()?;
            netutil::checked_get_url_bin(&client, original_content_url)
                .await
                .context("URL get network error")?
        }
    };
    let input_content = OpenAi::to_image_input(&bin)?;

    // タイムアウト処理
    line.check_history_timeout(ctrl).await;
    // 今回の発言を一時バッファに追加
    if let Some(v) = line.image_buffer.get_mut(&display_name) {
        v.push(input_content);
    } else {
        line.image_buffer.insert(display_name, vec![input_content]);
    }

    Ok(())
}

async fn source_to_display_name_and_permission(
    line: &Line,
    src: &Source,
) -> Result<(String, bool)> {
    let user_id = match src {
        Source::User { user_id } => user_id,
        Source::Group {
            group_id: _,
            user_id,
        } => {
            if let Some(user_id) = user_id {
                user_id
            } else {
                bail!("userId is null");
            }
        }
        Source::Room {
            room_id: _,
            user_id: _,
        } => bail!("Source::Room is not supported"),
    };
    let display_name = if let Some(name) = line.config.id_name_map.get(user_id) {
        name.to_string()
    } else {
        line.get_profile(user_id).await?.display_name
    };
    let privileged = line.config.privileged_user_ids.contains(user_id);

    Ok((display_name, privileged))
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
    messages: Vec<SendMessage>,
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
    messages: Vec<SendMessage>,
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

/// 画像サイズ制限。
/// 仕様ぴったりの 10 MB はスマホの通信料に対して大きすぎるので控えめにする。
const IMAGE_ORIGINAL_SIZE_MAX: usize = 1 * 1000 * 1000;
/// プレビュー画像サイズ制限。
/// 仕様は 1 MB。
const IMAGE_PREVIEW_SIZE_MAX: usize = 200 * 1000;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(tag = "type")]
#[serde_with::skip_serializing_none]
enum SendMessage {
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

        if let Some(timeout) = self.history_timeout
            && now > timeout
        {
            self.image_buffer.clear();
            self.chat_history_mut(ctrl).await.clear();
            self.history_timeout = None;
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
            messages.extend(splitted.iter().map(|&chunk| SendMessage::Text {
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
        info!("{resp:?}");

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
            .map(|&chunk| SendMessage::Text {
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
        info!("{resp:?}");

        Ok(resp)
    }

    /// <https://developers.line.biz/ja/reference/messaging-api/#send-push-message>
    pub async fn push_image_message(
        &self,
        to: &str,
        url_original: &str,
        url_preview: &str,
    ) -> Result<ReplyResp> {
        let messages = vec![SendMessage::Image {
            original_content_url: url_original.to_string(),
            preview_image_url: url_preview.to_string(),
        }];

        let req = PushReq {
            to: to.to_string(),
            messages,
            notification_disabled: None,
            custom_aggregation_units: None,
        };
        let resp = self.post_auth_json(URL_PUSH, &req).await?;
        info!("{resp:?}");

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
        let resp = netutil::send_with_retry(|| {
            self.client
                .get(url)
                .header("Authorization", format!("Bearer {token}"))
        })
        .await?;

        Self::check_resp_json(resp).await
    }

    async fn post_auth_json<T, R>(&self, url: &str, body: &T) -> Result<R>
    where
        T: Serialize + Debug,
        R: for<'de> Deserialize<'de>,
    {
        info!("[line] POST {url} {body:?}");
        let token = &self.config.token;
        let resp = netutil::send_with_retry(|| {
            self.client
                .post(url)
                .header("Authorization", format!("Bearer {token}"))
                .json(body)
        })
        .await?;

        Self::check_resp_json(resp).await
    }

    async fn get_auth_bin(&self, url: &str) -> Result<(StatusCode, Vec<u8>)> {
        info!("[line] GET {url}");
        let token = &self.config.token;

        let resp = netutil::send_with_retry(|| {
            self.client
                .get(url)
                .header("Authorization", format!("Bearer {token}"))
        })
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

/// 固有関数: 画像生成
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
            line.push_image_message(&ctx.reply_to, &url, &url).await?;
        }
        Ok(())
    });

    Ok("Accepted. The result will be automatially posted later. Assistant should not draw for now.".to_string())
}

/// 固有関数: カメラで写真を撮る
fn register_camera(func_table: &mut FunctionTable<FunctionContext>) {
    func_table.register_function(
        Function {
            name: "camera".to_string(),
            description: Some("Take a picture".to_string()),
            parameters: Parameters {
                properties: Default::default(),
                required: Default::default(),
                ..Default::default()
            },
            ..Default::default()
        },
        move |bctx, ctx, args| Box::pin(camera(bctx, ctx, args)),
    );
}

async fn camera(bctx: Arc<BasicContext>, ctx: FunctionContext, _args: &FuncArgs) -> Result<String> {
    use crate::sysmod::camera;

    anyhow::ensure!(ctx.privileged, "Permission denied");

    let mut w = camera::PIC_DEF_W;
    let mut h = camera::PIC_DEF_H;
    info!("[line] take a picture: {}x{}", w, h);
    let mut orig = camera::take_a_pic(camera::TakePicOption::new().width(w).height(h)).await?;
    info!("[line] take a picture OK, size={}", orig.len());
    // サイズ制限に収まるまで小さくする
    while orig.len() > IMAGE_ORIGINAL_SIZE_MAX {
        w /= 2;
        h /= 2;
        orig = camera::resize(&orig, w, h)?;
        info!("[line] resize, size={}", orig.len());
    }
    let mut preview = orig.clone();
    while preview.len() > IMAGE_PREVIEW_SIZE_MAX {
        w /= 2;
        h /= 2;
        preview = camera::resize(&preview, w, h)?;
    }
    let (url_original, url_preview) = {
        let mut http = bctx.ctrl.sysmods().http.lock().await;

        (
            http.export_tmp_data(ContentType::jpeg(), orig)?,
            http.export_tmp_data(ContentType::jpeg(), preview)?,
        )
    };

    let ctrl = bctx.ctrl.clone();
    taskserver::spawn_oneshot_fn(&ctrl, "line_camera_send", async move {
        {
            let line = bctx.ctrl.sysmods().line.lock().await;

            line.push_image_message(&ctx.reply_to, &url_original, &url_preview)
                .await?;
        }
        Ok(())
    });

    Ok("OK. Now the users can see the picture.".to_string())
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

    #[test]
    fn parse_text_message() {
        // https://developers.line.biz/ja/reference/messaging-api/#wh-text
        let src = r#"
{
  "destination": "xxxxxxxxxx",
  "events": [
    {
      "replyToken": "nHuyWiB7yP5Zw52FIkcQobQuGDXCTA",
      "type": "message",
      "mode": "active",
      "timestamp": 1462629479859,
      "source": {
        "type": "group",
        "groupId": "Ca56f94637c...",
        "userId": "U4af4980629..."
      },
      "webhookEventId": "01FZ74A0TDDPYRVKNK77XKC3ZR",
      "deliveryContext": {
        "isRedelivery": false
      },
      "message": {
        "id": "444573844083572737",
        "type": "text",
        "quoteToken": "q3Plxr4AgKd...",
        "text": "@All @example Good Morning!! (love)",
        "emojis": [
          {
            "index": 29,
            "length": 6,
            "productId": "5ac1bfd5040ab15980c9b435",
            "emojiId": "001"
          }
        ],
        "mention": {
          "mentionees": [
            {
              "index": 0,
              "length": 4,
              "type": "all"
            },
            {
              "index": 5,
              "length": 8,
              "userId": "U49585cd0d5...",
              "type": "user",
              "isSelf": false
            }
          ]
        }
      }
    }
  ]
}"#;
        let _: WebHookRequest = serde_json::from_str(src).unwrap();
    }

    #[test]
    fn parse_image_message() {
        // https://developers.line.biz/ja/reference/messaging-api/#wh-image
        let src1 = r#"
{
    "destination": "xxxxxxxxxx",
    "events": [
        {
            "type": "message",
            "message": {
                "type": "image",
                "id": "354718705033693859",
                "quoteToken": "q3Plxr4AgKd...",
                "contentProvider": {
                    "type": "line"
                },
                "imageSet": {
                    "id": "E005D41A7288F41B65593ED38FF6E9834B046AB36A37921A56BC236F13A91855",
                    "index": 1,
                    "total": 2
                }
            },
            "timestamp": 1627356924513,
            "source": {
                "type": "user",
                "userId": "U4af4980629..."
            },
            "webhookEventId": "01FZ74A0TDDPYRVKNK77XKC3ZR",
            "deliveryContext": {
                "isRedelivery": false
            },
            "replyToken": "7840b71058e24a5d91f9b5726c7512c9",
            "mode": "active"
        }
    ]
}"#;
        let src2 = r#"
{
    "destination": "xxxxxxxxxx",
    "events": [
        {
            "type": "message",
            "message": {
                "type": "image",
                "id": "354718705033693861",
                "quoteToken": "yHAz4Ua2wx7...",
                "contentProvider": {
                    "type": "line"
                },
                "imageSet": {
                    "id": "E005D41A7288F41B65593ED38FF6E9834B046AB36A37921A56BC236F13A91855",
                    "index": 2,
                    "total": 2
                }
            },
            "timestamp": 1627356924722,
            "source": {
                "type": "user",
                "userId": "U4af4980629..."
            },
            "webhookEventId": "01FZ74A0TDDPYRVKNK77XKC3ZR",
            "deliveryContext": {
                "isRedelivery": false
            },
            "replyToken": "fbf94e269485410da6b7e3a5e33283e8",
            "mode": "active"
        }
    ]
}"#;
        let _: WebHookRequest = serde_json::from_str(src1).unwrap();
        let _: WebHookRequest = serde_json::from_str(src2).unwrap();
    }

    #[test]
    fn base64_decode() {
        let line_signature = "A+JCmhu7Tg6f4lwANmLGirCS2rY8kHBmSG18ctUtvjQ=";
        let res = verify_signature(line_signature, "1234567890", "test");
        assert!(res.is_err());
        // base64 decode に成功し、MAC 検証に失敗する
        assert!(res.unwrap_err().to_string().contains("MAC"));
    }
}
