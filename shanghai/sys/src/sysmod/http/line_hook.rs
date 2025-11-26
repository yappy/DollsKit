//! LINE Webhook.
//!
//! <https://developers.line.biz/ja/docs/messaging-api/>

use super::{ActixError, WebResult};
use crate::taskserver::Control;

use actix_web::{HttpRequest, HttpResponse, Responder, http::header::ContentType, web};
use log::info;

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

    // get signature
    let headers = req.headers();
    let signature = if let Some(s) = headers.get("x-line-signature") {
        if let Ok(s) = s.to_str() {
            s
        } else {
            return Err(ActixError::new("Bad x-line-signature header", 400));
        }
    } else {
        return Err(ActixError::new("x-line-signature required", 400));
    };
    info!("x-line-signature: {signature}");

    {
        let line = ctrl.sysmods().line.lock().await;
        // verify signature
        if let Err(err) = line.verify_signature(signature, &body) {
            return Err(ActixError::new(&err.to_string(), 401));
        }

        if let Err(e) = line.process_post(&body) {
            return Err(ActixError::new(&e.to_string(), 400));
        }
    }
    Ok(HttpResponse::Ok()
        .content_type(ContentType::plaintext())
        .body(""))
}
