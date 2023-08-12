//! Uploader.
//!
//! FileName := [a-zA-Z0-9.-_]+

use std::path::Path;

use super::{ActixError, WebResult};
use crate::sys::taskserver::Control;
use actix_multipart::form::{tempfile::TempFile, MultipartForm};
use actix_web::{http::header::ContentType, web, HttpResponse, Responder};
use anyhow::Context;
use log::info;

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
async fn index_post(
    MultipartForm(form): MultipartForm<UploadForm>,
    ctrl: web::Data<Control>,
) -> WebResult {
    info!("POST /upload/ ({} files)", form.files.len());

    let dir = ctrl.sysmods().http.lock().await.config.upload_dir.clone();
    let dir = Path::new(&dir);
    info!("Upload: create {}", dir.to_string_lossy());
    std::fs::create_dir_all(dir).context("")?;

    for f in form.files {
        info!(
            "Upload: {:?} {:?} size={}",
            f.content_type, f.file_name, f.size
        );
        let file_name = check_file_name(f.file_name)?;
        let dst = dir.join(file_name);
        info!("Upload: saving to {}", dst.to_string_lossy());
        f.file.persist(dst).context("Temp file error")?;
    }

    Ok(HttpResponse::Ok()
        .content_type(ContentType::plaintext())
        .body(""))
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
