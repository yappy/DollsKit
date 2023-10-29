//! Uploader.
//!
//! ファイル名のルールは [[check_file_name]] を参照。

use super::{ActixError, WebResult};
use crate::sys::taskserver::Control;
use actix_multipart::{Multipart, MultipartError};
use actix_web::{http::header::ContentType, web, HttpResponse, Responder};
use anyhow::{anyhow, ensure, Context, Result};
use log::{error, info, trace, warn};
use std::path::Path;
use tokio::{fs::File, io::AsyncWriteExt, process::Command};
use tokio_stream::StreamExt;

const FILE_NAME_MAX_LEN: usize = 32;
const TMP_FILE_NAME: &str = "upload.tmp~";
// TODO: config file
const UPLOAD_FILE_LIMIT_MB: usize = 4 << 30;
const UPLOAD_TOTAL_LIMIT_MB: usize = 32 << 30;

/// テンポラリファイルに書き込めるのは一度に一人だけ。
///
/// アップロード完了までには時間がかかるのでロック中に await 可能な Mutex とする。
static FS_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

#[actix_web::get("/upload/")]
async fn index_get() -> impl Responder {
    info!("GET /upload/");
    let body = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/res/http/upload/index.html"
    ));

    HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(body)
}

#[actix_web::post("/upload/")]
async fn index_post(mut payload: Multipart, ctrl: web::Data<Control>) -> WebResult {
    let res = index_post_main(&mut payload, ctrl).await;

    // finally
    loop {
        match payload.try_next().await {
            Ok(res) => {
                // 読み捨てる、None で完了
                if res.is_none() {
                    break;
                }
            }
            Err(e) => {
                warn!("Upload: Error while drain: {e}");
                break;
            }
        }
    }

    res
}

/// <https://github.com/actix/examples/tree/master/forms/multipart>
async fn index_post_main(payload: &mut Multipart, ctrl: web::Data<Control>) -> WebResult {
    info!("POST /upload/");

    let (dir, _flimit, _tlimit) = {
        let http = ctrl.sysmods().http.lock().await;
        let config = &http.config;
        (
            config.upload_dir.clone(),
            UPLOAD_FILE_LIMIT_MB,
            UPLOAD_TOTAL_LIMIT_MB,
        )
    };
    let dir = Path::new(&dir);
    let tmppath = dir.join(TMP_FILE_NAME);

    // tempfile の使用権を取得する
    // 取れない場合は 503 System Unavailable
    let fs_lock = match FS_LOCK.try_lock() {
        Ok(lock) => lock,
        Err(_) => return Err(ActixError::new("Upload is busy", 503)),
    };

    // ディレクトリが無ければ作成
    info!("Upload: create all: {}", dir.to_string_lossy());
    std::fs::create_dir_all(dir).context("Failed to create upload dir")?;

    // multipart/form-data のパース
    if let Some(mut field) = conv_mperror(payload.try_next().await)? {
        info!("Upload: multipart/form-data entry");

        // content_disposition からファイル名を取得、チェック
        let cont_disp = field.content_disposition();
        let fname = cont_disp.get_filename();
        let fname = check_file_name(fname)?;
        info!("Upload: filename: {fname}");
        let dstpath = dir.join(fname);

        // tempfile 作成
        info!("Upload: create: {}", tmppath.to_string_lossy());
        let mut tmpf = File::create(&tmppath)
            .await
            .context("Failed to create temp file")?;

        // ファイルデータ本体
        let mut total = 0;
        while let Some(chunk) = conv_mperror(field.try_next().await)? {
            tmpf.write(&chunk).await.context("Write error")?;
            total += chunk.len();
            trace!("{total} B received");
            if total > UPLOAD_FILE_LIMIT_MB << 20 {
                return Err(ActixError::new("File size is too large", 413));
            }
        }
        info!("{total} B received");

        let dirsize = get_disk_usage(&dir.to_string_lossy()).await?;
        info!("Upload: du: {dirsize}");

        if dirsize <= UPLOAD_TOTAL_LIMIT_MB << 20 {
            // リネーム
            info!(
                "Upload: rename from {} to {}",
                tmppath.to_string_lossy(),
                dstpath.to_string_lossy()
            );
            tokio::fs::rename(&tmppath, &dstpath)
                .await
                .context("Rename failed")?;
        } else {
            // トータルサイズオーバーなので削除
            error!("Upload: total size over");
            info!("Upload: remove {}", tmppath.to_string_lossy());
            tokio::fs::remove_file(&tmppath)
                .await
                .context("Remove failed")?;
            return Err(ActixError::new("Insufficient storage", 507));
        }

        // close
    } else {
        return Err(ActixError::new("File data required", 400));
    }

    // ファイルシステムアンロック
    drop(fs_lock);

    Ok(HttpResponse::Ok()
        .content_type(ContentType::plaintext())
        .body(""))
}

/// MultipartError に [Send] が実装されていないからか自動変換が効かない。
/// 文字列データを取り出して [anyhow::Error] 型に変換する。
fn conv_mperror<T>(res: Result<T, MultipartError>) -> Result<T, anyhow::Error> {
    match res {
        Ok(x) => Ok(x),
        Err(e) => Err(anyhow!(e.to_string())),
    }
}

/// ファイル名をチェックする。
///
/// * None および空文字列は NG
/// * [FILE_NAME_MAX_LEN] 文字まで
/// * 半角英数字およびドット、ハイフン、アンダースコアのみ
/// * ドットで始まらない (隠しファイルや `.` `..` などで何かが起こらないようにする)
fn check_file_name(name: Option<&str>) -> Result<&str, ActixError> {
    let name = name.unwrap_or("");
    if name.is_empty() {
        Err(ActixError::new("No file name", 400))
    } else if name.len() > FILE_NAME_MAX_LEN {
        Err(ActixError::new("File name too long", 400))
    } else if name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_')
        && !name.starts_with('.')
    {
        Ok(name)
    } else {
        Err(ActixError::new("Invalid file name", 400))
    }
}

async fn get_disk_usage(dirpath: &str) -> Result<usize> {
    let mut cmd = Command::new(format!("du -s -B 1 {dirpath}"));
    let output = cmd.output().await?;
    ensure!(output.status.success(), "du command failed");

    // example: "2903404544      ."
    let stdout = String::from_utf8_lossy(&output.stdout);
    let token = stdout
        .split_ascii_whitespace()
        .next()
        .context("du parse error")?;
    let size = token.parse().context("du parse error")?;

    Ok(size)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_file_name_ok() {
        assert!(check_file_name(Some("ok.txt")).is_ok());
    }

    #[test]
    fn check_file_name_empty() {
        assert!(check_file_name(None).is_err());
        assert!(check_file_name(Some("")).is_err());
    }

    #[test]
    fn check_file_name_long() {
        let long_name = "012345678901234567890123456789.txt";
        assert!(check_file_name(Some(long_name)).is_err());
    }

    #[test]
    fn check_file_name_invalid() {
        assert!(check_file_name(Some("あ.txt")).is_err());
    }

    #[test]
    fn check_file_name_tmpfile() {
        assert!(check_file_name(Some(TMP_FILE_NAME)).is_err());
    }
}
