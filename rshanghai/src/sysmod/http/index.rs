use super::HttpConfig;
use crate::sys::{taskserver::Control, version};
use actix_web::{http::header::ContentType, web, HttpResponse, Responder};
use chrono::Local;
use std::sync::atomic::{AtomicU32, Ordering};

struct IndexState {
    access_counter: AtomicU32,
}

pub(super) fn server_config() -> impl Fn(&mut web::ServiceConfig, &HttpConfig) -> () + Clone {
    let state = web::Data::new(IndexState {
        access_counter: Default::default(),
    });

    move |cfg: &mut web::ServiceConfig, http_config: &HttpConfig| {
        cfg.app_data(state.clone());
        cfg.service(index_get);
    }
}

#[actix_web::get("/")]
async fn index_get(
    state: web::Data<IndexState>,
    cfg: web::Data<HttpConfig>,
    ctrl: web::Data<Control>,
) -> impl Responder {
    let counter = state.access_counter.fetch_add(1, Ordering::Relaxed) + 1;

    let sysinfo = ctrl.sysmods().sysinfo.lock().await;
    let started = sysinfo.started;
    let op_time = Local::now() - sysinfo.started;
    // unlock
    drop(sysinfo);

    let day = op_time.num_days();
    let hour = op_time.num_hours() - op_time.num_days() * 24;
    let min = op_time.num_minutes() - op_time.num_hours() * 60;
    let sec = op_time.num_seconds() - op_time.num_minutes() * 60;
    let ms = op_time.num_milliseconds() - op_time.num_seconds() * 1000;
    let info_list = [
        format!("Started: {}", started.format("%F %T %:z")),
        format!(
            "Operated for: {} days, {:02}:{:02}:{:02}.{:03}",
            day, hour, min, sec, ms
        ),
    ];

    let info_str = info_list
        .iter()
        .map(|s| format!("      <li>{}</li>", s))
        .collect::<Vec<_>>()
        .join("\n");

    let ver_str = version::VERSION_INFO_VEC
        .iter()
        .map(|s| format!("      <li>{}</li>", s))
        .collect::<Vec<_>>()
        .join("\n");

    let body = format!(
        r#"<!DOCTYPE html>
<html lang="en">
  <head>
    <title>House Management System Web Interface</title>
  </head>
  <body>
    <h1>House Management System Web Interface</h1>
    <h2>System Available</h2>
    <ul>
{info}
    </ul>
    <p>Access count: {counter}</p>

    <hr>

    <ul>
{ver}
    </ul>
  </body>
</html>
"#,
        info = info_str,
        counter = counter,
        ver = ver_str
    );

    HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(body)
}
