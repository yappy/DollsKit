//! OpenAI API - function.

use super::{basicfuncs, ModelInfo};
use crate::sys::config;
use crate::sysmod::openai::{ChatMessage, Role};
use anyhow::bail;
use anyhow::{anyhow, Result};
use log::{info, warn};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::future::Future;
use std::ops::RangeBounds;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::Arc;

/// Function でもトークンを消費するが、算出方法がよく分からないので定数で確保する。
/// トークン制限エラーが起きた場合、エラーメッセージ中に含まれていた気がするので
/// それより大きめに確保する。
pub const FUNCTION_TOKEN: usize = 800;

// https://users.rust-lang.org/t/how-to-handle-a-vector-of-async-function-pointers/39804

/// OpenAI API json 定義の再エクスポート。
pub use super::Function;
pub use super::ParameterElement;
pub use super::Parameters;
/// sync fn で、async fn に引数を引き渡して呼び出しその Future を返す関数型。
pub type FuncBodyAsync<'a> = Pin<Box<dyn Future<Output = Result<String>> + Sync + Send + 'a>>;
/// 関数の Rust 上での定義。
///
/// 引数は [BasicContext], T, [FuncArgs] で、返り値は文字列の async fn。
pub type FuncBody<T> = Box<dyn Fn(Arc<BasicContext>, T, &FuncArgs) -> FuncBodyAsync + Sync + Send>;
/// 引数。文字列から Json value へのマップ。
pub type FuncArgs = HashMap<String, serde_json::value::Value>;

/// 引数は JSON ソース文字列で与えられる。
/// デシリアライズでパースするための構造体。
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct Args {
    #[serde(flatten)]
    args: FuncArgs,
}

/// 標準で関数に提供されるコンテキスト情報。
#[derive(Debug, Clone, Serialize)]
pub struct BasicContext {
    pub model: ModelInfo,
    pub storage_dir: Option<PathBuf>,
}

/// OpenAI function 群の管理テーブル。
pub struct FunctionTable<T> {
    /// OpenAI API に渡すためのリスト。
    function_list: Vec<Function>,
    /// 関数名から Rust 関数へのマップ。
    call_table: HashMap<String, FuncBody<T>>,

    basic_context: Arc<BasicContext>,
}

impl<T: 'static> FunctionTable<T> {
    pub fn new(model: ModelInfo, storage_dir_name: Option<&str>) -> Self {
        // openai config でディレクトリが指定されており、かつ、
        // この関数にストレージディレクトリ名が指定されている場合、Some
        let storage_dir = if let Some(storage_dir_name) = storage_dir_name {
            let dir = config::get(|c| c.openai.storage_dir.clone());
            if !dir.is_empty() {
                Some(Path::new(&dir).join(storage_dir_name))
            } else {
                None
            }
        } else {
            None
        };
        let basic_context = BasicContext { model, storage_dir };

        Self {
            function_list: Default::default(),
            call_table: Default::default(),
            basic_context: Arc::new(basic_context),
        }
    }

    pub fn basic_context(&self) -> &BasicContext {
        &self.basic_context
    }

    /// OpenAI API に渡すためのリストを取得する。
    pub fn function_list(&self) -> &Vec<Function> {
        &self.function_list
    }

    /// 関数一覧のヘルプ文字列を生成する。
    pub fn create_help(&self) -> String {
        let mut text = String::new();

        let mut first = true;
        for f in self.function_list.iter() {
            if first {
                first = false;
            } else {
                text.push('\n');
            }

            text.push_str(&f.name);

            let mut params: Vec<_> = f.parameters.properties.keys().cloned().collect();
            params.sort();
            text.push_str(&format!("({})", params.join(", ")));

            if let Some(desc) = &f.description {
                text.push_str(&format!("\n    {}", desc));
            }
        }

        text
    }

    /// 関数を呼び出す。
    ///
    /// OpenAI API からのデータをそのまま渡せ、
    /// 結果も API にそのまま渡せる [ChatMessage] で返す。
    /// エラーも適切なメッセージとして返す。
    pub async fn call(&self, ctx: T, func_name: &str, args_json_str: &str) -> ChatMessage {
        info!("[openai-func] Call {func_name} {args_json_str}");

        let res = {
            let args = serde_json::from_str::<Args>(args_json_str)
                .map_err(|err| anyhow!("Arguments parse error: {err}"));
            match args {
                Ok(args) => self.call_internal(ctx, func_name, &args.args).await,
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
            role: Role::Function,
            name: Some(func_name.to_string()),
            content: Some(content),
            ..Default::default()
        }
    }

    /// [Self::call] の内部メイン処理。
    async fn call_internal(&self, ctx: T, func_name: &str, args: &FuncArgs) -> Result<String> {
        let func = self
            .call_table
            .get(func_name)
            .ok_or_else(|| anyhow!("Error: Function {func_name} not found"))?;

        // call body
        let bctx = Arc::clone(&self.basic_context);
        func(bctx, ctx, args)
            .await
            .map_err(|err| anyhow!("Error: {err}"))
    }

    /// 関数を登録する。
    pub fn register_function(&mut self, function: Function, body: FuncBody<T>) {
        let name = function.name.clone();
        self.function_list.push(function);
        self.call_table.insert(name, Box::new(body));
    }

    /// [basicfuncs] 以下のすべての基本的な関数を登録する。
    pub fn register_basic_functions(&mut self) {
        basicfuncs::register_all(self);
    }
}

/// args から引数名で文字列値を取得する。
/// 見つからない、または型が違う場合、いい感じのエラーメッセージの [anyhow::Error] を返す。
pub fn get_arg_str<'a>(args: &'a FuncArgs, name: &str) -> Result<&'a str> {
    let value = args.get(&name.to_string());
    let value = value.ok_or_else(|| anyhow!("Error: Argument {name} is required"))?;
    let value = value
        .as_str()
        .ok_or_else(|| anyhow!("Error: Argument {name} must be string"))?;

    Ok(value)
}

/// args から引数名で i64 を取得する。
/// 見つからない、または変換に失敗した場合、または範囲外の場合、
/// いい感じのエラーメッセージの [anyhow::Error] を返す。
pub fn get_arg_i64<R>(args: &FuncArgs, name: &str, range: R) -> Result<i64>
where
    R: RangeBounds<i64>,
{
    let value = args.get(&name.to_string());
    let value = value.ok_or_else(|| anyhow!("Error: Argument {name} is required"))?;
    let value = value
        .as_i64()
        .ok_or_else(|| anyhow!("Error: Argument {name} must be integer"))?;

    if range.contains(&value) {
        Ok(value)
    } else {
        bail!("Error: Out of range: {name}")
    }
}

/// args から引数名で i64 を取得する。
/// 見つからない場合は None を返す。
/// 変換に失敗した場合、または範囲外の場合、
/// いい感じのエラーメッセージの [anyhow::Error] を返す。
pub fn get_arg_i64_opt<R>(args: &FuncArgs, name: &str, range: R) -> Result<Option<i64>>
where
    R: RangeBounds<i64>,
{
    let value = args.get(&name.to_string());
    if value.is_none() {
        return Ok(None);
    }
    let value = value.ok_or_else(|| anyhow!("Error: Argument {name} is required"))?;
    let value = value
        .as_i64()
        .ok_or_else(|| anyhow!("Error: Argument {name} must be integer"))?;

    if range.contains(&value) {
        Ok(Some(value))
    } else {
        bail!("Error: Out of range: {name}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn function_args() {
        let mut args = FuncArgs::new();
        args.insert("keytest".to_string(), "ok".into());
        args.insert("int".to_string(), 42.into());
        args.insert("notint".to_string(), "abcde".into());

        assert!(get_arg_str(&args, "keytest").unwrap() == "ok");
        assert!(get_arg_str(&args, "unknown")
            .unwrap_err()
            .to_string()
            .contains("required"));

        assert!(get_arg_i64(&args, "int", 1..=42).unwrap() == 42);
        assert!(get_arg_i64(&args, "notint", 1..43)
            .unwrap_err()
            .to_string()
            .contains("must be integer"));
        assert!(get_arg_i64(&args, "int", 1..42)
            .unwrap_err()
            .to_string()
            .contains("Out of range"));

        assert!(get_arg_i64_opt(&args, "int", 1..=42).unwrap() == Some(42));
        assert!(get_arg_i64_opt(&args, "notint", 1..43)
            .unwrap_err()
            .to_string()
            .contains("must be integer"));
        assert!(get_arg_i64_opt(&args, "int", 1..42)
            .unwrap_err()
            .to_string()
            .contains("Out of range"));
    }
}
