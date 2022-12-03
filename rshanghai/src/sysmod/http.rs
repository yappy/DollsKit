mod github;
mod index;
mod priv_camera;
mod priv_index;

use super::SystemModule;
use crate::sys::{config, taskserver::Control};
use actix_web::{http::header::ContentType, HttpResponse, Responder};
use actix_web::{web, HttpResponseBuilder};
use anyhow::{anyhow, Result};
use log::info;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use std::fmt::Display;

#[derive(Clone, Serialize, Deserialize)]
pub struct HttpConfig {
    enabled: bool,
    priv_enabled: bool,
    port: u16,
    path_prefix: String,
    priv_prefix: String,
    github_hook: bool,
    github_secret: String,
}

pub struct HttpServer {
    config: HttpConfig,
}

impl HttpServer {
    pub fn new() -> Result<Self> {
        info!("[http] initialize");

        let jsobj =
            config::get_object(&["http"]).map_or(Err(anyhow!("Config not found: http")), Ok)?;
        let config: HttpConfig = serde_json::from_value(jsobj)?;

        Ok(Self { config })
    }
}

async fn http_main_task(ctrl: Control) -> Result<()> {
    let http_config = {
        let http = ctrl.sysmods().http.lock().await;
        http.config.clone()
    };

    let port = http_config.port;
    // クロージャ内に move するデータの準備
    let data_config = web::Data::new(http_config.clone());
    let data_ctrl = web::Data::new(ctrl.clone());
    let config_regular = index::server_config();
    let config_privileged = priv_index::server_config();
    // クロージャはワーカースレッドごとに複数回呼ばれる
    let server = actix_web::HttpServer::new(move || {
        actix_web::App::new()
            .app_data(data_config.clone())
            .app_data(data_ctrl.clone())
            .service(root_index_get)
            .service(
                web::scope(&data_config.path_prefix)
                    .configure(|cfg| {
                        config_regular(cfg, &http_config);
                    })
                    .service(web::scope(&data_config.priv_prefix).configure(|cfg| {
                        config_privileged(cfg, &http_config);
                    })),
            )
    })
    .disable_signals()
    .bind(("127.0.0.1", port))?
    .run();

    // シャットダウンが来たらハンドルでサーバを停止するタスクを生成
    let mut ctrl_for_stop = ctrl.clone();
    let handle = server.handle();
    ctrl.spawn_oneshot_fn("http-exit", async move {
        ctrl_for_stop.cancel_rx().changed().await.unwrap();
        info!("[http-exit] recv cancel");
        handle.stop(true).await;
        info!("[http-exit] server stop ok");

        Ok(())
    });

    server.await?;
    info!("[http] server exit");

    Ok(())
}

impl SystemModule for HttpServer {
    fn on_start(&self, ctrl: &Control) {
        info!("[http] on_start");
        if self.config.enabled {
            ctrl.spawn_oneshot_task("http", http_main_task);
        }
    }
}

pub type WebResult = Result<HttpResponse, ActixError>;

#[derive(Debug)]
pub struct ActixError {
    err: anyhow::Error,
    status: StatusCode,
}

impl actix_web::error::ResponseError for ActixError {
    fn error_response(&self) -> HttpResponse<actix_web::body::BoxBody> {
        HttpResponse::build(self.status_code())
            .insert_header(ContentType::plaintext())
            .body(self.status_code().to_string())
    }

    fn status_code(&self) -> StatusCode {
        self.status
    }
}

impl Display for ActixError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}, status={}", self.err, self.status.as_str())
    }
}

impl From<anyhow::Error> for ActixError {
    fn from(err: anyhow::Error) -> ActixError {
        ActixError {
            err,
            status: StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

pub fn simple_error(status: StatusCode) -> HttpResponse {
    let body = format!(
        "{} {}",
        status.as_str(),
        status.canonical_reason().unwrap_or_default()
    );
    HttpResponseBuilder::new(status)
        .content_type(ContentType::plaintext())
        .body(body)
}

#[actix_web::get("/")]
async fn root_index_get(cfg: web::Data<HttpConfig>) -> impl Responder {
    let body = format!(
        r#"<!DOCTYPE html>
<html lang="en">
  <head>
    <title>House Management System Web Interface</title>
  </head>
  <body>
    <h1>House Management System Web Interface</h1>
    <p>This is the root page. Web module is working fine.</p>
    <p>
      This system is intended to be connected from a front web server (reverse proxy).
      Therefore, this page will not be visible from the network.
    </p>
    <p>Application endpoint (reverse proxy root) is <strong>{}</strong>.<p>
  </body>
</html>
"#,
        cfg.path_prefix
    );
    HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(body)
}
