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

/// Object tree を keys: 文字列の配列 (スライス) で検索する。
/// 見つからなかった場合、および
/// 数値, 真偽値, 文字列, 配列 以外が見つかった場合は null を返す。
/// 結果はコピーして返す。
fn search(root: &serde_json::Value, keys: &[&str]) -> serde_json::Value {
    let null_value = serde_json::Value::Null;

    let mut value = root;
    for key in keys.iter() {
        value = match value.as_object() {
            Some(map) => map.get(*key).unwrap_or(&null_value),
            None => &null_value,
        };
    }

    if value.is_number() || value.is_boolean() || value.is_string() || value.is_array() {
        value.clone()
    }
    else {
        serde_json::Value::Null
    }
}

/// main -> def の順番で検索する。
fn search_all(keys: &[&str]) -> serde_json::Value {
    // rlock
    let config = CONFIG
        .get().unwrap()
        .read().unwrap();

    let value = search(&config.main, keys);
    if !value.is_null() {
        value
    }
    else {
        search(&config.def, keys)
    }
}

/// 文字列設定データを取得する。
/// 見つからない場合や文字列でない場合は None を返す。
pub fn get_string(keys: &[&str]) -> Option<String>
{
    search_all(keys).as_str().map(|s| s.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic() {
        let main = r#"{"a": {"b": {"c": "main"}}}"#;
        let def = r#"{"a": {"b": {"d": "default"}}}"#;

        let result = init_and_load(def, main);
        assert!(result.is_ok());

        let v = get_string(&["a", "b", "c"]);
        assert!(v.unwrap_or_default() == "main");
        let v = get_string(&["a", "b", "C"]);
        assert!(v.is_none());
        let v = get_string(&["a", "b", "d"]);
        assert!(v.unwrap_or_default() == "default");
        let v = get_string(&["a", "b", "D"]);
        assert!(v.is_none());
    }
}
