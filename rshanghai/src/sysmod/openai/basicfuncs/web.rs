//! Web アクセス関連。

use crate::sysmod::openai::function::{
    get_arg_str, BasicContext, FuncArgs, FuncBodyAsync, Function, FunctionTable, ParameterElement,
    Parameters,
};
use crate::utils::netutil;
use crate::utils::weather::{self, ForecastRoot, OverviewForecast};
use anyhow::{anyhow, bail, Context, Result};
use reqwest::Client;
use std::sync::Arc;
use std::{collections::HashMap, time::Duration};

/// このモジュールの関数をすべて登録する。
pub fn register_all<T: 'static>(func_table: &mut FunctionTable<T>) {
    register_request_url(func_table);
    register_get_weather_report(func_table);
}


/// URL に対して GET リクエストを行い結果を文字列で返す。
async fn request_url(args: &FuncArgs) -> Result<String> {
    const TIMEOUT: Duration = Duration::from_secs(10);
    const SIZE_MAX: usize = 8 * 1024;
    let url = get_arg_str(args, "url")?;

    let client = Client::builder().timeout(TIMEOUT).build()?;
    let resp = client.get(url).send().await?;

    let status = resp.status();
    if status.is_success() {
        let text = resp.text().await?;

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

fn request_url_pin<T>(_bctx: Arc<BasicContext>, _ctx: T, args: &FuncArgs) -> FuncBodyAsync {
    Box::pin(request_url(args))
}

fn register_request_url<T: 'static>(func_table: &mut FunctionTable<T>) {
    let mut properties = HashMap::new();
    properties.insert(
        "url".to_string(),
        ParameterElement {
            type_: "string".to_string(),
            description: Some("URL to access".to_string()),
            enum_: None,
            ..Default::default()
        },
    );

    func_table.register_function(
        Function {
            name: "request_url".to_string(),
            description: Some("Request HTTP GET".to_string()),
            parameters: Parameters {
                type_: "object".to_string(),
                properties,
                required: vec!["url".to_string()],
            },
        },
        Box::new(request_url_pin),
    );
}

/// 気象情報を取得する。
async fn get_weather_report(args: &FuncArgs) -> Result<String> {
    const TIMEOUT: Duration = Duration::from_secs(10);
    let area = get_arg_str(args, "area")?;

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

    let ov: OverviewForecast =
        serde_json::from_str(&s1).with_context(|| format!("OverviewForecast parse error: {s1}"))?;
    let fc: ForecastRoot =
        serde_json::from_str(&s2).with_context(|| format!("ForecastRoot parse error: {s2}"))?;
    let obj = weather::weather_to_ai_readable(&code, &ov, &fc)?;

    Ok(serde_json::to_string(&obj).unwrap())
}

fn get_weather_report_pin<T>(_bctx: Arc<BasicContext>, _ctx: T, args: &FuncArgs) -> FuncBodyAsync {
    Box::pin(get_weather_report(args))
}

fn register_get_weather_report<T: 'static>(func_table: &mut FunctionTable<T>) {
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
            ..Default::default()
        },
    );

    func_table.register_function(
        Function {
            name: "get_weather_report".to_string(),
            description: Some("Get whether report data".to_string()),
            parameters: Parameters {
                type_: "object".to_string(),
                properties,
                required: vec!["area".to_string()],
            },
        },
        Box::new(get_weather_report_pin),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    #[tokio::test]
    #[ignore]
    // cargo test parse_real_html -- --ignored --nocapture
    async fn parse_real_html() -> Result<()> {
        let mut args = FuncArgs::new();
        args.insert(
            "url".into(),
            Value::String("https://www.google.co.jp/".into()),
        );

        let text = request_url(&args).await?;
        println!("{}", text);

        Ok(())
    }

    #[tokio::test]
    #[ignore]
    // cargo test weather_report -- --ignored --nocapture
    async fn weather_report() -> Result<()> {
        let mut args = FuncArgs::new();
        args.insert("area".into(), Value::String("広島県".into()));

        let text = get_weather_report(&args).await?;
        println!("{}", text);

        Ok(())
    }
}
