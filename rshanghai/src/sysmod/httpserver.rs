use super::SystemModule;
use crate::sys::{config, taskserver::Control};
use actix_web::{http::header::ContentType, HttpResponse, Responder};
use anyhow::{anyhow, Result};
use log::info;
use serde::{Deserialize, Serialize};
use tokio::select;

#[derive(Clone, Serialize, Deserialize)]
struct HttpConfig {
    enabled: bool,
    inaddr_any: bool,
}

#[actix_web::get("/")]
async fn hello() -> impl Responder {
    HttpResponse::Ok()
        .content_type(ContentType::plaintext())
        .body("Hello world!\n")
}

pub struct HttpServer {
    config: HttpConfig,
}

impl HttpServer {
    pub fn new() -> Result<Self> {
        info!("[health] initialize");

        let jsobj =
            config::get_object(&["http"]).map_or(Err(anyhow!("Config not found: http")), Ok)?;
        let config: HttpConfig = serde_json::from_value(jsobj)?;

        Ok(Self { config })
    }
}

async fn http_main_task(mut ctrl: Control) -> Result<()> {
    loop {
        let config = {
            let http = ctrl.sysmods().http.lock().await;
            http.config.clone()
        };
        let addr = if config.inaddr_any {
            "0.0.0.0"
        } else {
            "127.0.0.1"
        };
        let server = actix_web::HttpServer::new(|| actix_web::App::new().service(hello))
            .disable_signals()
            .bind(("0.0.0.0", 8080))?
            .run();

        select! {
            _ = server => {
                info!("[http] server exit (reboot)");
            },
            _ = ctrl.cancel_rx().changed() => {
                info!("[http] task cancel");
                break;
            },
        }
    }

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
