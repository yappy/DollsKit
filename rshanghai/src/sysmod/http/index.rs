use actix_web::{http::header::ContentType, web, HttpResponse, Responder};

use super::HttpConfig;
use crate::sys::version;

#[actix_web::get("/")]
pub(super) async fn index_get(cfg: web::Data<HttpConfig>) -> impl Responder {
    let verstr = version::VERSION_INFO_VEC
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
    <h1>System Available</h1>
    TODO
    <hr>
    <ul>
{}
    </ul>
  </body>
</html>
"#,
        verstr
    );
    HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(body)
}
