use super::SystemModule;
use crate::sys::config;
use crate::sys::taskserver::Control;
use anyhow::{anyhow, bail, ensure, Result};
use chrono::{Local, NaiveTime};
use image::ImageOutputFormat;
use log::{info, warn};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    io::{Seek, Write},
    os::linux::fs::MetadataExt,
    path::{Path, PathBuf},
};
use tokio::{fs::File, io::AsyncWriteExt, process::Command, sync::Mutex};

const THUMB_POSTFIX: &str = "thumb";
/// 60 * 24 = 1440 /day
const HISTORY_QUEUE_SIZE: usize = 60 * 1024 * 2;

#[derive(Clone, Serialize, Deserialize)]
pub struct CameraConfig {
    enabled: bool,
    debug_exec_once: bool,
    fake_camera: bool,
    pic_history_dir: String,
    pic_archive_dir: String,
    total_size_limit_mb: u32,
    pub page_by: u32,
}

#[derive(Clone)]
pub struct PicEntry {
    pub path_main: PathBuf,
    pub path_th: PathBuf,
    pub total_size: u64,
}

type PicDict = BTreeMap<String, PicEntry>;
/// ストレージ上の全データを管理する
struct Storage {
    pic_history_list: PicDict,
    pic_archive_list: PicDict,
}

pub struct Camera {
    pub config: CameraConfig,
    wakeup_list: Vec<NaiveTime>,

    storage: Storage,
}

impl Camera {
    pub fn new(wakeup_list: Vec<NaiveTime>) -> Result<Self> {
        info!("[camera] initialize");

        let jsobj =
            config::get_object(&["camera"]).map_or(Err(anyhow!("Config not found: camera")), Ok)?;
        let config: CameraConfig = serde_json::from_value(jsobj)?;
        ensure!(config.page_by > 0);

        let pic_history_list = init_pics(&config.pic_history_dir)?;
        let pic_archive_list = init_pics(&config.pic_archive_dir)?;

        Ok(Camera {
            config,
            wakeup_list,
            storage: Storage {
                pic_history_list,
                pic_archive_list,
            },
        })
    }

    pub fn pic_list(&self) -> (&PicDict, &PicDict) {
        (
            &self.storage.pic_history_list,
            &self.storage.pic_archive_list,
        )
    }

    pub async fn push_pic_history(&mut self, img: &[u8], thumb: &[u8]) -> Result<()> {
        let now = Local::now();
        let dtstr = now.format("%Y%m%d_%H%M%S").to_string();
        let total_size = img.len() + thumb.len();
        let total_size = total_size as u64;

        let root = Path::new(&self.config.pic_history_dir);
        let mut path_main = PathBuf::from(root);
        path_main.push(&dtstr);
        path_main.set_extension("jpg");
        let mut path_th = PathBuf::from(root);
        path_th.push(format!("{}_{}.jpg", dtstr, THUMB_POSTFIX));
        path_th.set_extension("jpg");

        info!("write {}", path_main.display());
        let mut file = File::create(&path_main).await?;
        file.write_all(img).await?;
        drop(file);

        info!("write {}", path_th.display());
        let mut file = File::create(&path_th).await?;
        file.write_all(thumb).await?;
        drop(file);

        let entry = PicEntry {
            path_main,
            path_th,
            total_size,
        };
        if let Some(old) = self.storage.pic_history_list.insert(dtstr, entry) {
            warn!("duplicate picture: {}", old.path_main.display());
        }

        Ok(())
    }

    async fn check_task_entry(ctrl: Control) -> Result<()> {
        Ok(())
    }
}

impl SystemModule for Camera {
    fn on_start(&self, ctrl: &Control) {
        info!("[camera] on_start");
        if self.config.enabled {
            if self.config.debug_exec_once {
                ctrl.spawn_oneshot_task("health-check", Camera::check_task_entry);
            } else {
                ctrl.spawn_periodic_task(
                    "health-check",
                    &self.wakeup_list,
                    Camera::check_task_entry,
                );
            }
        }
    }
}

/// 検索ルートディレクトリ内から jpg ファイルを検索して [PicDict] を構築する。
///
/// ルートディレクトリが存在しない場合は作成する。
fn init_pics(dir: &str) -> Result<PicDict> {
    let root = Path::new(dir);
    if !root.try_exists()? {
        warn!("create dir: {}", root.to_string_lossy());
        std::fs::create_dir_all(root)?;
    }

    let mut result = PicDict::new();
    result = find_files_rec(result, root)?;
    info!("find {} files in {}", result.len(), dir);

    Ok(result)
}

/// [init_pics] 用の再帰関数。
fn find_files_rec(mut dict: PicDict, path: &Path) -> Result<PicDict> {
    if path.is_file() {
        // 拡張子が jpg でないものは無視
        if path.extension().unwrap_or_default() != "jpg" {
            return Ok(dict);
        }
        // 拡張子を除いた部分が空文字列およびサムネイルの場合は無視
        let name = path.file_stem().unwrap_or_default().to_string_lossy();
        if name.is_empty() || name.ends_with(THUMB_POSTFIX) {
            return Ok(dict);
        }

        // サムネイル画像のパスを生成
        let mut path_th = PathBuf::from(path);
        path_th.set_file_name(format!("{}_{}", name, THUMB_POSTFIX));
        path_th.set_extension("jpg");

        // サイズ取得
        let size = std::fs::metadata(path)?.st_size();
        let size_th = std::fs::metadata(&path_th).map_or(0, |m| m.st_size());
        let total_size = size + size_th;

        // PicEntry を生成して結果に追加
        let entry = PicEntry {
            path_main: PathBuf::from(path),
            path_th,
            total_size,
        };
        if let Some(old) = dict.insert(name.to_string(), entry) {
            warn!(
                "duplicate picture: {}, {}",
                old.path_main.display(),
                path.display()
            );
        }
    } else if path.is_dir() {
        for entry in std::fs::read_dir(path)? {
            let entry = entry?;
            dict = find_files_rec(dict, &entry.path())?;
        }
    }

    Ok(dict)
}

const PIC_MAX_W: u32 = 3280;
const PIC_MAX_H: u32 = 2464;
const PIC_MIN_W: u32 = 32;
const PIC_MIN_H: u32 = 24;
const PIC_DEF_W: u32 = PIC_MAX_W;
const PIC_DEF_H: u32 = PIC_MAX_H;
const PIC_MAX_Q: u32 = 100;
const PIC_MIN_Q: u32 = 0;
const PIC_DEF_Q: u32 = 85;
const PIC_DEF_TO_MS: u32 = 1000;
const THUMB_W: u32 = 128;
const THUMB_H: u32 = 96;
const THUMB_Q: u32 = 35;

pub struct TakePicOption {
    w: u32,
    h: u32,
    q: u32,
    timeout_ms: u32,
}

impl TakePicOption {
    pub fn new() -> Self {
        Self {
            w: PIC_DEF_W,
            h: PIC_DEF_H,
            q: PIC_DEF_Q,
            timeout_ms: PIC_DEF_TO_MS,
        }
    }
    pub fn width(mut self, w: u32) -> Self {
        assert!((PIC_MIN_W..=PIC_MAX_W).contains(&w));
        self.w = w;
        self
    }
    pub fn height(mut self, h: u32) -> Self {
        assert!((PIC_MIN_H..=PIC_MAX_H).contains(&h));
        self.h = h;
        self
    }
    pub fn quality(mut self, q: u32) -> Self {
        assert!((PIC_MIN_Q..=PIC_MAX_Q).contains(&q));
        self.q = q;
        self
    }
    pub fn timeout_ms(mut self, timeout_ms: u32) -> Self {
        self.timeout_ms = timeout_ms;
        self
    }
}

pub async fn take_a_pic(opt: TakePicOption) -> Result<Vec<u8>> {
    // 他の関数でも raspistill を使う場合外に出す
    static LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

    let fake = config::get_bool(&["camera", "fake_camera"])?;

    let bin = if !fake {
        let lock = LOCK.lock().await;
        let output = Command::new("raspistill")
            .arg("-o")
            .arg("-")
            .arg("-t")
            .arg(opt.timeout_ms.to_string())
            .arg("-q")
            .arg(opt.q.to_string())
            .arg("-w")
            .arg(opt.w.to_string())
            .arg("-h")
            .arg(opt.h.to_string())
            .output()
            .await?;
        if !output.status.success() {
            bail!("raspistill failed: {}", output.status);
        }

        output.stdout
        // unlock
    } else {
        // バイナリ同梱のデフォルト画像が撮れたことにする
        include_bytes!("../res/camera_def.jpg").to_vec()
    };
    // raspistill は同時に複数プロセス起動できないので mutex で保護する

    Ok(bin)
}

pub fn create_thumbnail<W>(w: &mut W, src_buf: &[u8]) -> Result<()>
where
    W: Write + Seek,
{
    let src = image::load_from_memory_with_format(src_buf, image::ImageFormat::Jpeg)?;
    let dst = src.thumbnail(THUMB_W, THUMB_H);
    dst.write_to(w, ImageOutputFormat::Jpeg(85))?;

    Ok(())
}
