use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

use super::SystemModule;
use crate::sys::config;
use crate::sys::taskserver::Control;
use anyhow::{anyhow, Ok, Result};
use chrono::NaiveTime;
use log::{info, warn};
use serde::{Deserialize, Serialize};

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
            warn!("duplicate picture: {}, {}", old.path.display(), path.display());
        }
    } else if path.is_dir() {
        for entry in std::fs::read_dir(path)? {
            let entry = entry?;
            dict = find_files_rec(dict, &entry.path())?;
        }
    }

    Ok(dict)
}
