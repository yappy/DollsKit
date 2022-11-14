use super::HttpConfig;
use crate::sys::taskserver::Control;
use actix_web::{http::header::ContentType, web, HttpResponse, Responder};

pub(super) fn server_config() -> impl Fn(&mut web::ServiceConfig, &HttpConfig) -> () + Clone {
    move |cfg: &mut web::ServiceConfig, http_config: &HttpConfig| {
        if !http_config.priv_enabled {
            return;
        }
        cfg.service(index_get);
    }
}

#[actix_web::get("/")]
async fn index_get(cfg: web::Data<HttpConfig>, ctrl: web::Data<Control>) -> impl Responder {
    let body = format!(
        r#"<!DOCTYPE html>
<html lang="en">
  <head>
    <title>(Privileged) House Management System Web Interface</title>
  </head>
  <body>
    <h1>(Privileged) House Management System Web Interface</h1>
    <h2>Caution</h2>
    <p>This backend program does not provide any security schemes.</p>
    <p>Check again the front server settings for client authentication.</p>
  </body>
</html>
"#
    );

    HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(body)
}
