use once_cell::sync::OnceCell;
use std::sync::RwLock;
use serde_json;
use serde_json::json;

struct ConfigData {
    def:  serde_json::Value,
    main: serde_json::Value,
}

// default = empty Object (map)
impl Default for ConfigData {
    fn default() -> Self {
        Self { def: json!({}), main: json!({}) }
    }
}

static CONFIG: OnceCell<RwLock<ConfigData>> = OnceCell::new();

/// 設定データを json 文字列からロードして設定データシステムを初期化する。
pub fn init_and_load(def_json: &str, main_json: &str) -> Result<(), String>
{
    // lazy inialize + wlock
    let mut config = CONFIG
        .get_or_init(|| RwLock::new(Default::default()))
        .write()
        .unwrap();

    config.def = match serde_json::from_str(def_json) {
        Ok(json) => json,
        Err(e) => return Err(format!("{}", e)),
    };
    config.main = match serde_json::from_str(main_json) {
        Ok(json) => json,
        Err(e) => return Err(format!("{}", e)),
    };

    Ok(())
}
