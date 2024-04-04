//! システム情報取得。

use crate::sysmod::health::{
    get_cpu_cores, get_current_freq, get_freq_conf, get_throttle_status, ThrottleFlags,
};
use crate::sysmod::openai::function::{
    get_arg_str, FuncArgs, FuncBodyAsync, Function, FunctionTable, ParameterElement, Parameters,
};
use anyhow::{bail, Result};
use chrono::{DateTime, Local, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// このモジュールの関数をすべて登録する。
pub fn register_all<T: 'static>(func_table: &mut FunctionTable<T>) {
    register_get_version(func_table);
    register_get_cpu_status(func_table);
    register_get_current_datetime(func_table);
}

/// バージョン情報取得。
async fn get_version(_args: &FuncArgs) -> Result<String> {
    use crate::sys::version;

    Ok(version::version_info().to_string())
}

fn get_version_pin<T>(_ctx: T, args: &FuncArgs) -> FuncBodyAsync {
    Box::pin(get_version(args))
}

fn register_get_version<T: 'static>(func_table: &mut FunctionTable<T>) {
    func_table.register_function(
        Function {
            name: "get_version".to_string(),
            description: Some("Get the version of the assistant program".to_string()),
            parameters: Parameters {
                type_: "object".to_string(),
                properties: Default::default(),
                required: Default::default(),
            },
        },
        Box::new(get_version_pin),
    );
}

#[derive(Serialize, Deserialize)]
struct CpuStatus {
    number_of_cores: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    current_frequency: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    config_frequency: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    throttle_status: Option<Vec<String>>,
}

/// CPU 使用率情報取得。
async fn get_cpu_status(_args: &FuncArgs) -> Result<String> {
    let number_of_cores = get_cpu_cores().await?;
    let current_frequency = get_current_freq()
        .await?
        .map(|hz| format!("{} MHz", hz / 1_000_000));
    let config_frequency = get_freq_conf()
        .await?
        .map(|hz| format!("{} MHz", hz / 1_000_000));
    let throttle_status = get_throttle_status().await?.map(|st| {
        let mut v = vec![];
        if st.contains(ThrottleFlags::UNDER_VOLTAGE) {
            v.push("Under Voltage".to_string());
        }
        if st.contains(ThrottleFlags::SOFT_TEMP_LIMIT) {
            v.push("Soft Throttled".to_string());
        }
        if st.contains(ThrottleFlags::THROTTLED) {
            v.push("Hard Throttled".to_string());
        }
        v
    });

    let obj = CpuStatus {
        number_of_cores,
        current_frequency,
        config_frequency,
        throttle_status,
    };

    Ok(serde_json::to_string(&obj)?)
}

fn get_cpu_status_pin<T>(_ctx: T, args: &FuncArgs) -> FuncBodyAsync {
    Box::pin(get_cpu_status(args))
}

fn register_get_cpu_status<T: 'static>(func_table: &mut FunctionTable<T>) {
    func_table.register_function(
        Function {
            name: "get_cpu_status".to_string(),
            description: Some("Get the current status of assistant's CPU".to_string()),
            parameters: Parameters {
                type_: "object".to_string(),
                properties: Default::default(),
                required: Default::default(),
            },
        },
        Box::new(get_cpu_status_pin),
    );
}

/// 現在の日時を取得する。
async fn get_current_datetime(args: &FuncArgs) -> Result<String> {
    let tz = get_arg_str(args, "tz")?;
    match tz {
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

fn get_current_datetime_pin<T>(_ctx: T, args: &FuncArgs) -> FuncBodyAsync {
    Box::pin(get_current_datetime(args))
}

fn register_get_current_datetime<T: 'static>(func_table: &mut FunctionTable<T>) {
    let mut properties = HashMap::new();
    properties.insert(
        "tz".to_string(),
        ParameterElement {
            type_: "string".to_string(),
            description: Some("Time zone".to_string()),
            enum_: Some(vec!["JST".to_string(), "UTC".to_string()]),
            ..Default::default()
        },
    );

    func_table.register_function(
        Function {
            name: "get_current_datetime".to_string(),
            description: Some("Get the current date and time".to_string()),
            parameters: Parameters {
                type_: "object".to_string(),
                properties,
                required: vec!["tz".to_string()],
            },
        },
        Box::new(get_current_datetime_pin),
    );
}
