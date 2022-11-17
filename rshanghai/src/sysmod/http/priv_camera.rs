use std::io::Cursor;

use super::{HttpConfig, WebResult};
use crate::{
    sys::taskserver::Control,
    sysmod::camera::{create_thumbnail, take_a_pic, TakePicOption},
    sysmod::http::simple_error,
};
use actix_web::{http::header::ContentType, web, HttpResponse, Responder};
use anyhow::anyhow;
use log::error;
use reqwest::StatusCode;
use tokio::{fs::File, io::AsyncReadExt};

#[actix_web::get("/camera/")]
async fn camera_take_get(cfg: web::Data<HttpConfig>, ctrl: web::Data<Control>) -> impl Responder {
    let body = r#"<!DOCTYPE html>
<html lang="en">
  <head>
    <title>(Privileged) Camera</title>
  </head>
  <body>
    <h1>(Privileged) Camera</h1>
    <p><a href="./take">Take a picture!</a></p>
  </body>
</html>
"#;

    HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(body)
}

#[actix_web::get("/camera/pic/history/{name}/{kind}")]
async fn camera_pic_history_get(
    cfg: web::Data<HttpConfig>,
    ctrl: web::Data<Control>,
    path: web::Path<(String, String)>,
) -> WebResult {
    let (name, kind) = path.into_inner();
    let is_th = match kind.as_str() {
        "main" => false,
        "thumb" => true,
        _ => return Ok(simple_error(StatusCode::BAD_REQUEST)),
    };

    let camera = ctrl.sysmods().camera.lock().await;
    let (dict, _) = camera.pic_list();
    let value = dict.get(&name).cloned();
    drop(camera);

    if let Some(entry) = value {
        let path = match is_th {
            false => entry.path_main,
            true => entry.path_th,
        };
        let mut file = File::open(path).await.map_err(|e| anyhow!(e))?;
        let mut bin = Vec::new();
        let _ = file.read_to_end(&mut bin).await.map_err(|e| anyhow!(e))?;

        let resp = HttpResponse::Ok()
            .content_type(ContentType::jpeg())
            .body(bin);
        Ok(resp)
    } else {
        let resp = HttpResponse::NotFound()
            .content_type(ContentType::plaintext())
            .body("Not Found");
        Ok(resp)
    }
}

#[actix_web::get("/camera/take")]
async fn camera_get(cfg: web::Data<HttpConfig>, ctrl: web::Data<Control>) -> WebResult {
    let pic = take_a_pic(TakePicOption::new()).await;
    if let Err(ref e) = pic {
        error!("take a picture error");
        error!("{:#}", e);
    }
    let pic = pic?;

    let mut thumb = Cursor::new(Vec::new());
    create_thumbnail(&mut thumb, &pic)?;

    let mut camera = ctrl.sysmods().camera.lock().await;
    camera.push_pic_history(&pic, thumb.get_ref()).await?;
    drop(camera);

    let resp = HttpResponse::Ok()
        .content_type(ContentType::jpeg())
        .body(pic);
    Ok(resp)
}
