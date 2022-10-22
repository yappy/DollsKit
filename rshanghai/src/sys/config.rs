//! 設定データの管理。

use once_cell::sync::OnceCell;
use std::ops::RangeBounds;
use std::sync::RwLock;

/// グローバルに保持するデータ。
/// それぞれは Json object によるツリー構造。
#[derive(Default)]
struct ConfigData {
    root_list:  Vec<serde_json::Value>,
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
/// json のパースに失敗した場合、その詳細を示す文字列を返す。
pub fn add_config(json_src: &str) -> Result<(), String>
{
    // parse
    let jsobj = match serde_json::from_str(json_src) {
        Ok(json) => json,
        Err(e) => return Err(e.to_string()),
    };

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
/// 見つからなかった場合、および
/// 数値, 真偽値, 文字列, 配列 以外が見つかった場合は json - null を返す。
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

    if value.is_number() || value.is_boolean() || value.is_string() || value.is_array() {
        value.clone()
    }
    else {
        serde_json::Value::Null
    }
}

/// [ConfigData::root_list] を順番に、[search] によって検索する。
fn search_all(keys: &[&str]) -> serde_json::Value {
    // rlock
    let config = CONFIG
        .get().unwrap()
        .read().unwrap();

    for root in config.root_list.iter().rev() {
        let value = search(root, keys);
        if !value.is_null() {
            return value;
        }
    }

    serde_json::Value::Null
}

/// 真偽値設定データを取得する。
/// 見つからない場合や真偽値でない場合は None を返す。
pub fn get_bool(keys: &[&str]) -> Option<bool> {
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
        init();
        add_config(def).unwrap();
        add_config(main).unwrap();

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
    fn bool_string() {
        let main = r#"{"root": {"bool1": false, "bool2": true, "str": "Hello"}}"#;
        let def = "{}";
        init();
        add_config(def).unwrap();
        add_config(main).unwrap();

        let v = get_bool(&["root", "bool1"]);
        assert!(v.unwrap() == false);
        let v = get_bool(&["root", "bool2"]);
        assert!(v.unwrap() == true);
        let v = get_string(&["root", "str"]);
        assert!(v.unwrap() == "Hello");
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

        let v = get_i64(&["i64", "min"], i64::MIN+1..);
        assert!(v.is_none());
        let v = get_i64(&["i64", "max"], ..=i64::MAX-1);
        assert!(v.is_none());
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

        let v = get_u64(&["u64", "min"], u64::MIN+1..);
        assert!(v.is_none());
        let v = get_u64(&["u64", "max"], ..=u64::MAX-1);
        assert!(v.is_none());
    }

    #[test]
    #[should_panic]
    fn parse_error() {
        let jsonstr = "{1: 2";
        init();
        add_config(jsonstr).unwrap();
    }
}
