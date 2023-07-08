//! 設定データの管理。

use anyhow::{anyhow, Result};
use once_cell::sync::OnceCell;
use serde_json::Value;
use std::fmt::Debug;
use std::ops::RangeBounds;
use std::sync::RwLock;

/// グローバルに保持するデータ。
/// それぞれは Json object によるツリー構造。
#[derive(Default)]
struct ConfigData {
    root_list: Vec<serde_json::Value>,
}

/// 設定データ(グローバル変数)。
///
/// 一応スレッドセーフとするが、初期化時に一度だけ書き換え、
/// その後は read only アクセスしかしないので RwLock とする。
static CONFIG: OnceCell<RwLock<ConfigData>> = OnceCell::new();

// 設定データシステムを初期化する。
pub fn init() {
    // lazy inialize + wlock
    let mut config = CONFIG
        .get_or_init(|| RwLock::new(Default::default()))
        .write()
        .unwrap();
    config.root_list.clear();
}

/// 設定データを json 文字列からロードして検索リストに追加する。
///
/// 同じキーを持つ場合は後で追加したものが優先される。
pub fn add_config(json_src: &str) -> Result<()> {
    // parse
    let jsobj = serde_json::from_str(json_src)?;

    // lazy inialize + wlock
    let mut config = CONFIG
        .get_or_init(|| RwLock::new(Default::default()))
        .write()
        .unwrap();

    config.root_list.push(jsobj);

    Ok(())
}

/// json Object tree を文字列キーの配列で検索する。
///
/// 検索に成功した場合、その値をコピーして返す。
/// 見つからなかった場合、json - null を返す。
///
/// * `root` - 検索開始する json オブジェクト。
/// * `keys` - 文字列キーの配列(スライス)。
fn search(root: &serde_json::Value, keys: &[&str]) -> serde_json::Value {
    let null_value = serde_json::Value::Null;

    let mut value = root;
    for key in keys.iter() {
        value = match value.as_object() {
            Some(map) => map.get(*key).unwrap_or(&null_value),
            None => &null_value,
        };
    }

    value.clone()
}

/// [ConfigData::root_list] を順番に、[search] によって検索する。
fn search_all(keys: &[&str]) -> serde_json::Value {
    // rlock
    let config = CONFIG.get().unwrap().read().unwrap();

    for root in config.root_list.iter().rev() {
        let value = search(root, keys);
        if !value.is_null() {
            return value;
        }
    }

    serde_json::Value::Null
}

/// json オブジェクトを取得する。
///
/// 見つからない場合やオブジェクトでない場合はエラーを返す。
pub fn get_object(keys: &[&str]) -> Result<Value> {
    let value = search_all(keys);
    if value.is_object() {
        Ok(value)
    } else {
        Err(anyhow!("object config not found: {:?}", keys))
    }
}

/// 真偽値設定データを取得する。
///
/// 見つからない場合や真偽値でない場合はエラーを返す。
pub fn get_bool(keys: &[&str]) -> Result<bool> {
    let value = search_all(keys).as_bool();
    if let Some(b) = value {
        Ok(b)
    } else {
        Err(anyhow!("bool config not found: {:?}", keys))
    }
}

/// 数値データを i64 で取得する。
///
/// 見つからない場合、数値でない場合、範囲外の場合はエラーを返す。
#[allow(dead_code)]
pub fn get_i64<R>(keys: &[&str], range: R) -> Result<i64>
where
    R: RangeBounds<i64> + Debug,
{
    let value = search_all(keys).as_i64();
    if let Some(n) = value {
        if range.contains(&n) {
            Ok(n)
        } else {
            Err(anyhow!("i64 config range error: {:?} {:?}", keys, range))
        }
    } else {
        Err(anyhow!("i64 config not found: {:?}", keys))
    }
}

/// 数値データを u64 で取得する。
///
/// 見つからない場合、数値でない場合、範囲外の場合はエラーを返す。
#[allow(dead_code)]
pub fn get_u64<R>(keys: &[&str], range: R) -> Result<u64>
where
    R: RangeBounds<u64> + Debug,
{
    let value = search_all(keys).as_u64();
    if let Some(n) = value {
        if range.contains(&n) {
            Ok(n)
        } else {
            Err(anyhow!("u64 config range error: {:?} {:?}", keys, range))
        }
    } else {
        Err(anyhow!("u64 config not found: {:?}", keys))
    }
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
        init();
        add_config(def).unwrap();
        add_config(main).unwrap();

        let v = get_string(&["a", "b", "c"]);
        assert_eq!(v.unwrap(), "main");
        let v = get_string(&["a", "b", "C"]);
        assert!(v.is_none());
        let v = get_string(&["a", "b", "d"]);
        assert_eq!(v.unwrap(), "default");
        let v = get_string(&["a", "b", "D"]);
        assert!(v.is_none());
    }

    #[test]
    #[serial(config)]
    fn bool_string() {
        let main = r#"{"root": {"bool1": false, "bool2": true, "str": "Hello"}}"#;
        let def = "{}";
        init();
        add_config(def).unwrap();
        add_config(main).unwrap();

        let v = get_bool(&["root", "bool1"]);
        assert!(!v.unwrap());
        let v = get_bool(&["root", "bool2"]);
        assert!(v.unwrap());
        let v = get_string(&["root", "str"]);
        assert_eq!(v.unwrap(), "Hello");
    }

    #[test]
    #[serial(config)]
    fn i64() {
        let main = r#"{"i64": {"min": -9223372036854775808, "max": 9223372036854775807}}"#;
        let def = "{}";
        init();
        add_config(def).unwrap();
        add_config(main).unwrap();

        let v = get_i64(&["i64", "min"], ..);
        assert_eq!(v.unwrap(), i64::MIN);
        let v = get_i64(&["i64", "max"], ..);
        assert_eq!(v.unwrap(), i64::MAX);

        let v = get_i64(&["i64", "min"], i64::MIN + 1..);
        assert!(v.is_err());
        let v = get_i64(&["i64", "max"], ..=i64::MAX - 1);
        assert!(v.is_err());
    }

    #[test]
    #[serial(config)]
    fn u64() {
        let main = r#"{"u64": {"min": 0, "max": 18446744073709551615}}"#;
        let def = "{}";
        init();
        add_config(def).unwrap();
        add_config(main).unwrap();

        let v = get_u64(&["u64", "min"], ..);
        assert_eq!(v.unwrap(), u64::MIN);
        let v = get_u64(&["u64", "max"], ..);
        assert_eq!(v.unwrap(), u64::MAX);

        let v = get_u64(&["u64", "min"], u64::MIN + 1..);
        assert!(v.is_err());
        let v = get_u64(&["u64", "max"], ..=u64::MAX - 1);
        assert!(v.is_err());
    }

    #[test]
    #[serial(config)]
    #[should_panic]
    fn parse_error() {
        let jsonstr = "{1: 2";
        init();
        add_config(jsonstr).unwrap();
    }
}
