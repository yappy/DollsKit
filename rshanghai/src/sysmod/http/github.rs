//! Github Webhook.
//!
//! <https://docs.github.com/ja/developers/webhooks-and-events/webhooks>

use std::sync::Arc;

use super::WebResult;
use crate::{
    sys::taskserver::{self, Control},
    utils::netutil,
};
use actix_web::{http::header::ContentType, web, HttpRequest, HttpResponse, Responder};
use anyhow::{anyhow, Result};
use log::{error, info};
use serde_json::Value;

#[actix_web::get("/github/")]
async fn index_get() -> impl Responder {
    let body = r#"<!DOCTYPE html>
<html lang="en">
  <head>
    <title>Github Webhook</title>
  </head>
  <body>
    <h1>Github Webhook</h1>
    <p>Your request is GET.</p>
  </body>
</html>
"#;

    HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(body)
}

/// Github webhook 設定で Content type = application/json に設定すること。
#[actix_web::post("/github/")]
async fn index_post(req: HttpRequest, body: String, ctrl: web::Data<Control>) -> WebResult {
    info!("POST github webhook");

    let headers = req.headers();
    let event = headers.get("X-GitHub-Event");
    let delivery = headers.get("X-GitHub-Delivery");
    let signature256 = headers.get("X-Hub-Signature-256");
    if event.is_none() || delivery.is_none() || signature256.is_none() {
        error!("400: X-GitHub-Event, X-GitHub-Delivery, X-Hub-Signature-256 required");
        return Ok(HttpResponse::BadRequest()
            .content_type(ContentType::plaintext())
            .body("X-GitHub-Event, X-GitHub-Delivery, X-Hub-Signature-256 required"));
    }

    let event = event.unwrap().to_str();
    let delivery = delivery.unwrap().to_str();
    let signature256 = signature256.unwrap().to_str();
    if event.is_err() || delivery.is_err() || signature256.is_err() {
        error!("400: Bad HTTP header");
        return Ok(HttpResponse::BadRequest()
            .content_type(ContentType::plaintext())
            .body("Bad X-Hub header"));
    }
    let event = event.unwrap();
    let delivery = delivery.unwrap();
    let signature256 = signature256.unwrap();
    info!("X-GitHub-Event: {}", event);
    info!("X-GitHub-Delivery: {}", delivery);
    info!("X-Hub-Signature-256: {}", signature256);

    // "push" 以外は無視する
    if event != "push" {
        info!(r#"event "{}" ignored"#, event);
        return Ok(HttpResponse::BadRequest()
            .content_type(ContentType::plaintext())
            .body(""));
    }

    // "sha256=" の部分を取り除く
    let prefix = "sha256=";
    if !signature256.starts_with(prefix) {
        error!("400: Invalid signature");
        return Ok(HttpResponse::BadRequest()
            .content_type(ContentType::plaintext())
            .body("Invalid signature"));
    }
    let signature256 = &signature256[prefix.len()..];

    // 16 進文字列を 2 文字ずつ u8 配列に変換する
    if signature256.len() % 2 != 0 {
        return Ok(HttpResponse::BadRequest()
            .content_type(ContentType::plaintext())
            .body("Invalid signature"));
    }
    let mut hash: Vec<u8> = Vec::new();
    for i in (0..signature256.len()).step_by(2) {
        let bytestr = &signature256[i..i + 2];
        let byte = u8::from_str_radix(bytestr, 16).map_err(|_| anyhow!("Invalid signature"))?;
        hash.push(byte);
    }

    // json body の SHA256 を計算して検証する
    let secret = ctrl
        .sysmods()
        .http
        .lock()
        .await
        .config
        .ghhook_secret
        .clone();
    if netutil::hmac_sha256_verify(secret.as_bytes(), body.as_bytes(), &hash).is_err() {
        error!("SHA256 verify error (see github webhook settings)");
        return Ok(HttpResponse::BadRequest()
            .content_type(ContentType::plaintext())
            .body("SHA256 verify error"));
    }
    info!("verify request body OK");

    process_post(&ctrl, &body).await;

    Ok(HttpResponse::Ok()
        .content_type(ContentType::plaintext())
        .body(""))
}

async fn process_post(ctrl: &Control, json_body: &str) {
    match create_msg_from_json(json_body) {
        Ok(msg) => {
            let ctrl_clone = Arc::clone(ctrl);
            let msg_clone = msg.clone();
            taskserver::spawn_oneshot_fn(ctrl, "http-github-tweet", async move {
                ctrl_clone
                    .sysmods()
                    .twitter
                    .lock()
                    .await
                    .tweet(&msg_clone)
                    .await
            });

            let ctrl_clone = ctrl.clone();
            taskserver::spawn_oneshot_fn(ctrl, "http-github-discord", async move {
                ctrl_clone.sysmods().discord.lock().await.say(&msg).await
            });
        }
        Err(why) => {
            error!("{:#?}", why);
        }
    }
}

fn create_msg_from_json(json_body: &str) -> Result<String> {
    let root: Value = serde_json::from_str(json_body)?;

    let refstr = root["ref"]
        .as_str()
        .ok_or_else(|| anyhow!("ref not found"))?;
    let compare = root["compare"]
        .as_str()
        .ok_or_else(|| anyhow!("compare not found"))?;

    Ok(format!("Pushed to Github: {refstr}\n{compare}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn webhook_simple_push() {
        let jsonstr = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/res/test/github/simplepush.json"
        ));
        let msg = create_msg_from_json(jsonstr).unwrap();

        assert_eq!(msg, "Pushed to Github: refs/heads/rust\nhttps://github.com/yappy/DollsKit/compare/ac61a0d5b3e5...2faf7b5f1bb6");
    }

    #[test]
    fn webhook_empty_branch() {
        let jsonstr = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/res/test/github/emptybranch.json"
        ));
        let msg = create_msg_from_json(jsonstr).unwrap();

        assert_eq!(
            msg,
            "Pushed to Github: refs/heads/newbranch\nhttps://github.com/yappy/DollsKit/compare/newbranch"
        );
    }

    #[test]
    fn webhook_delete_branch() {
        let jsonstr = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/res/test/github/deletebranch.json"
        ));
        let msg = create_msg_from_json(jsonstr).unwrap();

        assert_eq!(msg, "Pushed to Github: refs/heads/newbranch\nhttps://github.com/yappy/DollsKit/compare/9591d010ba32...000000000000");
    }
}
