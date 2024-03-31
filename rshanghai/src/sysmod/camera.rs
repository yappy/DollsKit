//! Raspberry Pi カメラ機能。
//!
//! 専用カメラを搭載した Raspberry Pi 以外の環境では撮影できない。
//! [CameraConfig::fake_camera] 設定でフェイクできる。

use super::SystemModule;
use crate::sys::taskserver::Control;
use crate::sys::{config, taskserver};
use anyhow::{anyhow, bail, ensure, Result};
use chrono::{Local, NaiveTime};
use image::{imageops::FilterType, ImageOutputFormat};
use log::{error, info, warn};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    io::Cursor,
    os::linux::fs::MetadataExt,
    path::{Path, PathBuf},
};
use tokio::{
    fs::{self, File},
    io::AsyncWriteExt,
    process::Command,
};

/// サムネイルファイル名のポストフィクス。
const THUMB_POSTFIX: &str = "thumb";

/// カメラ設定データ。toml 設定に対応する。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CameraConfig {
    /// カメラ自動撮影タスクを有効化する。
    enabled: bool,
    /// 起動時に1回だけカメラ自動撮影タスクを起動する。デバッグ用。
    debug_exec_once: bool,
    /// raspistill によるリアル撮影ではなく、ダミー黒画像が撮れたことにする。
    /// Raspberry Pi 以外の環境でのデバッグ用。
    fake_camera: bool,
    /// 撮影した画像を保存するディレクトリ。
    /// [Self::total_size_limit_mb] により自動で削除される。
    pic_history_dir: String,
    /// [Self::pic_history_dir] から移す、永久保存ディレクトリ。
    pic_archive_dir: String,
    /// [Self::pic_history_dir] のサイズ制限。これを超えた分が古いものから削除される。
    total_size_limit_mb: u32,
    /// 画像一覧ページの1ページ当たりの画像数。
    pub page_by: u32,
}

impl Default for CameraConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            debug_exec_once: false,
            fake_camera: true,
            pic_history_dir: "./camera/history".to_string(),
            pic_archive_dir: "./camera/archive".to_string(),
            total_size_limit_mb: 1024,
            page_by: 100,
        }
    }
}

/// ストレージ上の画像を示すエントリ。
#[derive(Clone)]
pub struct PicEntry {
    /// メイン画像のファイルパス。
    pub path_main: PathBuf,
    /// サムネイル画像のファイルパス。
    pub path_th: PathBuf,
    /// [Self::path_main] と [Self::path_th] の合計ファイルサイズ。
    pub total_size: u64,
}

/// 画像リストは [BTreeMap] により名前でソートされた状態で管理する。
///
/// 名前は撮影日時とするため、古い順にソートされる。
type PicDict = BTreeMap<String, PicEntry>;

/// ストレージ上の全データを管理するデータ構造。
struct Storage {
    /// 撮影された画像リスト。自動削除対象。
    pic_history_list: PicDict,
    /// [Self::pic_archive_list] から移動された画像リスト。自動削除しない。
    pic_archive_list: PicDict,
}

/// Camera システムモジュール。
pub struct Camera {
    /// 設定データ。
    ///
    /// web からも参照される。
    pub config: CameraConfig,
    /// 自動撮影の時刻リスト。
    wakeup_list: Vec<NaiveTime>,
    /// ストレージ上の画像リストデータ。
    storage: Storage,
}

impl Camera {
    /// コンストラクタ。
    ///
    /// 設定データの読み込みと、ストレージの状態取得を行い画像リストを初期化する。
    pub fn new(wakeup_list: Vec<NaiveTime>) -> Result<Self> {
        info!("[camera] initialize");

        let config = config::get(|cfg| cfg.camera.clone());
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

    /// ストレージ上の画像リスト (history, archive) を取得する。
    pub fn pic_list(&self) -> (&PicDict, &PicDict) {
        (
            &self.storage.pic_history_list,
            &self.storage.pic_archive_list,
        )
    }

    /// キーからファイル名を生成する。
    fn create_file_names(key: &str) -> (String, String) {
        (format!("{key}.jpg"), format!("{key}_{THUMB_POSTFIX}.jpg"))
    }

    /// 撮影した画像をストレージに書き出し、管理構造に追加する。
    ///
    /// 名前は現在日時から自動的に付与される。
    ///
    /// * `img` - jpg ファイルのバイナリデータ。
    /// * `thumb` - サムネイル jpg ファイルのバイナリデータ。
    pub async fn push_pic_history(&mut self, img: &[u8], thumb: &[u8]) -> Result<()> {
        // 現在時刻からキーを生成する
        // 重複するなら少し待ってからリトライする
        let mut now;
        let mut dtstr;
        loop {
            now = Local::now();
            dtstr = now.format("%Y%m%d_%H%M%S").to_string();

            if self.storage.pic_history_list.contains_key(&dtstr) {
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await
            } else {
                break;
            }
        }
        let (name_main, name_th) = Self::create_file_names(&dtstr);

        let total_size = img.len() + thumb.len();
        let total_size = total_size as u64;

        // ファイルパスを生成する
        let root = Path::new(&self.config.pic_history_dir);
        let mut path_main = PathBuf::from(root);
        path_main.push(name_main);
        let mut path_th = PathBuf::from(root);
        path_th.push(name_th);

        // ファイルに書き込む
        {
            info!("[camera-pic] write {}", path_main.display());
            let mut file = File::create(&path_main).await?;
            file.write_all(img).await?;
        }
        {
            info!("[camera-pic] write {}", path_th.display());
            let mut file = File::create(&path_th).await?;
            file.write_all(thumb).await?;
        }
        // 成功したらマップに追加する
        let entry = PicEntry {
            path_main,
            path_th,
            total_size,
        };
        assert!(self.storage.pic_history_list.insert(dtstr, entry).is_none());

        Ok(())
    }

    /// ヒストリ内の `key` で指定したエントリを永続領域にコピーする。
    ///
    /// * `key` - エントリ名。
    pub async fn push_pic_archive(&mut self, key: &str) -> Result<()> {
        // ヒストリから name を検索する
        let history = &self.storage.pic_history_list;
        let archive = &mut self.storage.pic_archive_list;
        let entry = history
            .get(key)
            .ok_or_else(|| anyhow!("picture not found: {}", key))?;

        // key からファイル名を生成し、パスを生成する
        let (name_main, name_th) = Self::create_file_names(key);
        let root = Path::new(&self.config.pic_archive_dir);
        let mut path_main = PathBuf::from(root);
        path_main.push(name_main);
        let mut path_th = PathBuf::from(root);
        path_th.push(name_th);

        // コピーを実行する
        let main_size = fs::copy(&entry.path_main, &path_main).await?;
        let th_size = fs::copy(&entry.path_th, &path_th).await?;

        // 成功したらマップに追加する
        let entry = PicEntry {
            path_main,
            path_th,
            total_size: main_size + th_size,
        };
        if archive.insert(key.to_string(), entry).is_some() {
            warn!("[camera-pic] pic archive is overwritten: {}", key);
        }

        Ok(())
    }

    pub async fn delete_pic_history(&mut self, id: &str) -> Result<()> {
        Self::delete_pic(&mut self.storage.pic_history_list, id).await
    }

    pub async fn delete_pic_archive(&mut self, id: &str) -> Result<()> {
        Self::delete_pic(&mut self.storage.pic_archive_list, id).await
    }

    async fn delete_pic(list: &mut PicDict, id: &str) -> Result<()> {
        let entry = list
            .remove(id)
            .ok_or_else(|| anyhow!("picture not found: {}", id))?;

        if let Err(why) = fs::remove_file(&entry.path_main).await {
            error!("[camera] cannot remove {} main: {}", id, why);
        }
        if let Err(why) = fs::remove_file(&entry.path_th).await {
            error!("[camera] cannot remove {} thumb: {}", id, why);
        }
        info!("[camera] deleted: {}", id);

        Ok(())
    }

    /// [PicDict] の合計ファイルサイズを計算する。
    ///
    /// オーダ O(n)。
    /// 64 bit でオーバーフローすると panic する。
    fn calc_total_size(list: &PicDict) -> u64 {
        list.iter().fold(0, |acc, (_, entry)| {
            // panic if overflow
            acc.checked_add(entry.total_size).unwrap()
        })
    }

    /// 必要に応じて自動削除を行う。
    async fn clean_pic_history(&mut self) -> Result<()> {
        info!("[camera] clean history");

        let limit = self.config.total_size_limit_mb as u64 * 1024 * 1024;
        let history = &mut self.storage.pic_history_list;

        let mut total = Self::calc_total_size(history);
        while total > limit {
            info!("[camera] total: {}, limit: {}", total, limit);

            // 一番古いものを削除する (1.66.0 or later)
            let (id, entry) = history.pop_first().unwrap();
            // 削除でのエラーはログを出して続行する
            if let Err(why) = fs::remove_file(entry.path_main).await {
                error!("[camera] cannot remove {} main: {}", id, why);
            }
            if let Err(why) = fs::remove_file(entry.path_th).await {
                error!("[camera] cannot remove {} thumb: {}", id, why);
            }
            info!("[camera] deleted: {}", id);

            total -= entry.total_size;
        }

        assert!(total == Self::calc_total_size(history));
        info!("[camera] clean history completed");
        Ok(())
    }

    /// 自動撮影タスク。
    async fn auto_task(ctrl: Control) -> Result<()> {
        let pic = take_a_pic(TakePicOption::new()).await?;

        let thumb = create_thumbnail(&pic)?;

        let mut camera = ctrl.sysmods().camera.lock().await;
        camera.push_pic_history(&pic, &thumb).await?;
        camera.clean_pic_history().await?;
        drop(camera);

        Ok(())
    }
}

impl SystemModule for Camera {
    /// async 使用可能になってからの初期化。
    ///
    /// 設定有効ならば [Self::auto_task] を spawn する。
    fn on_start(&self, ctrl: &Control) {
        info!("[camera] on_start");
        if self.config.enabled {
            if self.config.debug_exec_once {
                taskserver::spawn_oneshot_task(ctrl, "camera-auto", Camera::auto_task);
            } else {
                taskserver::spawn_periodic_task(
                    ctrl,
                    "camera-auto",
                    &self.wakeup_list,
                    Camera::auto_task,
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
        path_th.set_file_name(format!("{name}_{THUMB_POSTFIX}"));
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

/// 横最大サイズ。(Raspberry Pi Camera V2)
const PIC_MAX_W: u32 = 3280;
/// 縦最大サイズ。(Raspberry Pi Camera V2)
const PIC_MAX_H: u32 = 2464;
/// 横最小サイズ。(Raspberry Pi Camera V2)
const PIC_MIN_W: u32 = 32;
/// 縦最小サイズ。(Raspberry Pi Camera V2)
const PIC_MIN_H: u32 = 24;
/// 横デフォルトサイズ。
const PIC_DEF_W: u32 = PIC_MAX_W;
/// 縦デフォルトサイズ。
const PIC_DEF_H: u32 = PIC_MAX_H;
/// jpeg 最大クオリティ。
const PIC_MAX_Q: u8 = 100;
/// jpeg 最小クオリティ。
const PIC_MIN_Q: u8 = 0;
/// jpeg デフォルトクオリティ。
const PIC_DEF_Q: u8 = 85;
/// デフォルト撮影時間(ms)。TO はタイムアウト。
const PIC_DEF_TO_MS: u32 = 1000;
/// サムネイルの横サイズ。
const THUMB_W: u32 = 128;
/// サムネイルの縦サイズ。
const THUMB_H: u32 = 96;
/// サムネイルの jpeg クオリティ。
const THUMB_Q: u8 = 35;

/// 写真撮影オプション。
pub struct TakePicOption {
    /// 横サイズ。
    w: u32,
    /// 縦サイズ。
    h: u32,
    /// jpeg クオリティ。
    q: u8,
    /// 撮影時間(ms)。
    timeout_ms: u32,
}

#[allow(dead_code)]
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
    pub fn quality(mut self, q: u8) -> Self {
        assert!((PIC_MIN_Q..=PIC_MAX_Q).contains(&q));
        self.q = q;
        self
    }
    pub fn timeout_ms(mut self, timeout_ms: u32) -> Self {
        self.timeout_ms = timeout_ms;
        self
    }
}

/// 写真を撮影する。成功すると jpeg バイナリデータを返す。
///
/// 従来は raspistill コマンドを使っていたが、Bullseye より廃止された。
/// カメラ関連の各種操作は libcamera に移動、集約された。
/// raspistill コマンド互換の libcamera-still コマンドを使う。
///
/// 同時に2つ以上を実行できないかつ時間がかかるので、[tokio::sync::Mutex] で排他する。
///
/// * `opt` - 撮影オプション。
pub async fn take_a_pic(opt: TakePicOption) -> Result<Vec<u8>> {
    // 他の関数でも raspistill を使う場合外に出す
    static LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

    let fake = config::get(|cfg| cfg.camera.fake_camera);

    let bin = if !fake {
        let _lock = LOCK.lock().await;
        let output = Command::new("libcamera-still")
            .arg("-o")
            .arg("-")
            .arg("-t")
            .arg(opt.timeout_ms.to_string())
            .arg("-q")
            .arg(opt.q.to_string())
            .arg("--width")
            .arg(opt.w.to_string())
            .arg("--height")
            .arg(opt.h.to_string())
            .output()
            .await?;
        if !output.status.success() {
            bail!("libcamera-still failed: {}", output.status);
        }

        output.stdout
        // unlock
    } else {
        // バイナリ同梱のデフォルト画像が撮れたことにする
        let buf =
            include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/res/camera_def.jpg")).to_vec();

        // オプションの w, h にリサイズする
        let src = image::load_from_memory_with_format(&buf, image::ImageFormat::Jpeg)?;
        let dst = src.resize_exact(opt.w, opt.h, FilterType::Nearest);
        let mut output = Cursor::new(vec![]);
        dst.write_to(&mut output, ImageOutputFormat::Jpeg(PIC_DEF_Q))?;

        output.into_inner()
    };
    // raspistill は同時に複数プロセス起動できないので mutex で保護する

    Ok(bin)
}

/// サムネイルを作成する。
/// 成功すれば jpeg バイナリデータを返す。
///
/// * `src_buf` - 元画像とする jpeg バイナリデータ。
pub fn create_thumbnail(src_buf: &[u8]) -> Result<Vec<u8>> {
    let src = image::load_from_memory_with_format(src_buf, image::ImageFormat::Jpeg)?;
    let dst = src.thumbnail(THUMB_W, THUMB_H);

    let mut buf = Cursor::new(Vec::<u8>::new());
    dst.write_to(&mut buf, ImageOutputFormat::Jpeg(THUMB_Q))?;

    Ok(buf.into_inner())
}

/// 画像をリサイズする。
/// 成功すれば jpeg バイナリデータを返す。
///
/// * `src_buf` - 元画像とする jpeg バイナリデータ。
pub fn resize(src_buf: &[u8], w: u32, h: u32) -> Result<Vec<u8>> {
    let src = image::load_from_memory_with_format(src_buf, image::ImageFormat::Jpeg)?;
    let dst = src.resize_exact(w, h, FilterType::Nearest);

    let mut buf = Cursor::new(Vec::<u8>::new());
    dst.write_to(&mut buf, ImageOutputFormat::Jpeg(PIC_DEF_Q))?;

    Ok(buf.into_inner())
}
