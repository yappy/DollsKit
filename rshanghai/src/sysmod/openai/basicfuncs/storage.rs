use crate::sysmod::openai::{
    function::{
        get_arg_i64_opt, get_arg_str, BasicContext, FuncArgs, FuncBodyAsync, FunctionTable,
    },
    Function, ParameterElement, Parameters,
};
use anyhow::{ensure, Result};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashMap, VecDeque},
    path::{Path, PathBuf},
    sync::Arc,
};

const NOTE_FILE_NAME: &str = "note.json";
const NOTE_COUNT_MAX: usize = 8;
const NOTE_LENGTH_MAX: usize = 256;
const NOTE_LENGTH_MAX_I64: i64 = NOTE_LENGTH_MAX as i64;

/// このモジュールの関数をすべて登録する。
///
/// [FunctionTable::basic_context] に [BasicContext::storage_dir] が
/// 設定されている場合のみ登録される。
pub fn register_all<T: 'static>(func_table: &mut FunctionTable<T>) {
    if func_table.basic_context().storage_dir.is_some() {
        register_load(func_table);
        register_save(func_table);
        register_delete(func_table);
    }
}

/// ストレージからノートを読み込む。
async fn load(storage_dir: PathBuf, args: &FuncArgs) -> Result<String> {
    let user = get_arg_str(args, "user")?.to_string();

    tokio::fs::create_dir_all(&storage_dir).await?;
    let path = storage_dir.join(NOTE_FILE_NAME);
    let json = {
        let _lock = rlock_file().await;

        let note = load_file(&path).await.unwrap_or_default();
        note.map.get(&user).map_or_else(
            || serde_json::to_string(&VecDeque::<String>::new()),
            serde_json::to_string,
        )?
    };

    Ok(json)
}

fn load_pin<T>(bctx: Arc<BasicContext>, _ctx: T, args: &FuncArgs) -> FuncBodyAsync {
    let storage_dir = bctx.storage_dir.as_ref().unwrap().clone();
    Box::pin(load(storage_dir, args))
}

fn register_load<T: 'static>(func_table: &mut FunctionTable<T>) {
    let mut properties = HashMap::new();
    properties.insert(
        "user".to_string(),
        ParameterElement {
            type_: "string".to_string(),
            description: Some("user name".to_string()),
            ..Default::default()
        },
    );

    func_table.register_function(
        Function {
            name: "note_load".to_string(),
            description: Some("Load note from permanent storage".to_string()),
            parameters: Parameters {
                type_: "object".to_string(),
                properties,
                required: vec!["user".to_string()],
            },
        },
        Box::new(load_pin),
    );
}

/// ストレージにノートを保存する。
async fn save(storage_dir: PathBuf, args: &FuncArgs) -> Result<String> {
    let user = get_arg_str(args, "user")?.to_string();
    let content = get_arg_str(args, "content")?;
    ensure!(
        content.len() <= NOTE_LENGTH_MAX,
        "content length must be less than {}",
        NOTE_LENGTH_MAX
    );
    // タイムスタンプ付与
    let elem = NoteElement {
        datetime: chrono::Local::now().to_rfc3339(),
        content: content.to_string(),
    };

    tokio::fs::create_dir_all(&storage_dir).await?;
    let mut deleted = vec![];
    let path = storage_dir.join(NOTE_FILE_NAME);
    {
        let _lock = wlock_file().await;

        let mut note = load_file(&path).await.unwrap_or_default();
        if !note.map.contains_key(&user) {
            note.map.insert(user.clone(), VecDeque::new());
        }
        let list = note.map.get_mut(&user).unwrap();
        list.push_back(elem);
        while list.len() > NOTE_COUNT_MAX {
            deleted.push(list.pop_front().unwrap());
        }

        save_file(&path, &note).await?;
    }

    #[derive(Serialize)]
    struct FuncResult {
        result: &'static str,
        deleted: Vec<NoteElement>,
    }
    let result = FuncResult {
        result: "OK",
        deleted,
    };

    Ok(serde_json::to_string(&result)?)
}

fn save_pin<T>(bctx: Arc<BasicContext>, _ctx: T, args: &FuncArgs) -> FuncBodyAsync {
    let storage_dir = bctx.storage_dir.as_ref().unwrap().clone();
    Box::pin(save(storage_dir, args))
}

fn register_save<T: 'static>(func_table: &mut FunctionTable<T>) {
    let mut properties = HashMap::new();
    properties.insert(
        "user".to_string(),
        ParameterElement {
            type_: "string".to_string(),
            description: Some("user name".to_string()),
            ..Default::default()
        },
    );
    properties.insert(
        "content".to_string(),
        ParameterElement {
            type_: "string".to_string(),
            description: Some("data to be saved".to_string()),
            ..Default::default()
        },
    );

    func_table.register_function(
        Function {
            name: "note_save".to_string(),
            description: Some(format!("Save note to permanent storage. If {} files already exist, the oldest one will be deleted.", NOTE_COUNT_MAX)),
            parameters: Parameters {
                type_: "object".to_string(),
                properties,
                required: vec!["user".to_string(), "content".to_string()],
            },
        },
        Box::new(save_pin),
    );
}

/// ストレージからノートを部分削除する
async fn delete(storage_dir: PathBuf, args: &FuncArgs) -> Result<String> {
    let user = get_arg_str(args, "user")?.to_string();
    let index = get_arg_i64_opt(args, "index", 0..NOTE_LENGTH_MAX_I64)?;

    tokio::fs::create_dir_all(&storage_dir).await?;
    let mut deleted = vec![];
    let path = storage_dir.join(NOTE_FILE_NAME);
    {
        let _lock = wlock_file().await;

        let mut note = load_file(&path).await.unwrap_or_default();
        if !note.map.contains_key(&user) {
            note.map.insert(user.clone(), VecDeque::new());
        }
        let list = note.map.get_mut(&user).unwrap();

        if let Some(index) = index {
            if let Some(elem) = list.remove(index as usize) {
                deleted.push(elem);
            }
        } else {
            while let Some(elem) = list.pop_front() {
                deleted.push(elem);
            }
        }

        save_file(&path, &note).await?;
    }

    #[derive(Serialize)]
    struct FuncResult {
        result: &'static str,
        deleted: Vec<NoteElement>,
    }
    let result_str = if deleted.is_empty() {
        "Error: No data deleted"
    } else {
        "OK"
    };
    let result = FuncResult {
        result: result_str,
        deleted,
    };

    Ok(serde_json::to_string(&result)?)
}

fn delete_pin<T>(bctx: Arc<BasicContext>, _ctx: T, args: &FuncArgs) -> FuncBodyAsync {
    let storage_dir = bctx.storage_dir.as_ref().unwrap().clone();
    Box::pin(delete(storage_dir, args))
}

fn register_delete<T: 'static>(func_table: &mut FunctionTable<T>) {
    let mut properties = HashMap::new();
    properties.insert(
        "user".to_string(),
        ParameterElement {
            type_: "string".to_string(),
            description: Some("user name".to_string()),
            ..Default::default()
        },
    );
    properties.insert(
        "index".to_string(),
        ParameterElement {
            type_: "integer".to_string(),
            description: Some(
                "Data index to be deleted (0-origin). If omitted, all data will be deleted."
                    .to_string(),
            ),
            minumum: Some(0),
            maximum: Some(NOTE_LENGTH_MAX_I64 - 1),
            ..Default::default()
        },
    );

    func_table.register_function(
        Function {
            name: "note_delete".to_string(),
            description: Some("Delete note".to_string()),
            parameters: Parameters {
                type_: "object".to_string(),
                properties,
                required: vec!["user".to_string(), "content".to_string()],
            },
        },
        Box::new(delete_pin),
    );
}

// -----------------------------------------------------------------------------

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct AssistantNote {
    #[serde(flatten)]
    map: BTreeMap<String, VecDeque<NoteElement>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct NoteElement {
    datetime: String,
    content: String,
}

static LOCK: tokio::sync::RwLock<()> = tokio::sync::RwLock::const_new(());

async fn rlock_file() -> tokio::sync::RwLockReadGuard<'static, ()> {
    LOCK.read().await
}

async fn wlock_file() -> tokio::sync::RwLockWriteGuard<'static, ()> {
    LOCK.write().await
}

async fn load_file(path: impl AsRef<Path>) -> Result<AssistantNote> {
    assert!(LOCK.try_write().is_err());

    let src = tokio::fs::read_to_string(path).await?;
    let note: AssistantNote = serde_json::from_str(&src)?;

    Ok(note)
}

async fn save_file(path: impl AsRef<Path>, note: &AssistantNote) -> Result<()> {
    assert!(LOCK.try_write().is_err());

    let mut src = serde_json::to_string_pretty(note)?;
    src.push('\n');
    tokio::fs::write(path, src.as_bytes()).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use super::*;

    #[tokio::test]
    async fn assistant_node_parse() -> Result<()> {
        let f = tempfile::NamedTempFile::new()?;

        let json_src = r#"{"user1": []}"#;
        writeln!(f.as_file(), "{json_src}")?;

        {
            let _lock = rlock_file().await;
            let note = load_file(f.path()).await?;
            assert_eq!(note.map.len(), 1);
            assert!(note.map.contains_key("user1"));
        }

        Ok(())
    }
}
