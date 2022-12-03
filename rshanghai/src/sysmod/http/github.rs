use super::{HttpConfig, WebResult};
use crate::sys::{net::hmac_sha256_verify, taskserver::Control};
use actix_web::{http::header::ContentType, web, HttpRequest, HttpResponse, Responder};
use anyhow::anyhow;
use log::info;

#[actix_web::get("/github/")]
async fn index_get(cfg: web::Data<HttpConfig>, ctrl: web::Data<Control>) -> impl Responder {
    let body = r#"<!DOCTYPE html>
<html lang="en">
  <head>
    <title>Github Webhook</title>
  </head>
  <body>
    <h1>Github Webhook</h1>
    <p></p>
  </body>
</html>
"#;

    HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(body)
}

#[actix_web::post("/github/")]
async fn index_post(
    req: HttpRequest,
    body: String,
    cfg: web::Data<HttpConfig>,
    ctrl: web::Data<Control>,
) -> WebResult {
    let headers = req.headers();
    let event = headers.get("X-GitHub-Event");
    let delivery = headers.get("X-GitHub-Delivery");
    let signature256 = headers.get("X-Hub-Signature-256");
    if event.is_none() || delivery.is_none() || signature256.is_none() {
        return Ok(HttpResponse::BadRequest()
            .content_type(ContentType::plaintext())
            .body("X-GitHub-Event, X-GitHub-Delivery, X-Hub-Signature-256 required"));
    }
    let event = event.unwrap();
    let delivery = delivery.unwrap();
    let signature256 = signature256.unwrap().to_str();
    if signature256.is_err() {
        return Ok(HttpResponse::BadRequest()
            .content_type(ContentType::plaintext())
            .body("X-GitHub-Event, X-GitHub-Delivery, X-Hub-Signature-256 required"));
    }
    let signature256 = signature256.unwrap();

    // "sha256=" の部分を取り除く
    let prefix = "sha256=";
    if !signature256.starts_with(prefix) {
        return Ok(HttpResponse::BadRequest()
            .content_type(ContentType::plaintext())
            .body("Invalid signature"));
    }
    let signature256 = &signature256[prefix.len()..];
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

    let secret = ctrl
        .sysmods()
        .http
        .lock()
        .await
        .config
        .github_secret
        .clone();
    if hmac_sha256_verify(secret.as_bytes(), body.as_bytes(), &hash).is_err() {
        return Ok(HttpResponse::BadRequest()
            .content_type(ContentType::plaintext())
            .body("SHA256 verify error"));
    }

    Ok(HttpResponse::Ok()
        .content_type(ContentType::plaintext())
        .body(""))
}
