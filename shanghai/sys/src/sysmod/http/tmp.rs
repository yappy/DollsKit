//! HTTP サーバーから一時ファイルを公開する。

use actix_web::{HttpResponse, Responder, http::header::ContentType, web};

use crate::taskserver::Control;

#[actix_web::get("/tmp/{id}")]
async fn index_get(ctrl: web::Data<Control>, path: web::Path<String>) -> impl Responder {
    let id = path.into_inner();

    {
        let http = ctrl.sysmods().http.lock().await;
        if let Some(elem) = http.tmp_data.iter().find(|elem| elem.id == id) {
            return HttpResponse::Ok()
                .content_type(elem.ctype.clone())
                .body(elem.data.clone());
        }
    }

    HttpResponse::NotFound()
        .content_type(ContentType::plaintext())
        .body(id)
}
