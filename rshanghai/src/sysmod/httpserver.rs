use super::SystemModule;
use crate::sys::{config, taskserver::Control};
use actix_web::{dev::ServerHandle, http::header::ContentType, HttpResponse, Responder};
use anyhow::{anyhow, Result};
use log::info;
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
struct HttpConfig {
    enabled: bool,
    inaddr_any: bool,
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
    let config = {
        let http = ctrl.sysmods().http.lock().await;
        http.config.clone()
    };
    let addr = if config.inaddr_any {
        "0.0.0.0"
    } else {
        "127.0.0.1"
    };

    // clone してクロージャ内に move する
    let data = AppData::new(ctrl.clone());
    // クロージャはワーカースレッドごとに複数回呼ばれる
    let server = actix_web::HttpServer::new(move || {
        actix_web::App::new().app_data(data.clone()).service(hello)
    })
    .disable_signals()
    .bind(("0.0.0.0", 8080))?
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

type AppData = actix_web::web::Data<Control>;

#[actix_web::get("/")]
async fn hello(ctrl: AppData) -> impl Responder {
    HttpResponse::Ok()
        .content_type(ContentType::plaintext())
        .body("Hello world!\n")
}
