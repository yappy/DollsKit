mod index;
mod priv_camera;
mod priv_index;

use super::SystemModule;
use crate::sys::{config, taskserver::Control};
use actix_web::web;
use actix_web::{dev::ServerHandle, http::header::ContentType, HttpResponse, Responder};
use anyhow::{anyhow, Result};
use log::info;
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct HttpConfig {
    enabled: bool,
    priv_enabled: bool,
    port: u16,
    path_prefix: String,
    priv_prefix: String,
    github_hook: bool,
}

pub struct HttpServer {
    config: HttpConfig,
    handle: Option<ServerHandle>,
}

impl HttpServer {
    pub fn new() -> Result<Self> {
        info!("[health] initialize");

        let jsobj =
            config::get_object(&["http"]).map_or(Err(anyhow!("Config not found: http")), Ok)?;
        let config: HttpConfig = serde_json::from_value(jsobj)?;

        Ok(Self {
            config,
            handle: None,
        })
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

    // self.handle にサーバ停止用のハンドルを保存する
    let mut http = ctrl.sysmods().http.lock().await;
    http.handle = Some(server.handle());
    drop(http);
    // シャットダウンが来たらハンドルでサーバを停止するタスクを生成
    ctrl.spawn_oneshot_task("http-exit", server_exit_task);

    server.await?;
    info!("[http] server exit");

    Ok(())
}

async fn server_exit_task(mut ctrl: Control) -> Result<()> {
    let mut http = ctrl.sysmods().http.lock().await;
    let handle = http.handle.take().unwrap();
    drop(http);

    let result = ctrl.cancel_rx().changed().await;

    handle.stop(true).await;

    Ok(result?)
}

impl SystemModule for HttpServer {
    fn on_start(&self, ctrl: &Control) {
        info!("[http] on_start");
        if self.config.enabled {
            ctrl.spawn_oneshot_task("http", http_main_task);
        }
    }
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
