use super::{HttpConfig, WebResult};
use crate::{
    sys::taskserver::Control,
    sysmod::camera::{create_thumbnail, take_a_pic, PicEntry, TakePicOption},
    sysmod::http::simple_error,
};
use actix_web::{http::header::ContentType, web, HttpResponse, Responder};
use anyhow::anyhow;
use log::error;
use reqwest::StatusCode;
use std::{cmp, collections::BTreeMap};
use tokio::{fs::File, io::AsyncReadExt};

/// GET /priv/camera/ Camera インデックスページ。
#[actix_web::get("/camera/")]
async fn index_get(cfg: web::Data<HttpConfig>, ctrl: web::Data<Control>) -> impl Responder {
    let body = r#"<!DOCTYPE html>
<html lang="en">
  <head>
    <title>(Privileged) Camera</title>
  </head>
  <body>
    <h1>(Privileged) Camera</h1>
    <form action="./take" method="post">
      <input type="submit" value="Take a picture!">
    </form>
    <p><a href="./history/">Picture List</a></p>
    <p><a href="./archive/">Archive</a></p>
    <h2>Navigation</h2>
    <p><a href="../">Main Page</a></p>
  </body>
</html>
"#;

    HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(body)
}

/// GET /priv/camera/history/ 写真一覧。
#[actix_web::get("/camera/history/")]
async fn history_get(ctrl: web::Data<Control>) -> HttpResponse {
    history_get_internal(ctrl, 0).await
}

/// GET /priv/camera/history/{start} 写真一覧 (開始番号指定)。
#[actix_web::get("/camera/history/{start}")]
async fn history_start_get(ctrl: web::Data<Control>, path: web::Path<String>) -> HttpResponse {
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

    history_get_internal(ctrl, start).await
}

/// GET /priv/camera/archive/ アーカイブ済み写真一覧。
#[actix_web::get("/camera/archive/")]
async fn archive_get(ctrl: web::Data<Control>) -> HttpResponse {
    archive_get_internal(ctrl, 0).await
}

/// GET /priv/camera/archive/{start} アーカイブ済み写真一覧 (開始番号指定)。
#[actix_web::get("/camera/archive/{start}")]
async fn archive_start_get(ctrl: web::Data<Control>, path: web::Path<String>) -> HttpResponse {
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

    archive_get_internal(ctrl, start).await
}

/// history_get シリーズの共通ルーチン。
async fn history_get_internal(ctrl: web::Data<Control>, start: usize) -> HttpResponse {
    let html = {
        let camera = ctrl.sysmods().camera.lock().await;
        let page_by = camera.config.page_by as usize;
        let (hist, _) = camera.pic_list();

        create_pic_list_page(hist, start, page_by, "(Privileged) Picture History")
    };

    HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(html)
}

/// archive_get シリーズの共通ルーチン。
async fn archive_get_internal(ctrl: web::Data<Control>, start: usize) -> HttpResponse {
    let html = {
        let camera = ctrl.sysmods().camera.lock().await;
        let page_by = camera.config.page_by as usize;
        let (_, archive) = camera.pic_list();

        create_pic_list_page(archive, start, page_by, "(Privileged) Picture Archive")
    };

    HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(html)
}

fn create_pic_list_page(
    pic_list: &BTreeMap<String, PicEntry>,
    start: usize,
    page_by: usize,
    title: &str,
) -> String {
    let total = pic_list.len();
    let data: Vec<_> = pic_list.keys().rev().skip(start).take(page_by).collect();

    let mut fig_list = String::new();
    for name in data {
        fig_list.push_str(&format!(
            r#"    <figure class="pic">
      <a href="../pic/history/{0}/main"><img src="../pic/history/{0}/thumb" alt="{0}"></a>
      <figcaption class="pic">{0}</figcaption>
    </figure>
"#,
            name
        ));
    }

    let mut page_navi = String::new();
    page_navi.push_str("<p>");
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

    format!(
        r#"<!DOCTYPE html>
<html lang="en">
  <head>
    <title>{title}</title>
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
    <h1>{title}</h1>
      {page_navi}

{fig_list}

    <h2>Navigation</h2>
    <p><a href="./camera/">Camera Main Page</a></p>
  </body>
</html>
"#,
        title = title,
        page_navi = page_navi,
        fig_list = fig_list
    )
}

/// GET /priv/camera/pic/history/{name}/{kind}
/// 写真取得エンドポイント。
///
/// image/jpeg を返す。
#[actix_web::get("/camera/pic/history/{name}/{kind}")]
async fn pic_history_get(
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

#[actix_web::post("/camera/take")]
async fn take_post(cfg: web::Data<HttpConfig>, ctrl: web::Data<Control>) -> WebResult {
    let pic = take_a_pic(TakePicOption::new()).await;
    if let Err(ref e) = pic {
        error!("take a picture error");
        error!("{:#}", e);
    }
    let pic = pic?;

    let thumb = create_thumbnail(&pic)?;

    let mut camera = ctrl.sysmods().camera.lock().await;
    camera.push_pic_history(&pic, &thumb).await?;
    drop(camera);

    let resp = HttpResponse::Ok()
        .content_type(ContentType::jpeg())
        .body(pic);
    Ok(resp)
}
