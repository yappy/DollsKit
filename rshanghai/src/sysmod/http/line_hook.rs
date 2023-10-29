//! LINE Webhook.
//!
//! <https://developers.line.biz/ja/docs/messaging-api/>

use super::{ActixError, WebResult};
use crate::{
    sys::taskserver::Control,
    sysmod::openai::{ChatMessage, OpenAi},
    utils::netutil,
};
use actix_web::{http::header::ContentType, web, HttpRequest, HttpResponse, Responder};
use anyhow::{anyhow, bail, ensure, Result};
use base64::{engine::general_purpose, Engine};
use log::{error, info};
use serde::{Deserialize, Serialize};

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
struct WebhookEventCommon {
    /// "active" or "standby"
    mode: String,
    timestamp: u64,
    source: Option<Source>,
    #[serde(rename = "webhookEventId")]
    webhook_event_id: String,
    #[serde(rename = "deliveryContext")]
    delivery_context: DeliveryContext,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
enum Source {
    #[serde(rename = "user")]
    User {
        #[serde(rename = "userId")]
        user_id: String,
    },
    #[serde(rename = "group")]
    Group {
        #[serde(rename = "groupId")]
        group_id: String,
        #[serde(rename = "userId")]
        user_id: Option<String>,
    },
    #[serde(rename = "room")]
    Room {
        #[serde(rename = "roomId")]
        room_id: String,
        #[serde(rename = "userId")]
        user_id: Option<String>,
    },
}

#[derive(Debug, Serialize, Deserialize)]
struct DeliveryContext {
    #[serde(rename = "isRedelivery")]
    is_redelivery: bool,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
enum WebhookEventBody {
    #[serde(rename = "message")]
    Message {
        #[serde(rename = "replyToken")]
        reply_token: String,
        message: Message,
    },
    #[serde(rename = "unsend")]
    Unsend {
        #[serde(rename = "messageId")]
        message_id: String,
    },
    #[serde(rename = "follow")]
    Follow {
        #[serde(rename = "replyToken")]
        reply_token: String,
    },
    #[serde(rename = "unfollow")]
    Unfollow,
    #[serde(rename = "join")]
    Join {
        #[serde(rename = "replyToken")]
        reply_token: String,
    },
    #[serde(rename = "leave")]
    Leave,
    #[serde(rename = "memberJoined")]
    MemberJoined {
        joined: Members,
        #[serde(rename = "replyToken")]
        reply_token: String,
    },
    #[serde(rename = "memberLeft")]
    MemberLeft {
        left: Members,
        #[serde(rename = "replyToken")]
        reply_token: String,
    },
    #[serde(other)]
    Other,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
enum Message {
    #[serde(rename = "text")]
    Text {
        id: String,
        #[serde(rename = "quoteToken")]
        quote_token: String,
        text: String,
        // emojis
        mention: Option<Mention>,
        #[serde(rename = "quotedMessageId")]
        quoted_message_id: Option<String>,
    },
    #[serde(other)]
    Other,
}

#[derive(Debug, Serialize, Deserialize)]
struct Mention {
    mentionees: Vec<Mentionee>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Mentionee {
    index: usize,
    length: usize,
    #[serde(flatten)]
    target: MentioneeTarget,
    #[serde(rename = "quotedMessageId")]
    quoted_message_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
enum MentioneeTarget {
    #[serde(rename = "user")]
    User {
        #[serde(rename = "userId")]
        user_id: Option<String>,
    },
    #[serde(rename = "all")]
    All,
}

#[derive(Debug, Serialize, Deserialize)]
struct Members {
    /// type = "user"
    members: Vec<Source>,
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
    let req = serde_json::from_str::<WebHookRequest>(json_body).map_err(|err| {
        error!("[line] Json parse error: {json_body}");
        err
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

async fn on_text_message(
    ctrl: &Control,
    src: &Option<Source>,
    reply_token: &str,
    text: &str,
) -> Result<()> {
    let (prompt, display_name) = {
        let line = ctrl.sysmods().line.lock().await;
        let prompt = line.config.prompt.clone();

        // source フィールドからプロフィール情報を取得
        ensure!(src.is_some(), "Field 'source' is required");
        let src = src.as_ref().unwrap();
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

        (prompt, display_name)
    };

    let mut msgs = Vec::new();
    // 先頭システムメッセージ
    msgs.push(ChatMessage {
        role: "system".to_string(),
        content: Some(prompt.pre.join("")),
        ..Default::default()
    });
    // 今回の発言 (システムメッセージ + 本体)
    let sysmsg = prompt.each.join("").replace("${user}", &display_name);
    msgs.push({
        ChatMessage {
            role: "system".to_string(),
            content: Some(sysmsg),
            ..Default::default()
        }
    });
    msgs.push(ChatMessage {
        role: "user".to_string(),
        content: Some(text.to_string()),
        ..Default::default()
    });

    loop {
        let line = ctrl.sysmods().line.lock().await;
        let ai = ctrl.sysmods().openai.lock().await;

        // msgs から応答を取得
        let reply = ai
            .chat_with_function(&msgs, line.func_table.function_list())
            .await;
        match &reply {
            Ok(reply) => {
                msgs.push(reply.clone());
                if reply.function_call.is_some() {
                    // function call が返ってきた
                    let func_name = &reply.function_call.as_ref().unwrap().name;
                    let func_args = &reply.function_call.as_ref().unwrap().arguments;
                    let func_res = line.func_table.call(func_name, func_args).await;
                    msgs.push(func_res);
                    // continue
                } else {
                    // 通常の応答が返ってきた
                    // LINE へ返信
                    let msg = reply
                        .content
                        .clone()
                        .ok_or_else(|| anyhow!("content is required"))?;
                    line.reply(reply_token, &msg).await?;
                    break;
                }
            }
            Err(err) => {
                // エラーが発生した
                // LINE へ返信
                error!("{:#?}", err);
                let errmsg = if OpenAi::is_timeout(err) {
                    prompt.timeout_msg
                } else {
                    prompt.error_msg
                };
                line.reply(reply_token, &errmsg).await?;
                break;
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::digest::MacError;

    #[test]
    fn base64_decode() {
        let line_signature = "A+JCmhu7Tg6f4lwANmLGirCS2rY8kHBmSG18ctUtvjQ=";
        let res = verify_signature(line_signature, "1234567890", "test");
        assert!(res.is_err());
        // base64 decode に成功し、MAC 検証に失敗する
        assert!(res.unwrap_err().is::<MacError>());
    }
}
