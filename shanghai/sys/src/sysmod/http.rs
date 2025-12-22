//! HTTP Server 機能。
//!
//! actix_web ライブラリ / フレームワークによる。

mod github;
mod index;
mod line_hook;
mod priv_camera;
mod priv_index;
mod upload;

use super::SystemModule;
use crate::taskserver;
use crate::{config, taskserver::Control};
use actix_web::{HttpResponse, Responder, http::header::ContentType};
use actix_web::{HttpResponseBuilder, web};
use anyhow::{Result, anyhow};
use log::{error, info};
use serde::{Deserialize, Serialize};
use serenity::http::StatusCode;
use std::fmt::Display;
use std::sync::Arc;

/// HTTP Server 設定データ。toml 設定に対応する。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpConfig {
    /// HTTP Server 機能を有効化する。
    enabled: bool,
    /// 管理者専用ページを有効化する。
    priv_enabled: bool,
    /// ポート番号。
    port: u16,
    /// ルートパス。
    /// リバースプロキシ条件の URL プレフィクスに合わせること。
    ///
    /// 例: "/rhouse" で始まるものを転送するという条件のリバースプロキシ設定の場合、
    /// "/rhouse/some/path" はそのままバックエンドサーバに送られる。
    /// ここで [path_prefix] を "/rhouse" に設定すると、
    /// バックエンドサーバ側では "{path_prefix}/some/path" が有効なパスとなり、
    /// 正常に稼働する。
    path_prefix: String,
    /// 管理者専用ページのルートパス。[path_prefix] の後に連結される。
    /// リバースプロキシ条件の URL プレフィクスに合わせること。
    priv_prefix: String,
    /// アップローダ機能を有効化する。パスは /_rootpath_/upload/。
    upload_enabled: bool,
    /// アップロードされたファイルの保存場所。
    upload_dir: String,
    /// GitHub Hook 機能を有効化する。パスは /_rootpath_/github/。
    ghhook_enabled: bool,
    /// GitHub Hook の SHA256 検証に使うハッシュ。GitHub の設定ページから手に入る。
    ghhook_secret: String,
    /// LINE webhook 機能を有効化する。パスは /_rootpath_/line/。
    line_hook_enabled: bool,
}

impl Default for HttpConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            priv_enabled: false,
            port: 8899,
            path_prefix: "/rhouse".to_string(),
            priv_prefix: "/priv".to_string(),
            upload_enabled: false,
            upload_dir: "./upload".to_string(),
            ghhook_enabled: false,
            ghhook_secret: "".to_string(),
            line_hook_enabled: false,
        }
    }
}

pub struct HttpServer {
    config: HttpConfig,
}

impl HttpServer {
    pub fn new() -> Result<Self> {
        info!("[http] initialize");

        let config = config::get(|cfg| cfg.http.clone());

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
    let ctrl_for_stop = Arc::clone(&ctrl);
    let handle = server.handle();
    taskserver::spawn_oneshot_fn(&ctrl, "http-exit", async move {
        ctrl_for_stop.wait_cancel_rx().await;
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
    fn on_start(&mut self, ctrl: &Control) {
        info!("[http] on_start");
        if self.config.enabled {
            taskserver::spawn_oneshot_task(ctrl, "http", http_main_task);
        }
    }
}

pub type WebResult = Result<HttpResponse, ActixError>;

#[derive(Debug)]
pub struct ActixError {
    err: anyhow::Error,
    status: StatusCode,
}

impl ActixError {
    pub fn new(msg: &str, status: u16) -> Self {
        if !(400..600).contains(&status) {
            panic!("status must be 400 <= status < 600");
        }
        ActixError {
            err: anyhow!(msg.to_string()),
            status: StatusCode::from_u16(status).unwrap(),
        }
    }
}

impl actix_web::error::ResponseError for ActixError {
    fn error_response(&self) -> HttpResponse<actix_web::body::BoxBody> {
        error!("HTTP error by Error: {}", self.status);
        error!("{:#}", self.err);

        HttpResponse::build(self.status)
            .insert_header(ContentType::plaintext())
            .body(self.status.to_string())
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

pub fn error_resp(status: StatusCode) -> HttpResponse {
    error_resp_msg(status, status.canonical_reason().unwrap_or_default())
}

pub fn error_resp_msg(status: StatusCode, msg: &str) -> HttpResponse {
    let body = format!("{} {}", status.as_str(), msg);

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
