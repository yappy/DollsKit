//! LINE Webhook.
//!
//! <https://developers.line.biz/ja/docs/messaging-api/>

use super::{ActixError, WebResult};
use crate::sysmod::line::{FunctionContext, Line};
use crate::sysmod::openai::{OpenAi, OpenAiErrorKind, Role, SearchContextSize, Tool, UserLocation};
use crate::taskserver::Control;

use actix_web::{HttpRequest, HttpResponse, Responder, http::header::ContentType, web};
use anyhow::{Context, Result, bail, ensure};
use base64::{Engine, engine::general_purpose};
use log::{error, info};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use utils::netutil;

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
        message: Message,
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
enum Message {
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

#[actix_web::get("/line/")]
async fn index_get() -> impl Responder {
    let body = r#"<!DOCTYPE html>
<html lang="en">
  <head>
    <title>LINE Webhook</title>
  </head>
  <body>
    <h1>LINE Webhook</h1>
    <p>Your request is GET.</p>
  </body>
</html>
"#;

    HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(body)
}

#[actix_web::post("/line/")]
async fn index_post(req: HttpRequest, body: String, ctrl: web::Data<Control>) -> WebResult {
    info!("LINE github webhook");

    let headers = req.headers();
    let signature = headers.get("x-line-signature");
    if signature.is_none() {
        return Err(ActixError::new("x-line-signature required", 400));
    }
    let signature = signature.unwrap().to_str();
    if signature.is_err() {
        return Err(ActixError::new("Bad x-line-signature header", 400));
    }
    let signature = signature.unwrap();
    info!("x-line-signature: {}", signature);

    // verify signature
    let channel_secret = {
        let line = ctrl.sysmods().line.lock().await;
        line.config.channel_secret.clone()
    };
    if let Err(err) = verify_signature(signature, &channel_secret, &body) {
        return Err(ActixError::new(&err.to_string(), 401));
    }

    if let Err(e) = process_post(&ctrl, &body).await {
        Err(ActixError::new(&e.to_string(), 400))
    } else {
        Ok(HttpResponse::Ok()
            .content_type(ContentType::plaintext())
            .body(""))
    }
}

/// 署名検証。
fn verify_signature(signature: &str, channel_secret: &str, body: &str) -> Result<()> {
    let key = channel_secret.as_bytes();
    let data = body.as_bytes();
    let expected = general_purpose::STANDARD.decode(signature)?;

    netutil::hmac_sha256_verify(key, data, &expected)
}

/// 署名検証後の POST request 処理本体。
async fn process_post(ctrl: &Control, json_body: &str) -> Result<()> {
    // JSON parse
    let req = serde_json::from_str::<WebHookRequest>(json_body).inspect_err(|err| {
        error!("[line] Json parse error: {err}");
        error!("[line] {json_body}");
    })?;
    info!("{:?}", req);

    // WebhookEvent が最大5個入っている
    for ev in req.events.iter() {
        // mode == "active" の時のみ処理
        if ev.common.mode != "active" {
            info!(
                "[line] Ignore event because mode is not active: {}",
                ev.common.mode
            );
            continue;
        }

        // イベントタイプに応じた処理にディスパッチ
        match &ev.body {
            WebhookEventBody::Message {
                reply_token,
                message,
            } => match message {
                Message::Text {
                    id: _,
                    quote_token: _,
                    text,
                    mention: _,
                    quoted_message_id: _,
                } => {
                    info!("[line] Receive text message: {text}");
                    on_text_message(ctrl, &ev.common.source, reply_token, text).await?;
                }
                Message::Image {
                    id,
                    quote_token: _,
                    content_provider,
                } => {
                    info!("[line] Receive image message");
                    on_image_message(ctrl, &ev.common.source, id, reply_token, content_provider)
                        .await?;
                }
                other => {
                    info!("[line] Ignore message type: {:?}", other);
                }
            },
            other => {
                info!("[line] Ignore event: {:?}", other);
            }
        }
    }

    Ok(())
}

async fn source_to_display_name(line: &Line, src: &Source) -> Result<String> {
    let display_name = match src {
        Source::User { user_id } => {
            if let Some(name) = line.config.id_name_map.get(user_id) {
                name.to_string()
            } else {
                line.get_profile(user_id).await?.display_name
            }
        }
        Source::Group { group_id, user_id } => {
            if let Some(user_id) = user_id {
                if let Some(name) = line.config.id_name_map.get(user_id) {
                    name.to_string()
                } else {
                    line.get_group_profile(group_id, user_id)
                        .await?
                        .display_name
                }
            } else {
                bail!("userId is null");
            }
        }
        Source::Room {
            room_id: _,
            user_id: _,
        } => bail!("Source::Room is not supported"),
    };

    Ok(display_name)
}

async fn on_text_message(
    ctrl: &Control,
    src: &Option<Source>,
    reply_token: &str,
    text: &str,
) -> Result<()> {
    ensure!(src.is_some(), "Field 'source' is required");
    let src = src.as_ref().unwrap();

    let prompt = {
        let mut line = ctrl.sysmods().line.lock().await;
        let prompt = line.config.prompt.clone();

        // source フィールドからプロフィール情報を取得
        let display_name = source_to_display_name(&line, src).await?;

        // タイムアウト処理
        line.check_history_timeout(ctrl).await;

        // 画像バッファから同一ユーザに対する画像リストを抽出
        let imgs = line.image_buffer.remove(&display_name).unwrap_or_default();

        // 今回の発言をヒストリに追加 (システムメッセージ + 本体)
        let sysmsg = prompt.each.join("").replace("${user}", &display_name);
        line.chat_history_mut(ctrl)
            .await
            .push_message(Role::Developer, &sysmsg)?;
        line.chat_history_mut(ctrl)
            .await
            .push_message_images(Role::User, text, imgs)?;

        prompt
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
                            "function call: {func_name}\nparameters: {func_args}\nresult: {}",
                            func_out
                        );
                    }
                    // function の結果を履歴に追加
                    line.chat_history_mut(ctrl)
                        .await
                        .push_function(call_id, func_name, func_args, &func_out)?;
                }
                // アシスタント応答と web search があれば履歴に追加
                let text = resp.output_text();
                if !text.is_empty() {
                    line.chat_history_mut(ctrl).await.push_message_tool(
                        std::iter::once((Role::Assistant, text.clone())),
                        resp.web_search_iter().cloned(),
                    )?;
                } else {
                    line.chat_history_mut(ctrl)
                        .await
                        .push_message_tool(std::iter::empty(), resp.web_search_iter().cloned())?;
                }

                if !text.is_empty() {
                    break Ok(text);
                }
            }
            Err(err) => {
                // エラーが発生した
                error!("{:#?}", err);
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
                error!("[line] openai error: {:#?}", err);
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
    let display_name = source_to_display_name(&line, src).await?;

    // 画像を取得
    let bin = match content_provider {
        ContentProvider::Line => line.get_content(id).await?,
        ContentProvider::External {
            original_content_url,
            preview_image_url: _,
        } => {
            const TIMEOUT: Duration = Duration::from_secs(30);
            let client = reqwest::ClientBuilder::new().timeout(TIMEOUT).build()?;
            let resp = client
                .get(original_content_url)
                .send()
                .await
                .context("URL get network error")?;

            netutil::check_http_resp_bin(resp)
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

#[cfg(test)]
mod tests {
    use super::*;

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
