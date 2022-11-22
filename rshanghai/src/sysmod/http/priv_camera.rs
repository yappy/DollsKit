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
use std::cmp;
use std::io::Cursor;
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
    <p><a href="./history/">Picture List</a></p>
    <p><a href="./archive/">Archive</a> (TODO)</p>
    <h2>Navigation</h2>
    <p><a href="../">Main Page</a></p>
  </body>
</html>
"#;

    HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(body)
}

#[actix_web::get("/camera/history/")]
async fn camera_history_get(ctrl: web::Data<Control>) -> HttpResponse {
    camera_history_get_internal(ctrl, 0).await
}

#[actix_web::get("/camera/history/{start}")]
async fn camera_history_start_get(
    ctrl: web::Data<Control>,
    path: web::Path<String>,
) -> HttpResponse {
    let start = path.into_inner();
    let start = match start.parse::<usize>() {
        Ok(n) => {
            if n == 0 {
                return simple_error(StatusCode::BAD_REQUEST);
            }
            n - 1
        }
        Err(_) => {
            return simple_error(StatusCode::BAD_REQUEST);
        }
    };

    camera_history_get_internal(ctrl, start).await
}

async fn camera_history_get_internal(ctrl: web::Data<Control>, start: usize) -> HttpResponse {
    let camera = ctrl.sysmods().camera.lock().await;
    let page_by = camera.config.page_by as usize;
    let (hist, _) = camera.pic_list();
    let total = hist.len();
    let data: Vec<_> = hist.keys().rev().skip(start).take(page_by).collect();
    let mut html = String::new();
    for name in data {
        html.push_str(&format!(
            r#"    <figure class="pic">
      <a href="../pic/history/{0}/main"><img src="../pic/history/{0}/thumb" alt="{0}"></a>
      <figcaption class="pic">{0}</figcaption>
    </figure>
"#,
            name
        ));
    }
    drop(camera);

    let mut page_navi = String::from("<p>");
    for left in (0..total).step_by(page_by) {
        let right = cmp::min(left + page_by - 1, total - 1);
        let link = if (left..=right).contains(&start) {
            format!(r#"{}-{} "#, left + 1, right + 1)
        } else {
            format!(r#"<a href="./{0}">{0}-{1}</a> "#, left + 1, right + 1)
        };
        page_navi.push_str(&link);
    }
    page_navi.push_str("</p>");

    let body = format!(
        r#"<!DOCTYPE html>
<html lang="en">
  <head>
    <title>(Privileged) Picture History</title>
    <style>
      figure.pic {{
        display: inline-block;
        margin: 5px;
      }}
      figcaption.pic {{
        font-size: 100%;
        text-align: center;
      }}
    </style>
  </head>
  <body>
    <h1>(Privileged) Picture History</h1>
      {}

{}

    <h2>Navigation</h2>
    <p><a href="./camera/">Camera Main Page</a></p>
  </body>
</html>
"#,
        page_navi, html
    );

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
