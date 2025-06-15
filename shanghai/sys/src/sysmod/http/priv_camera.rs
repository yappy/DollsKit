use super::{WebResult, error_resp};
use crate::sysmod::camera::{PicEntry, TakePicOption, create_thumbnail, take_a_pic};
use crate::sysmod::twitter::LIMIT_PHOTO_COUNT;
use crate::sysmod::{camera::resize, http::error_resp_msg, twitter::LIMIT_PHOTO_SIZE};
use crate::taskserver::Control;
use actix_web::{HttpResponse, Responder, http::header::ContentType, web};
use anyhow::{Result, anyhow, bail};
use log::error;
use serde::Deserialize;
use serenity::http::StatusCode;
use std::{cmp, collections::BTreeMap};
use tokio::{fs::File, io::AsyncReadExt};

/// GET /priv/camera/ Camera インデックスページ。
#[actix_web::get("/camera/")]
async fn index_get() -> impl Responder {
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
    <p><a href="./history">Picture List</a></p>
    <p><a href="./archive">Archive</a></p>
    <h2>Navigation</h2>
    <p><a href="../">Main Page</a></p>
  </body>
</html>
"#;

    HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(body)
}

#[derive(Deserialize)]
struct HistArGetQuery {
    #[serde(default)]
    page: usize,
}

/// GET /priv/camera/history/ 写真一覧。
#[actix_web::get("/camera/history")]
async fn history_get(ctrl: web::Data<Control>, query: web::Query<HistArGetQuery>) -> HttpResponse {
    let html = {
        let camera = ctrl.sysmods().camera.lock().await;
        let page_by = camera.config.page_by as usize;
        let (hist, _) = camera.pic_list();

        create_pic_list_page(
            hist,
            "history",
            query.page,
            page_by,
            "(Privileged) Picture History",
            &[("archive", "Archive"), ("delete", "Delete")],
        )
    };

    HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(html)
}

/// GET /priv/camera/archive/ アーカイブ済み写真一覧。
#[actix_web::get("/camera/archive")]
async fn archive_get(ctrl: web::Data<Control>, query: web::Query<HistArGetQuery>) -> HttpResponse {
    let html = {
        let camera = ctrl.sysmods().camera.lock().await;
        let page_by = camera.config.page_by as usize;
        let (_, archive) = camera.pic_list();

        create_pic_list_page(
            archive,
            "archive",
            query.page,
            page_by,
            "(Privileged) Picture Archive",
            &[
                ("twitter", "Post on Twitter (Max 4 pics or less)"),
                ("delete", "Delete"),
            ],
        )
    };

    HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(html)
}

/// history/archive 共用写真リスト HTML 生成。
///
/// * `pic_list` - 画像データ。
/// * `start` - pic_list の何番目から表示するか。
/// * `page_by` - start からいくつ画像を表示するか。
/// * `title` - タイトル。
/// * `commands` - POST の "cmd" パラメータで送られる値とラジオボタンに
///   添えるラベルからなるタプルの配列
fn create_pic_list_page(
    pic_list: &BTreeMap<String, PicEntry>,
    img_path_dir: &str,
    start: usize,
    page_by: usize,
    title: &str,
    commands: &[(&str, &str)],
) -> String {
    let total = pic_list.len();
    let data: Vec<_> = pic_list.keys().rev().skip(start).take(page_by).collect();

    let mut fig_list = String::new();
    fig_list += r#"    <form method="post">
      <fieldset>
        <legend>Commands for selected items</legend>
        <input type="submit" value="Execute">
"#;
    let mut is_first = true;
    for &(cmd, label) in commands {
        let checked = if is_first {
            is_first = false;
            r#" checked="checked""#
        } else {
            ""
        };
        fig_list += &format!(
            r#"        <label><input type="radio" name="cmd" value="{cmd}"{checked}>{label}</label>
"#
        );
    }
    fig_list += r#"      </fieldset>
      <p><input type="reset" value="Reset"></p>

"#;

    for name in data {
        fig_list += &format!(
            r#"      <input type="checkbox" id="{name}" name="target" value="{name}">
      <figure class="pic">
        <a href="./pic/{img_path_dir}/{name}/main"><img src="./pic/{img_path_dir}/{name}/thumb" alt="{name}"></a>
        <figcaption class="pic"><label for="{name}">{name}</label></figcaption>
      </figure>
"#
        );
    }
    fig_list += "    </form>";

    let mut page_navi = String::new();
    page_navi += &format!("<p>{} files</p>", pic_list.len());
    page_navi += "<p>";
    for left in (0..total).step_by(page_by) {
        let right = cmp::min(left + page_by - 1, total - 1);
        let link = if (left..=right).contains(&start) {
            format!(r#"{}-{} "#, left + 1, right + 1)
        } else {
            format!(
                r#"<a href="?page={}">{}-{}</a> "#,
                left,
                left + 1,
                right + 1
            )
        };
        page_navi += &link;
    }
    page_navi += "</p>";

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
    <p><a href="./">Camera Main Page</a></p>
  </body>
</html>
"#
    )
}

/// 同一キーはデシリアライズ対応していないので自力でパースする。
///
/// cmd=c&target=t1&target=t2&...
fn parse_cmd_targets(
    iter: impl IntoIterator<Item = (String, String)>,
) -> Result<(String, Vec<String>)> {
    let mut cmd = None;
    let mut targets = Vec::new();

    for (key, value) in iter {
        match key.as_str() {
            "cmd" => {
                cmd = Some(value);
            }
            "target" => {
                targets.push(value);
            }
            _ => {}
        }
    }

    cmd.map_or_else(|| Err(anyhow!("cmd is required")), |cmd| Ok((cmd, targets)))
}

/// POST /priv/camera/history
///
/// * `cmd` - "archive" or "delete"
/// * `target` - 対象の picture ID (複数回指定可)
#[actix_web::post("/camera/history")]
async fn history_post(
    ctrl: web::Data<Control>,
    form: web::Form<Vec<(String, String)>>,
) -> HttpResponse {
    let param = parse_cmd_targets(form.0);
    if let Err(why) = param {
        return HttpResponse::BadRequest()
            .content_type(ContentType::plaintext())
            .body(why.to_string());
    }
    let (cmd, targets) = param.unwrap();

    match cmd.as_str() {
        "archive" => {
            if let Err(why) = archive_pics(&ctrl, &targets).await {
                if why.is::<std::io::Error>() {
                    HttpResponse::InternalServerError()
                        .content_type(ContentType::plaintext())
                        .body(why.to_string())
                } else {
                    HttpResponse::BadRequest()
                        .content_type(ContentType::plaintext())
                        .body(why.to_string())
                }
            } else {
                // 成功したら archive GET へリダイレクト
                HttpResponse::SeeOther()
                    .append_header(("LOCATION", "./archive"))
                    .finish()
            }
        }
        "delete" => {
            if let Err(why) = delete_history_pics(&ctrl, &targets).await {
                HttpResponse::BadRequest()
                    .content_type(ContentType::plaintext())
                    .body(why.to_string())
            } else {
                // 成功したら history GET へリダイレクト
                HttpResponse::SeeOther()
                    .append_header(("LOCATION", "./history"))
                    .finish()
            }
        }
        _ => HttpResponse::BadRequest()
            .content_type(ContentType::plaintext())
            .body("invalid command"),
    }
}

/// POST /priv/camera/archive
///
/// * `cmd` - "twitter" or "delete"
/// * `target` - 対象の picture ID (複数回指定可)
#[actix_web::post("/camera/archive")]
async fn archive_post(
    ctrl: web::Data<Control>,
    form: web::Form<Vec<(String, String)>>,
) -> HttpResponse {
    let param = parse_cmd_targets(form.0);
    if let Err(why) = param {
        return HttpResponse::BadRequest()
            .content_type(ContentType::plaintext())
            .body(why.to_string());
    }
    let (cmd, targets) = param.unwrap();

    match cmd.as_str() {
        "twitter" => {
            if targets.is_empty() || targets.len() > LIMIT_PHOTO_COUNT {
                error_resp_msg(StatusCode::BAD_REQUEST, "invalid pic count")
            } else if let Err(why) = twitter_post(&ctrl, &targets).await {
                error_resp_msg(StatusCode::INTERNAL_SERVER_ERROR, &why.to_string())
            } else {
                // 成功したら archive GET へリダイレクト
                HttpResponse::SeeOther()
                    .append_header(("LOCATION", "./history"))
                    .finish()
            }
        }
        "delete" => {
            if let Err(why) = delete_archive_pics(&ctrl, &targets).await {
                error_resp_msg(StatusCode::BAD_REQUEST, &why.to_string())
            } else {
                // 成功したら archive GET へリダイレクト
                HttpResponse::SeeOther()
                    .append_header(("LOCATION", "./history"))
                    .finish()
            }
        }
        _ => HttpResponse::BadRequest()
            .content_type(ContentType::plaintext())
            .body("invalid command"),
    }
}

async fn archive_pics(ctrl: &Control, ids: &[String]) -> Result<()> {
    let mut camera = ctrl.sysmods().camera.lock().await;
    for id in ids {
        camera.push_pic_archive(id).await?;
    }

    Ok(())
}

async fn delete_history_pics(ctrl: &Control, ids: &[String]) -> Result<()> {
    let mut camera = ctrl.sysmods().camera.lock().await;
    for id in ids {
        camera.delete_pic_history(id).await?;
    }

    Ok(())
}

async fn delete_archive_pics(ctrl: &Control, ids: &[String]) -> Result<()> {
    let mut camera = ctrl.sysmods().camera.lock().await;
    for id in ids {
        camera.delete_pic_archive(id).await?;
    }

    Ok(())
}

async fn twitter_post(ctrl: &Control, ids: &[String]) -> Result<()> {
    assert!(ids.len() <= LIMIT_PHOTO_COUNT);

    let mut binlist = Vec::new();
    {
        let camera = ctrl.sysmods().camera.lock().await;

        let (_, archive) = camera.pic_list();
        for id in ids {
            if let Some(entry) = archive.get(id) {
                let mut file = File::open(&entry.path_main).await?;
                let mut bin = Vec::new();
                let _ = file.read_to_end(&mut bin).await?;
                binlist.push(bin);
            } else {
                bail!("ID not found");
            }
        }
    }
    let mut resized_list = Vec::new();
    for bin in binlist {
        let mut w = 1280_u32;
        let mut h = 720_u32;
        let mut resized = bin;
        while resized.len() > LIMIT_PHOTO_SIZE {
            resized = resize(&resized, w, h)?;
            w /= 2;
            h /= 2;
        }
        resized_list.push(resized);
    }
    {
        let mut tw = ctrl.sysmods().twitter.lock().await;

        let mut midlist: Vec<u64> = Vec::new();
        for bin in resized_list {
            let media_id = tw.media_upload(bin).await?;
            midlist.push(media_id);
        }
        let text = ids.join("\n");
        tw.tweet_custom(&text, None, &midlist).await?;
    }

    Ok(())
}

/// GET /priv/camera/pic/history/{name}/{kind}
/// 写真取得エンドポイント。
///
/// image/jpeg を返す。
#[actix_web::get("/camera/pic/history/{name}/{kind}")]
async fn pic_history_get(ctrl: web::Data<Control>, path: web::Path<(String, String)>) -> WebResult {
    let (name, kind) = path.into_inner();

    pic_get_internal(&ctrl, StorageType::History, &name, &kind).await
}

/// GET /priv/camera/pic/history/{name}/{kind}
/// 写真取得エンドポイント。
///
/// image/jpeg を返す。
#[actix_web::get("/camera/pic/archive/{name}/{kind}")]
async fn pic_archive_get(ctrl: web::Data<Control>, path: web::Path<(String, String)>) -> WebResult {
    let (name, kind) = path.into_inner();

    pic_get_internal(&ctrl, StorageType::Archive, &name, &kind).await
}

enum StorageType {
    History,
    Archive,
}

async fn pic_get_internal(ctrl: &Control, stype: StorageType, name: &str, kind: &str) -> WebResult {
    let is_th = match kind {
        "main" => false,
        "thumb" => true,
        _ => return Ok(error_resp(StatusCode::BAD_REQUEST)),
    };

    let value = {
        let camera = ctrl.sysmods().camera.lock().await;
        let (hist, ar) = camera.pic_list();
        let dict = match stype {
            StorageType::History => hist,
            StorageType::Archive => ar,
        };
        dict.get(name).cloned()
    };

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
        Ok(error_resp(StatusCode::NOT_FOUND))
    }
}

#[actix_web::post("/camera/take")]
async fn take_post(ctrl: web::Data<Control>) -> WebResult {
    let pic = take_a_pic(TakePicOption::new()).await;
    if let Err(ref e) = pic {
        error!("take a picture error");
        error!("{:#}", e);
    }
    let pic = pic?;

    /*
    // twitter upload test
    {
        let mini = create_thumbnail(&pic)?;
        let tw = ctrl.sysmods().twitter.lock().await;
        let id = tw.media_upload(mini).await;
        info!("{id:?}");
        id?;
    }
     */

    let thumb = create_thumbnail(&pic)?;

    let mut camera = ctrl.sysmods().camera.lock().await;
    camera.push_pic_history(&pic, &thumb).await?;
    drop(camera);

    let resp = HttpResponse::Ok()
        .content_type(ContentType::jpeg())
        .body(pic);
    Ok(resp)
}
