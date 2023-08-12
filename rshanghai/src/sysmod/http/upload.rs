//! Uploader.
//!
//! FileName := [a-zA-Z0-9.-_]+

use std::path::Path;

use super::{ActixError, WebResult};
use crate::sys::taskserver::Control;
use actix_multipart::{
    form::{tempfile::TempFile, MultipartForm},
    Multipart, MultipartError,
};
use actix_web::{http::header::ContentType, web, HttpResponse, Responder};
use anyhow::{anyhow, Context};
use log::info;
use tokio_stream::StreamExt;

const FILE_NAME_MAX_LEN: usize = 32;

#[actix_web::get("/upload/")]
async fn index_get() -> impl Responder {
    info!("GET /upload/");
    let body = include_str!("../../res/http/upload/index.html");

    HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(body)
}

#[derive(Debug, MultipartForm)]
struct UploadForm {
    #[multipart(rename = "file_content")]
    files: Vec<TempFile>,
}

/// <https://github.com/actix/examples/tree/master/forms/multipart>
#[actix_web::post("/upload/")]
async fn index_post(mut payload: Multipart, ctrl: web::Data<Control>) -> WebResult {
    info!("POST /upload/");

    let dir = ctrl.sysmods().http.lock().await.config.upload_dir.clone();
    let dir = Path::new(&dir);
    info!("Upload: create {}", dir.to_string_lossy());
    std::fs::create_dir_all(dir).context("")?;

    let res = index_post_internal(payload).await;
    match res {
        Ok(()) => Ok(HttpResponse::Ok()
            .content_type(ContentType::plaintext())
            .body("")),
        Err(e) => Err(ActixError::from(anyhow!(e.to_string()))),
    }
}

async fn index_post_internal(mut payload: Multipart) -> Result<(), MultipartError> {
    // iterate over multipart stream
    while let Some(mut field) = payload.try_next().await? {
        // A multipart/form-data stream has to contain `content_disposition`
        let content_disposition = field.content_disposition();
        let filename = content_disposition.get_filename();
        info!("filename: {:?}", filename);

        // Field in turn is stream of *Bytes* object
        let mut total = 0;
        while let Some(chunk) = field.try_next().await? {
            total += chunk.len();
            info!("{total} B received");
        }
    }
    Ok(())
}

fn check_file_name(name: Option<String>) -> Result<String, ActixError> {
    let name = name.unwrap_or("".to_string());
    if name.is_empty() {
        Err(ActixError::new("No file name", 400))
    } else if name.len() > FILE_NAME_MAX_LEN {
        Err(ActixError::new("File name too long", 400))
    } else if name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_')
    {
        Ok(name)
    } else {
        Err(ActixError::new("Invalid file name", 400))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_file_name_ok() {
        assert!(check_file_name(Some("ok.txt".to_string())).is_ok());
    }

    #[test]
    fn check_file_name_empty() {
        assert!(check_file_name(None).is_err());
        assert!(check_file_name(Some("".to_string())).is_err());
    }

    #[test]
    fn check_file_name_long() {
        let long_name = "012345678901234567890123456789.txt".to_string();
        assert!(check_file_name(Some(long_name)).is_err());
    }

    #[test]
    fn check_file_name_invalid() {
        assert!(check_file_name(Some("あ.txt".to_string())).is_err());
    }
}
