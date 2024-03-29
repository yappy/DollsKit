//! OpenAI API - function.

use super::{Function, ParameterElement, Parameters};

use crate::sysmod::health::{
    get_cpu_cores, get_current_freq, get_freq_conf, get_throttle_status, ThrottleFlags,
};
use crate::utils::weather::{self, ForecastRoot, OverviewForecast};
use crate::{
    sysmod::openai::{ChatMessage, Role},
    utils::netutil,
};

use anyhow::{anyhow, bail, Result};
use chrono::{DateTime, Local, Utc};
use log::{info, warn};
use reqwest::Client;
use serde::{Deserialize, Serialize};

use std::{collections::HashMap, future::Future, pin::Pin, time::Duration};

/// Function でもトークンを消費するが、算出方法がよく分からないので定数で確保する。
/// トークン制限エラーが起きた場合、エラーメッセージ中に含まれていた気がするので
/// それより大きめに確保する。
pub const FUNCTION_TOKEN: usize = 800;

// https://users.rust-lang.org/t/how-to-handle-a-vector-of-async-function-pointers/39804

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
    call_table: HashMap<&'static str, FuncBody<T>>,
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

    pub fn register_function(&mut self, function: Function, name: &'static str, body: FuncBody<T>) {
        self.function_list.push(function);
        self.call_table.insert(name, Box::new(body));
    }

    pub fn register_basic_functions(&mut self) {
        self.register_get_version();
        self.register_get_cpu_status();
        self.register_get_current_datetime();
        self.register_request_url();
        self.register_get_weather_report();
    }
}

/// args から引数名で検索し、値への参照を返す。
/// 見つからない場合、いい感じのエラーメッセージの [anyhow::Error] を返す。
pub fn get_arg<'a>(args: &'a FuncArgs, name: &str) -> Result<&'a String> {
    let value = args.get(&name.to_string());
    value.ok_or_else(|| anyhow!("Error: Argument {name} is required"))
}

// =============================================================================

fn get_version_sync<T>(_ctx: T, args: &FuncArgs) -> FuncBodyAsync {
    Box::pin(get_version(args))
}

async fn get_version(_args: &FuncArgs) -> Result<String> {
    use crate::sys::version;

    Ok(version::version_info().to_string())
}

impl<T: 'static> FunctionTable<T> {
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

// =============================================================================
fn get_cpu_status_sync<T>(_ctx: T, args: &FuncArgs) -> FuncBodyAsync {
    Box::pin(get_cpu_status(args))
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

impl<T: 'static> FunctionTable<T> {
    fn register_get_cpu_status(&mut self) {
        self.function_list.push(Function {
            name: "get_cpu_status".to_string(),
            description: Some("Get the current status of assistant's CPU".to_string()),
            parameters: Parameters {
                type_: "object".to_string(),
                properties: Default::default(),
                required: Default::default(),
            },
        });
        self.call_table
            .insert("get_cpu_status", Box::new(get_cpu_status_sync));
    }
}

// =============================================================================
fn get_current_datetime_sync<T>(_ctx: T, args: &FuncArgs) -> FuncBodyAsync {
    Box::pin(get_current_datetime(args))
}

async fn get_current_datetime(args: &FuncArgs) -> Result<String> {
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

impl<T: 'static> FunctionTable<T> {
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

// =============================================================================

fn request_url_sync<T>(_ctx: T, args: &FuncArgs) -> FuncBodyAsync {
    Box::pin(request_url(args))
}

fn compact_html(src: &str) -> Result<String> {
    use scraper::{Html, Selector};

    let fragment = Html::parse_document(src);
    // CSS セレクタで body タグを選択
    let selector = Selector::parse("body").unwrap();
    // イテレータを返すが最初の1つだけを対象とする
    let body = fragment
        .select(&selector)
        .next()
        .ok_or_else(|| anyhow!("body not found"))?;

    // 空白文字をまとめる
    let mut res = String::new();
    let mut prev_space = false;
    // body 内のテキストノードを巡る
    for text in body.text() {
        for c in text.chars() {
            if c.is_whitespace() {
                if !prev_space {
                    res.push(' ');
                }
                prev_space = true;
            } else {
                res.push(c);
                prev_space = false;
            }
        }
    }

    Ok(res)
}

async fn request_url(args: &FuncArgs) -> Result<String> {
    const TIMEOUT: Duration = Duration::from_secs(10);
    const SIZE_MAX: usize = 5 * 1024;
    let url = get_arg(args, "url")?;

    let client = Client::builder().timeout(TIMEOUT).build()?;
    let resp = client.get(url).send().await?;

    let status = resp.status();
    if status.is_success() {
        let text = resp.text().await?;

        let text = compact_html(&text)?;
        // SIZE_MAX バイトまで抜き出す
        if text.len() > SIZE_MAX {
            let mut end = 0;
            for (i, _c) in text.char_indices() {
                if i < SIZE_MAX {
                    end = i;
                }
            }
            Ok(text[0..end].to_string())
        } else {
            Ok(text.to_string())
        }
    } else {
        bail!(
            "{}, {}",
            status.as_str(),
            status.canonical_reason().unwrap_or("")
        );
    }
}

impl<T: 'static> FunctionTable<T> {
    fn register_request_url(&mut self) {
        let mut properties = HashMap::new();
        properties.insert(
            "url".to_string(),
            ParameterElement {
                type_: "string".to_string(),
                description: Some("URL to access".to_string()),
                enum_: None,
            },
        );
        self.function_list.push(Function {
            name: "request_url".to_string(),
            description: Some("Request HTTP GET".to_string()),
            parameters: Parameters {
                type_: "object".to_string(),
                properties,
                required: vec!["url".to_string()],
            },
        });
        self.call_table
            .insert("request_url", Box::new(request_url_sync));
    }
}

// =============================================================================

fn get_weather_report_sync<T>(_ctx: T, args: &FuncArgs) -> FuncBodyAsync {
    Box::pin(get_weather_report(args))
}

async fn get_weather_report(args: &FuncArgs) -> Result<String> {
    const TIMEOUT: Duration = Duration::from_secs(10);
    let area = get_arg(args, "area")?;

    // 引数の都市名をコードに変換
    let code =
        weather::office_name_to_code(area).ok_or_else(|| anyhow!("Invalid area: {}", area))?;

    let url1 = weather::url_overview_forecast(&code);
    let url2 = weather::url_forecast(&code);
    let client = Client::builder().timeout(TIMEOUT).build()?;

    let fut1 = netutil::checked_get_url(&client, &url1);
    let fut2 = netutil::checked_get_url(&client, &url2);
    let (resp1, resp2) = tokio::join!(fut1, fut2);
    let (s1, s2) = (resp1?, resp2?);

    let ov: OverviewForecast = serde_json::from_str(&s1)?;
    let fc: ForecastRoot = serde_json::from_str(&s2)?;
    let obj = weather::weather_to_ai_readable(&code, &ov, &fc)?;

    Ok(serde_json::to_string(&obj).unwrap())
}

impl<T: 'static> FunctionTable<T> {
    fn register_get_weather_report(&mut self) {
        let area_list: Vec<_> = weather::offices()
            .iter()
            .map(|info| info.name.clone())
            .collect();

        let mut properties = HashMap::new();
        properties.insert(
            "area".to_string(),
            ParameterElement {
                type_: "string".to_string(),
                description: Some("Area name (city name, etc.)".to_string()),
                enum_: Some(area_list),
            },
        );
        self.function_list.push(Function {
            name: "get_weather_report".to_string(),
            description: Some("Get whether report data".to_string()),
            parameters: Parameters {
                type_: "object".to_string(),
                properties,
                required: vec!["area".to_string()],
            },
        });
        self.call_table
            .insert("get_weather_report", Box::new(get_weather_report_sync));
    }
}

// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_html() -> Result<()> {
        const SRC: &str = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/res/test/scraping/top.htm"
        ));

        let res = compact_html(SRC)?;
        println!("{res}");

        Ok(())
    }
}
