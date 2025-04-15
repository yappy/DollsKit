//! システム情報取得。

use crate::sysmod::health::{
    ThrottleFlags, get_cpu_cores, get_cpu_info, get_current_freq, get_freq_conf,
    get_throttle_status,
};
use crate::sysmod::openai::function::{
    BasicContext, FuncArgs, Function, FunctionTable, ParameterElement, Parameters,
    get_arg_bool_opt, get_arg_str,
};
use anyhow::{Result, anyhow, bail};
use chrono::{DateTime, Local, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::Ordering;

/// このモジュールの関数をすべて登録する。
pub fn register_all<T: 'static>(func_table: &mut FunctionTable<T>) {
    register_debug_mode(func_table);
    register_get_model(func_table);
    register_get_rate_limit(func_table);
    register_get_version(func_table);
    register_get_cpu_status(func_table);
    register_get_current_datetime(func_table);
}

/// デバッグモード取得/設定。
async fn debug_mode(bctx: Arc<BasicContext>, args: &FuncArgs) -> Result<String> {
    let enabled = get_arg_bool_opt(args, "enabled")?;

    #[derive(Serialize)]
    struct FuncResult {
        current: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        previous: Option<bool>,
    }

    let result = if let Some(enabled) = enabled {
        let old = bctx.debug_mode.swap(enabled, Ordering::SeqCst);

        FuncResult {
            current: enabled,
            previous: Some(old),
        }
    } else {
        let current = bctx.debug_mode.load(Ordering::SeqCst);

        FuncResult {
            current,
            previous: None,
        }
    };

    Ok(serde_json::to_string(&result).unwrap())
}

fn register_debug_mode<T: 'static>(func_table: &mut FunctionTable<T>) {
    let mut properties = HashMap::new();
    properties.insert(
        "enabled".to_string(),
        ParameterElement {
            type_: "boolean".to_string(),
            description: Some(
                "New value. If not specified, just get the current value.".to_string(),
            ),
            ..Default::default()
        },
    );

    func_table.register_function(
        Function {
            name: "debug_mode".to_string(),
            description: Some("Get/Set debug mode of function calls".to_string()),
            parameters: Parameters {
                type_: "object".to_string(),
                properties,
                required: Default::default(),
            },
        },
        |bctx, _ctx, args| Box::pin(debug_mode(bctx, args)),
    );
}

/// モデル情報取得。
async fn get_model(bctx: Arc<BasicContext>, _args: &FuncArgs) -> Result<String> {
    let model = bctx.ctrl.sysmods().openai.lock().await.model_info().await?;

    Ok(serde_json::to_string(&model).unwrap())
}

fn register_get_model<T: 'static>(func_table: &mut FunctionTable<T>) {
    func_table.register_function(
        Function {
            name: "get_model".to_string(),
            description: Some("Get GPT model info of the assistant".to_string()),
            parameters: Parameters {
                type_: "object".to_string(),
                properties: Default::default(),
                required: Default::default(),
            },
        },
        |bctx, _ctx, args| Box::pin(get_model(bctx, args)),
    );
}

/// レートリミット情報取得。
async fn get_rate_limit(bctx: Arc<BasicContext>, _args: &FuncArgs) -> Result<String> {
    let exp = bctx
        .ctrl
        .sysmods()
        .openai
        .lock()
        .await
        .get_expected_rate_limit();

    exp.ok_or_else(|| anyhow!("No data")).map(|exp| {
        format!(
            "Remaining\nRequests: {} / {}\nTokens: {} / {}",
            exp.remaining_requests, exp.limit_requests, exp.remaining_tokens, exp.limit_tokens,
        )
    })
}

fn register_get_rate_limit<T: 'static>(func_table: &mut FunctionTable<T>) {
    func_table.register_function(
        Function {
            name: "get_rate_limit".to_string(),
            description: Some("Get rate limit info of GPT usage".to_string()),
            parameters: Parameters {
                type_: "object".to_string(),
                properties: Default::default(),
                required: Default::default(),
            },
        },
        |bctx, _ctx, args| Box::pin(get_rate_limit(bctx, args)),
    );
}

/// バージョン情報取得。
async fn get_version(_args: &FuncArgs) -> Result<String> {
    Ok(verinfo::version_info().to_string())
}

fn register_get_version<T: 'static>(func_table: &mut FunctionTable<T>) {
    func_table.register_function(
        Function {
            name: "get_version".to_string(),
            description: Some("Get version of the assistant program".to_string()),
            parameters: Parameters {
                type_: "object".to_string(),
                properties: Default::default(),
                required: Default::default(),
            },
        },
        |_, _, args| Box::pin(get_version(args)),
    );
}

#[derive(Serialize, Deserialize)]
struct CpuStatus {
    number_of_cores: u32,
    cpu_usage_percent: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    current_frequency: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    original_frequency: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature_celsius: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    throttle_status: Option<Vec<String>>,
}

/// CPU 使用率情報取得。
async fn get_cpu_status(_args: &FuncArgs) -> Result<String> {
    let cpu_info = get_cpu_info().await?;

    let number_of_cores = get_cpu_cores().await?;
    let current_frequency = get_current_freq()
        .await?
        .map(|hz| format!("{} MHz", hz / 1_000_000));
    let original_frequency = get_freq_conf()
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
        cpu_usage_percent: cpu_info.cpu_percent_total,
        current_frequency,
        original_frequency,
        temperature_celsius: cpu_info.temp,
        throttle_status,
    };

    Ok(serde_json::to_string(&obj)?)
}

fn register_get_cpu_status<T: 'static>(func_table: &mut FunctionTable<T>) {
    func_table.register_function(
        Function {
            name: "get_cpu_status".to_string(),
            description: Some("Get current status of assistant's CPU".to_string()),
            parameters: Parameters {
                type_: "object".to_string(),
                properties: Default::default(),
                required: Default::default(),
            },
        },
        |_, _, args| Box::pin(get_cpu_status(args)),
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
        |_, _, args| Box::pin(get_current_datetime(args)),
    );
}
