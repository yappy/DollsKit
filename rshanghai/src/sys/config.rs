use once_cell::sync::OnceCell;
use std::ops::RangeBounds;
use std::sync::RwLock;
use serde_json::json;

/// グローバルに保持するデータ。
/// ユーザによる設定と、見つからなかった場合に使うデフォルト値からなる。
/// それぞれは Json object によるツリー構造。
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

/// 設定データ(グローバル変数)。
/// 一応スレッドセーフとするが、初期化時に一度だけ書き換え、
/// その後は read only アクセスしかしないので RwLock とする。
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
/// 数値, 真偽値, 文字列, 配列 以外が見つかった場合は json - null を返す。
/// グローバル構造内のデータを返す場合、コピーを返す。
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

/// 真偽値設定データを取得する。
/// 見つからない場合や真偽値でない場合は None を返す。
pub fn get_boolean(keys: &[&str]) -> Option<bool> {
    search_all(keys).as_bool()
}

/// 数値データを i64 で取得する。
/// 見つからない場合、数値でない場合、範囲外の場合は None を返す。
pub fn get_i64<R: RangeBounds<i64>>(keys: &[&str], range: R) -> Option<i64> {
    search_all(keys).as_i64().and_then(|num| {
        if range.contains(&num) { Some(num) }
        else { None }
    })
}

/// 数値データを u64 で取得する。
/// 見つからない場合、数値でない場合、範囲外の場合は None を返す。
pub fn get_u64<R: RangeBounds<u64>>(keys: &[&str], range: R) -> Option<u64> {
    search_all(keys).as_u64().and_then(|num| {
        if range.contains(&num) { Some(num) }
        else { None }
    })
}

/// 文字列設定データを取得する。
/// 見つからない場合や文字列でない場合は None を返す。
pub fn get_string(keys: &[&str]) -> Option<String> {
    search_all(keys).as_str().map(|s| s.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    #[serial(config)]
    fn search() {
        let main = r#"{"a": {"b": {"c": "main"}}}"#;
        let def = r#"{"a": {"b": {"d": "default"}}}"#;

        let result = init_and_load(def, main);
        assert!(result.is_ok());

        let v = get_string(&["a", "b", "c"]);
        assert!(v.unwrap() == "main");
        let v = get_string(&["a", "b", "C"]);
        assert!(v.is_none());
        let v = get_string(&["a", "b", "d"]);
        assert!(v.unwrap() == "default");
        let v = get_string(&["a", "b", "D"]);
        assert!(v.is_none());
    }

    #[test]
    #[serial(config)]
    fn i64() {
        let main = r#"{"i64": {"min": -9223372036854775808, "max": 9223372036854775807}}"#;
        let def = "{}";

        let result = init_and_load(def, main);
        assert!(result.is_ok());

        let v = get_i64(&["i64", "min"], ..);
        assert_eq!(v.unwrap(), i64::MIN);
        let v = get_i64(&["i64", "max"], ..);
        assert_eq!(v.unwrap(), i64::MAX);

        let v = get_i64(&["i64", "min"], i64::MIN+1..);
        assert!(v.is_none());
        let v = get_i64(&["i64", "max"], ..=i64::MAX-1);
        assert!(v.is_none());
    }
}
