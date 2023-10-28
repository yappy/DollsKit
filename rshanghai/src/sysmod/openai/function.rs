//! OpenAI API - function.

use crate::sysmod::openai::ChatMessage;

use super::{Function, ParameterElement, Parameters};
use anyhow::{anyhow, bail, Result};
use log::{info, warn};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, future::Future, pin::Pin};

// https://users.rust-lang.org/t/how-to-handle-a-vector-of-async-function-pointers/39804

/// sync fn で、async fn に引数を引き渡して呼び出しその Future を返す関数型。
type FuncBodyAsync<'a> = Pin<Box<dyn Future<Output = Result<String>> + Sync + Send + 'a>>;
/// 関数の Rust 上での定義。
///
/// 引数は [FuncArgs] で、返り値は文字列の async fn。
type FuncBody = Box<dyn Fn(&FuncArgs) -> FuncBodyAsync + Sync + Send>;
/// 引数。文字列から文字列へのマップ。
type FuncArgs = HashMap<String, String>;

/// 引数は JSON ソース文字列で与えられる。
/// デシリアライズでパースするための構造体。
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct Args {
    #[serde(flatten)]
    args: FuncArgs,
}

/// 関数群の管理。
pub struct FunctionTable {
    /// OpenAI API に渡すためのリスト。
    function_list: Vec<Function>,
    /// 関数名から Rust 関数へのマップ。
    call_table: HashMap<&'static str, FuncBody>,
}

impl FunctionTable {
    pub fn new() -> Self {
        Self {
            function_list: Default::default(),
            call_table: Default::default(),
        }
    }

    pub fn register_all_functions(&mut self) {
        self.register_get_version();
        self.register_get_current_datetime();
    }

    /// OpenAI API に渡すためのリストを取得する。
    pub fn function_list(&self) -> &Vec<Function> {
        &self.function_list
    }

    /// 関数を呼び出す。
    ///
    /// OpenAI API からのデータをそのまま渡せ、
    /// 結果も API にそのまま渡せる [ChatMessage] で返す。
    /// エラーも適切なメッセージとして返す。
    pub async fn call(&self, func_name: &str, args_json_str: &str) -> ChatMessage {
        info!("[openai-func] Call {func_name} {args_json_str}");

        let res = {
            let args = serde_json::from_str::<Args>(args_json_str)
                .map_err(|err| anyhow!("Arguments parse error: {err}"));
            match args {
                Ok(args) => self.call_internal(func_name, &args.args).await,
                Err(err) => Err(err),
            }
        };

        let content = match &res {
            Ok(res) => {
                info!("[openai-func] {func_name} returned: {res}");
                res.to_string()
            }
            Err(err) => {
                warn!("[openai-func] {func_name} failed: {:#?}", err);
                err.to_string()
            }
        };

        ChatMessage {
            role: "function".to_string(),
            name: Some(func_name.to_string()),
            content: Some(content),
            ..Default::default()
        }
    }

    /// [Self::call] の内部メイン処理。
    async fn call_internal(&self, func_name: &str, args: &FuncArgs) -> Result<String> {
        let func = self
            .call_table
            .get(func_name)
            .ok_or_else(|| anyhow!("Error: Function {func_name} not found"))?;

        // call body
        func(args).await.map_err(|err| anyhow!("Error: {err}"))
    }
}

////////////////////////////////////////////////////////////////////////////////

/// args から引数名で検索し、値への参照を返す。
/// 見つからない場合、いい感じのエラーメッセージの [anyhow::Error] を返す。
fn get_arg<'a>(args: &'a FuncArgs, name: &str) -> Result<&'a String> {
    let value = args.get(&name.to_string());
    value.ok_or_else(|| anyhow!("Error: Argument {name} is required"))
}

fn get_version_sync(args: &FuncArgs) -> FuncBodyAsync {
    Box::pin(get_version(args))
}

async fn get_version(_args: &FuncArgs) -> Result<String> {
    use crate::sys::version;

    Ok(version::version_info().to_string())
}

impl FunctionTable {
    fn register_get_version(&mut self) {
        self.function_list.push(Function {
            name: "get_version".to_string(),
            description: Some("Get the version of the assistant program".to_string()),
            parameters: Parameters {
                type_: "object".to_string(),
                properties: Default::default(),
                required: Default::default(),
            },
        });
        self.call_table
            .insert("get_version", Box::new(get_version_sync));
    }
}

fn get_current_datetime_sync(args: &FuncArgs) -> FuncBodyAsync {
    Box::pin(get_current_datetime(args))
}

async fn get_current_datetime(args: &FuncArgs) -> Result<String> {
    use chrono::{DateTime, Local, Utc};

    let tz = get_arg(args, "tz")?;
    match tz.as_str() {
        "JST" => {
            let dt: DateTime<Local> = Local::now();
            Ok(dt.to_string())
        }
        "UTC" => {
            let dt: DateTime<Utc> = Utc::now();
            Ok(dt.to_string())
        }
        _ => {
            bail!("Parameter tz must be JST or UTC")
        }
    }
}

impl FunctionTable {
    fn register_get_current_datetime(&mut self) {
        let mut properties = HashMap::new();
        properties.insert(
            "tz".to_string(),
            ParameterElement {
                type_: "string".to_string(),
                description: Some("Time zone".to_string()),
                enum_: Some(vec!["JST".to_string(), "UTC".to_string()]),
            },
        );
        self.function_list.push(Function {
            name: "get_current_datetime".to_string(),
            description: Some("Get the current date and time".to_string()),
            parameters: Parameters {
                type_: "object".to_string(),
                properties,
                required: vec!["tz".to_string()],
            },
        });
        self.call_table
            .insert("get_current_datetime", Box::new(get_current_datetime_sync));
    }
}
