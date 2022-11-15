use super::{HttpConfig, WebResult};
use crate::{
    sys::taskserver::Control,
    sysmod::camera::{take_a_pic, TakePicOption},
};
use actix_web::{http::header::ContentType, web, HttpResponse, Responder};
use log::error;

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

#[actix_web::get("/camera/take")]
async fn camera_get(cfg: web::Data<HttpConfig>, ctrl: web::Data<Control>) -> WebResult {
    let pic_bin = take_a_pic(TakePicOption::new()).await;
    if let Err(ref e) = pic_bin {
        error!("take a picture error");
        error!("{:#}", e);
    }

    let resp = HttpResponse::Ok()
        .content_type(ContentType::jpeg())
        .body(pic_bin?);
    Ok(resp)
}
