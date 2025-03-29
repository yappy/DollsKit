//! LINE Webhook.
//!
//! <https://developers.line.biz/ja/docs/messaging-api/>

use std::time::{Duration, Instant};

use super::{ActixError, WebResult};
use crate::{
    sys::taskserver::Control,
    sysmod::{
        line::FunctionContext,
        openai::{ChatMessage, OpenAi, Role},
    },
    utils::netutil,
};
use actix_web::{HttpRequest, HttpResponse, Responder, http::header::ContentType, web};
use anyhow::{Result, anyhow, bail, ensure};
use base64::{Engine, engine::general_purpose};
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
    ensure!(src.is_some(), "Field 'source' is required");
    let src = src.as_ref().unwrap();

    let prompt = {
        let mut line = ctrl.sysmods().line.lock().await;
        let prompt = line.config.prompt.clone();

        // source フィールドからプロフィール情報を取得
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

        // タイムアウト処理
        line.check_history_timeout();

        // 今回の発言をヒストリに追加 (システムメッセージ + 本体)
        let sysmsg = prompt.each.join("").replace("${user}", &display_name);
        line.chat_history_mut().push({
            ChatMessage {
                role: Role::System,
                content: Some(sysmsg),
                ..Default::default()
            }
        });
        line.chat_history_mut().push(ChatMessage {
            role: Role::User,
            content: Some(text.to_string()),
            ..Default::default()
        });

        prompt
    };

    let mut func_trace = String::new();
    let reply_msg = loop {
        let mut line = ctrl.sysmods().line.lock().await;
        let ai = ctrl.sysmods().openai.lock().await;

        // 送信用リスト
        let mut all_msgs = Vec::new();
        // 先頭システムメッセージ
        all_msgs.push(ChatMessage {
            role: Role::System,
            content: Some(prompt.pre.join("")),
            ..Default::default()
        });
        // それ以降 (ヒストリの中身全部) を追加
        for m in line.chat_history().iter() {
            all_msgs.push(m.clone());
        }
        // ChatGPT API
        let reply_msg = ai
            .chat_with_function(&all_msgs, line.func_table().function_list())
            .await;
        match &reply_msg {
            Ok(reply) => {
                // 応答を履歴に追加
                line.chat_history_mut().push(reply.clone());
                if reply.function_call.is_some() {
                    // function call が返ってきた
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
                    let ctx = FunctionContext {
                        reply_to: reply_to.to_string(),
                    };
                    let func_name = &reply.function_call.as_ref().unwrap().name;
                    let func_args = &reply.function_call.as_ref().unwrap().arguments;
                    let func_res = line.func_table_mut().call(ctx, func_name, func_args).await;
                    // debug trace
                    if line.func_table().debug_mode() {
                        if !func_trace.is_empty() {
                            func_trace.push('\n');
                        }
                        func_trace += &format!(
                            "function call: {func_name}\nparameters: {func_args}\nresult: {}",
                            func_res.content.as_ref().unwrap()
                        );
                    }
                    // function 応答を履歴に追加
                    line.chat_history_mut().push(func_res);
                    // continue
                } else {
                    // 通常の応答が返ってきた
                    break reply_msg;
                }
            }
            Err(err) => {
                // エラーが発生した
                error!("{:#?}", err);
                break reply_msg;
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
                let text = reply_msg
                    .content
                    .ok_or_else(|| anyhow!("content required"))?;
                msgs.push(&text);
                for msg in msgs.iter() {
                    info!("[line] openai reply: {msg}");
                }
                line.reply_multi(reply_token, &msgs).await?;

                // タイムアウト延長
                line.chat_timeout = Some(
                    Instant::now() + Duration::from_secs(prompt.history_timeout_min as u64 * 60),
                );
            }
            Err(err) => {
                error!("[line] openai error: {:#?}", err);
                let errmsg = if OpenAi::is_timeout(&err) {
                    prompt.timeout_msg
                } else {
                    prompt.error_msg
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
