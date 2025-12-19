//! システム情報取得。

use crate::rpienv::{self, CameraInfo, RaspiEnv};
use crate::sysmod::health::{
    ThrottleFlags, get_cpu_cores, get_cpu_info, get_disk_info, get_mem_info, get_throttle_status,
};
use crate::sysmod::openai::function::{
    BasicContext, FuncArgs, Function, FunctionTable, ParameterElement, Parameters,
    get_arg_bool_opt, get_arg_str,
};
use crate::sysmod::openai::{ModelInfo, ParameterType};
use anyhow::{Result, anyhow, bail};
use chrono::{DateTime, Local, Utc};
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use verinfo::VersionInfo;

/// このモジュールの関数をすべて登録する。
pub fn register_all<T: 'static>(func_table: &mut FunctionTable<T>) {
    register_debug_mode(func_table);
    register_get_assistant_info(func_table);
    //register_get_rate_limit(func_table);
    register_get_current_datetime(func_table);
}

/// デバッグモード取得/設定。
async fn debug_mode(bctx: Arc<BasicContext>, args: &FuncArgs) -> Result<String> {
    let enabled = get_arg_bool_opt(args, "enabled")?;

    #[skip_serializing_none]
    #[derive(Serialize)]
    struct FuncResult {
        current: bool,
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
            type_: vec![ParameterType::Boolean, ParameterType::Null],
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
                properties,
                required: vec!["enabled".to_string()],
                ..Default::default()
            },
            ..Default::default()
        },
        |bctx, _ctx, args| Box::pin(debug_mode(bctx, args)),
    );
}

/// AI アシスタント情報取得
async fn get_assistant_info(bctx: Arc<BasicContext>, _args: &FuncArgs) -> Result<String> {
    let model = bctx.ctrl.sysmods().openai.lock().await.model_info().await?;
    let build_info = verinfo::version_info_struct();
    let rpienv = rpienv::raspi_env();
    let cpu = get_cpu_status().await?;
    let memory = get_memory_status().await?;
    let disk = get_disk_status().await?;

    #[skip_serializing_none]
    #[derive(Serialize)]
    struct RpiEnv {
        model: &'static str,
        cameras: Option<&'static Vec<CameraInfo>>,
    }
    let rpienv = match rpienv {
        RaspiEnv::RasRi { model, cameras } => {
            let cameras = Some(cameras).filter(|v| !v.is_empty());
            RpiEnv {
                model: &model,
                cameras,
            }
        }
        RaspiEnv::NotRasRi => RpiEnv {
            model: "Not Raspberry Pi",
            cameras: None,
        },
    };

    #[derive(Serialize)]
    struct Info {
        ai_model: ModelInfo,
        build: &'static VersionInfo,
        env: RpiEnv,
        cpu: CpuStatus,
        memory: MemoryStatus,
        disk: DiskStatus,
    }
    let info = Info {
        ai_model: model,
        build: build_info,
        env: rpienv,
        cpu,
        memory,
        disk,
    };

    Ok(serde_json::to_string(&info).unwrap())
}

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize)]
struct CpuStatus {
    number_of_cores: u32,
    usage_percent: f32,
    temperature_celsius: Option<f32>,
    throttle_status: Option<Vec<String>>,
}

/// CPU 使用率情報取得。
async fn get_cpu_status() -> Result<CpuStatus> {
    let cpu_info = get_cpu_info().await?;

    let number_of_cores = get_cpu_cores().await?;
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
    let throttle_status = throttle_status.filter(|v| !v.is_empty());

    Ok(CpuStatus {
        number_of_cores,
        usage_percent: cpu_info.cpu_percent_total as f32,
        temperature_celsius: cpu_info.temp.map(|t| t as f32),
        throttle_status,
    })
}

#[derive(Serialize)]
struct MemoryStatus {
    total_gib: f32,
    available_gib: f32,
    usage_percent: f32,
}

/// メモリ使用量取得。
async fn get_memory_status() -> Result<MemoryStatus> {
    let mem_info = get_mem_info().await?;
    let usage_percent =
        (((mem_info.total_mib - mem_info.avail_mib) / mem_info.total_mib) * 100.0) as f32;

    Ok(MemoryStatus {
        total_gib: (mem_info.total_mib / 1024.0) as f32,
        available_gib: (mem_info.avail_mib / 1024.0) as f32,
        usage_percent,
    })
}

#[derive(Serialize)]
struct DiskStatus {
    total_gib: f32,
    available_gib: f32,
    usage_percent: f32,
}

/// ディスク使用量取得。
async fn get_disk_status() -> Result<DiskStatus> {
    let disk_info = get_disk_info().await?;
    let usage_percent =
        (((disk_info.total_gib - disk_info.avail_gib) / disk_info.total_gib) * 100.0) as f32;

    Ok(DiskStatus {
        total_gib: disk_info.total_gib as f32,
        available_gib: disk_info.avail_gib as f32,
        usage_percent,
    })
}

fn register_get_assistant_info<T: 'static>(func_table: &mut FunctionTable<T>) {
    func_table.register_function(
        Function {
            name: "get_assistant_info".to_string(),
            description: Some("AI model, build info, hardware env, cpu/memory/disk".to_string()),
            parameters: Parameters {
                properties: Default::default(),
                required: Default::default(),
                ..Default::default()
            },
            ..Default::default()
        },
        |bctx, _ctx, args| Box::pin(get_assistant_info(bctx, args)),
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

#[allow(dead_code)]
fn register_get_rate_limit<T: 'static>(func_table: &mut FunctionTable<T>) {
    func_table.register_function(
        Function {
            name: "get_rate_limit".to_string(),
            description: Some("Get rate limit info of GPT usage".to_string()),
            parameters: Parameters {
                properties: Default::default(),
                required: Default::default(),
                ..Default::default()
            },
            ..Default::default()
        },
        |bctx, _ctx, args| Box::pin(get_rate_limit(bctx, args)),
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
            type_: vec![ParameterType::String],
            description: Some("Time zone".to_string()),
            enum_: Some(vec!["JST".to_string(), "UTC".to_string()]),
        },
    );

    func_table.register_function(
        Function {
            name: "get_current_datetime".to_string(),
            description: Some("Get the current date and time".to_string()),
            parameters: Parameters {
                properties,
                required: vec!["tz".to_string()],
                ..Default::default()
            },
            ..Default::default()
        },
        |_, _, args| Box::pin(get_current_datetime(args)),
    );
}
