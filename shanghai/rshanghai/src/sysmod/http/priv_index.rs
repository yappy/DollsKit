use std::collections::BTreeMap;

use super::{HttpConfig, priv_camera};
use crate::utils::netutil;
use actix_web::{HttpRequest, HttpResponse, Responder, http::header::ContentType, web};

pub(super) fn server_config() -> impl Fn(&mut web::ServiceConfig, &HttpConfig) + Clone {
    move |cfg: &mut web::ServiceConfig, http_config: &HttpConfig| {
        if !http_config.priv_enabled {
            return;
        }
        cfg.service(index_get);
        cfg.service(priv_camera::take_post);
        cfg.service(priv_camera::history_get);
        cfg.service(priv_camera::archive_get);
        cfg.service(priv_camera::history_post);
        cfg.service(priv_camera::archive_post);
        cfg.service(priv_camera::pic_history_get);
        cfg.service(priv_camera::pic_archive_get);
        cfg.service(priv_camera::index_get);
    }
}

#[actix_web::get("/")]
async fn index_get(req: HttpRequest) -> impl Responder {
    let mut sorted = BTreeMap::new();
    for (k, v) in req.headers() {
        sorted.insert(k.as_str(), v.to_str().unwrap_or_default());
    }

    let mut header_str = String::new();
    for (k, v) in sorted {
        let k = netutil::html_escape(k);
        let v = netutil::html_escape(v);
        header_str.push_str(&format!("      <li>{k}: {v}</li>\n"));
    }

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

    <h2>HTTP Headers</h2>
    <ul>
      {}
    </ul>

    <h2>Camera</h2>
    <p><a href="./camera/">Camera Main Page</a></p>
  </body>
</html>
"#,
        header_str.trim()
    );

    HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(body)
}
