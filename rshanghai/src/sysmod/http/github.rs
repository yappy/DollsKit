use super::{HttpConfig, WebResult};
use crate::sys::taskserver::Control;
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
    let event = headers
        .get("X-GitHub-Event")
        .ok_or(anyhow!("X-GitHub-Event not found"))?;
    let event = headers
        .get("X-GitHub-Delivery")
        .ok_or(anyhow!("X-GitHub-Delivery not found"))?;
    let event = headers
        .get("X-Hub-Signature")
        .ok_or(anyhow!("X-Hub-Signature not found"))?;

    info!("github hook\n{}", body);

    Ok(HttpResponse::Ok()
        .content_type(ContentType::plaintext())
        .body(""))
}
