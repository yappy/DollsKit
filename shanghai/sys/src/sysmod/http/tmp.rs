//! HTTP サーバーから一時ファイルを公開する。

use actix_web::{HttpResponse, Responder, http::header::ContentType, web};

#[actix_web::get("/tmp/{id}")]
async fn index_get(path: web::Path<(String,)>) -> impl Responder {
    let id: String = path.into_inner().0;
    log::info!("tmp request: {}", id);

    HttpResponse::NotFound()
        .content_type(ContentType::plaintext())
        .body(id)
}
