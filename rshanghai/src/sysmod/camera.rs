use super::SystemModule;
use crate::sys::config;
use crate::sys::taskserver::Control;
use anyhow::{anyhow, bail, Ok, Result};
use chrono::NaiveTime;
use image::ImageOutputFormat;
use log::{info, warn};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    io::{Seek, Write},
    path::{Path, PathBuf},
};
use tokio::{process::Command, sync::Mutex};

/// 60 * 24 = 1440 /day
const HISTORY_QUEUE_SIZE: usize = 60 * 1024 * 2;

#[derive(Clone, Serialize, Deserialize)]
struct CameraConfig {
    enabled: bool,
    debug_exec_once: bool,
    fake_camera: bool,
    pic_tmp_dir: String,
    pic_save_dir: String,
    pic_del_days: i32,
}

struct PicEntry {
    path: PathBuf,
    //thumb_path: PathBuf,
    // timestamp
}

type PicDict = BTreeMap<String, PicEntry>;
/// ストレージ上の全データを管理する
struct Storage {
    pic_tmp_list: PicDict,
    pic_save_list: PicDict,
}

pub struct Camera {
    config: CameraConfig,
    wakeup_list: Vec<NaiveTime>,

    storage: Storage,
}

impl Camera {
    pub fn new(wakeup_list: Vec<NaiveTime>) -> Result<Self> {
        info!("[camera] initialize");

        let jsobj =
            config::get_object(&["camera"]).map_or(Err(anyhow!("Config not found: camera")), Ok)?;
        let config: CameraConfig = serde_json::from_value(jsobj)?;

        let pic_tmp_list = init_pics(&config.pic_tmp_dir)?;
        let pic_save_list = init_pics(&config.pic_save_dir)?;

        Ok(Camera {
            config,
            wakeup_list,
            storage: Storage {
                pic_tmp_list,
                pic_save_list,
            },
        })
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
        if path.extension().unwrap_or_default() != "jpg" {
            return Ok(dict);
        }
        let name = path.file_stem().unwrap_or_default().to_string_lossy();
        let entry = PicEntry {
            path: PathBuf::from(path),
        };
        if let Some(old) = dict.insert(name.to_string(), entry) {
            warn!(
                "duplicate picture: {}, {}",
                old.path.display(),
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
const THUMB_W: u32 = 64;
const THUMB_H: u32 = 48;
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
