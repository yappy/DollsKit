use std::collections::BTreeMap;

use super::{priv_camera, HttpConfig};
use crate::sys::{net::html_escape, taskserver::Control};
use actix_web::{http::header::ContentType, web, HttpRequest, HttpResponse, Responder};

pub(super) fn server_config() -> impl Fn(&mut web::ServiceConfig, &HttpConfig) + Clone {
    move |cfg: &mut web::ServiceConfig, http_config: &HttpConfig| {
        if !http_config.priv_enabled {
            return;
        }
        cfg.service(index_get);
        cfg.service(priv_camera::camera_get);
        cfg.service(priv_camera::camera_history_get);
        cfg.service(priv_camera::camera_history_start_get);
        cfg.service(priv_camera::camera_pic_history_get);
        cfg.service(priv_camera::camera_take_get);
    }
}

#[actix_web::get("/")]
async fn index_get(
    req: HttpRequest,
    cfg: web::Data<HttpConfig>,
    ctrl: web::Data<Control>,
) -> impl Responder {
    let mut sorted = BTreeMap::new();
    for (k, v) in req.headers() {
        sorted.insert(k.as_str(), v.to_str().unwrap_or_default());
    }

    let mut header_str = String::new();
    for (k, v) in sorted {
        let k = html_escape(k);
        let v = html_escape(v);
        header_str.push_str(&format!("      <li>{}: {}</li>\n", k, v));
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
