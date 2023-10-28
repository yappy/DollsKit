//! OpenAI API - function.

use crate::sysmod::openai::ChatMessage;

use super::{Function, ParameterElement, Parameters};
use anyhow::{anyhow, bail, Result};
use log::{info, warn};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

type FuncBody = Box<dyn Fn(&FuncArgs) -> Result<String>>;
type FuncArgs = HashMap<String, String>;

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct Args {
    #[serde(flatten)]
    args: FuncArgs,
}

pub struct FunctionTable {
    function_list: Vec<Function>,
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
        self.register_get_current_datetime();
    }

    pub fn function_list(&self) -> &Vec<Function> {
        &self.function_list
    }

    pub fn call(&self, func_name: &str, args_json_str: &str) -> ChatMessage {
        info!("[openai-func] Call {func_name} {args_json_str}");

        let res = {
            let args = serde_json::from_str::<Args>(args_json_str)
                .map_err(|err| anyhow!("Arguments parse error: {err}"));
            match args {
                Ok(args) => self.call_internal(func_name, &args.args),
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

    fn call_internal(&self, func_name: &str, args: &FuncArgs) -> Result<String> {
        let func = self
            .call_table
            .get(func_name)
            .ok_or_else(|| anyhow!("Error: Function {func_name} not found"))?;

        // call body
        func(args).map_err(|err| anyhow!("Error: {err}"))
    }
}

/// args から引数名で検索し、値への参照を返す。
/// 見つからない場合、いい感じのエラーメッセージの [anyhow::Error] を返す。
fn get_arg<'a>(args: &'a FuncArgs, name: &str) -> Result<&'a String> {
    let value = args.get(&name.to_string());
    value.ok_or_else(|| anyhow!("Error: Argument {name} is required"))
}

// TODO: get with default

fn get_current_datetime(args: &FuncArgs) -> Result<String> {
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
        // get_current_datetime
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
            .insert("get_current_datetime", Box::new(get_current_datetime));
    }
}
