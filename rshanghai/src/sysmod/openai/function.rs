//! OpenAI API - function.

use super::basicfuncs;
use crate::sysmod::openai::{ChatMessage, Role};
use anyhow::{anyhow, Result};
use log::{info, warn};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;

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
/// 引数は T, [FuncArgs] で、返り値は文字列の async fn。
pub type FuncBody<T> = Box<dyn Fn(T, &FuncArgs) -> FuncBodyAsync + Sync + Send>;
/// 引数。文字列から文字列へのマップ。
pub type FuncArgs = HashMap<String, String>;

/// 引数は JSON ソース文字列で与えられる。
/// デシリアライズでパースするための構造体。
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct Args {
    #[serde(flatten)]
    args: FuncArgs,
}

/// 関数群の管理。
pub struct FunctionTable<T> {
    /// OpenAI API に渡すためのリスト。
    function_list: Vec<Function>,
    /// 関数名から Rust 関数へのマップ。
    call_table: HashMap<String, FuncBody<T>>,
}

impl<T: 'static> FunctionTable<T> {
    pub fn new() -> Self {
        Self {
            function_list: Default::default(),
            call_table: Default::default(),
        }
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
        func(ctx, args).await.map_err(|err| anyhow!("Error: {err}"))
    }

    pub fn register_function(&mut self, function: Function, body: FuncBody<T>) {
        let name = function.name.clone();
        self.function_list.push(function);
        self.call_table.insert(name, Box::new(body));
    }

    pub fn register_basic_functions(&mut self) {
        basicfuncs::register_all(self);
    }
}

/// args から引数名で検索し、値への参照を返す。
/// 見つからない場合、いい感じのエラーメッセージの [anyhow::Error] を返す。
pub fn get_arg<'a>(args: &'a FuncArgs, name: &str) -> Result<&'a String> {
    let value = args.get(&name.to_string());
    value.ok_or_else(|| anyhow!("Error: Argument {name} is required"))
}
