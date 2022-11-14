use super::HttpConfig;
use crate::sys::taskserver::Control;
use actix_web::{http::header::ContentType, web, HttpResponse, Responder};
use log::error;
use tokio::process::Command;

const PIC_MAX_W: u32 = 3280;
const PIC_MAX_H: u32 = 2464;
const PIC_MIN_W: u32 = 32;
const PIC_MIN_H: u32 = 24;
const PIC_DEF_W: u32 = PIC_MAX_W;
const PIC_DEF_H: u32 = PIC_MAX_H;
const PIC_TIMEOUT_SEC: u32 = 1;

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
async fn camera_get(cfg: web::Data<HttpConfig>, ctrl: web::Data<Control>) -> impl Responder {
    let output = Command::new("raspistill")
        .arg("-o")
        .arg("-")
        .arg("-t")
        .arg(PIC_TIMEOUT_SEC.to_string())
        .arg("-w")
        .arg(PIC_DEF_W.to_string())
        .arg("-h")
        .arg(PIC_DEF_H.to_string())
        .output()
        .await;

    match output {
        Ok(output) => {
            if output.status.success() {
                error!("raspistill take a picture");
                HttpResponse::Ok()
                    .content_type(ContentType::jpeg())
                    .body(output.stdout)
            } else {
                error!("raspistill error: {:?}", output.status.code());
                error!("{}", String::from_utf8_lossy(&output.stderr));
                HttpResponse::InternalServerError()
                    .content_type(ContentType::plaintext())
                    .body("Camera error.")
            }
        }
        Err(e) => {
            error!("raspistill error: {:?}", e);
            HttpResponse::InternalServerError()
                .content_type(ContentType::plaintext())
                .body("Camera error.")
        }
    }
}
