//! LINE Webhook.
//!
//! <https://developers.line.biz/ja/docs/messaging-api/>

use super::{ActixError, WebResult};
use crate::sys::taskserver::Control;
use actix_web::{http::header::ContentType, web, HttpRequest, HttpResponse, Responder};
use anyhow::Result;
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
        error!("400: x-line-signature required");
        return Ok(HttpResponse::BadRequest()
            .content_type(ContentType::plaintext())
            .body("x-line-signature required"));
    }
    let signature = signature.unwrap().to_str();
    if signature.is_err() {
        error!("400: Bad x-line-signature header");
        return Ok(HttpResponse::BadRequest()
            .content_type(ContentType::plaintext())
            .body("Bad x-line-signature header"));
    }
    let signature = signature.unwrap();
    info!("x-line-signature: {}", signature);

    // TODO: verify signature

    if let Err(e) = process_post(&ctrl, &body).await {
        Err(ActixError::new(&e.to_string(), 400))
    } else {
        Ok(HttpResponse::Ok()
            .content_type(ContentType::plaintext())
            .body(""))
    }
}

async fn process_post(ctrl: &Control, json_body: &str) -> Result<()> {
    // TODO
    let req = serde_json::from_str::<WebHookRequest>(json_body).map_err(|err| {
        error!("Json parse error: {json_body}");
        err
    })?;
    info!("{:?}", req);

    for ev in req.events.iter() {
        let line = ctrl.sysmods().line.lock().await;
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
                    // TODO: think reply
                    line.reply(reply_token, text).await?;
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
