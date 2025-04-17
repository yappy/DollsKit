//! Web アクセス関連。

use crate::sysmod::openai::ParameterType;
use crate::sysmod::openai::function::{
    FuncArgs, Function, FunctionTable, ParameterElement, Parameters, get_arg_str,
};
use anyhow::{Context, Result, anyhow};
use reqwest::Client;
use std::{collections::HashMap, time::Duration};
use utils::netutil;
use utils::weather::{self, ForecastRoot, OverviewForecast};

/// このモジュールの関数をすべて登録する。
pub fn register_all<T: 'static>(func_table: &mut FunctionTable<T>) {
    register_get_weather_areas(func_table);
    register_get_weather_report(func_table);
}

/// 気象情報地域のリストを取得する。
async fn get_weather_areas(_args: &FuncArgs) -> Result<String> {
    let area_list: Vec<_> = weather::offices()
        .iter()
        .map(|info| info.name.clone())
        .collect();

    Ok(serde_json::to_string(&area_list).unwrap())
}

fn register_get_weather_areas<T: 'static>(func_table: &mut FunctionTable<T>) {
    func_table.register_function(
        Function {
            name: "get_weather_areas".to_string(),
            description: Some("Get area list for get_weather_report".to_string()),
            parameters: Parameters {
                properties: Default::default(),
                required: Default::default(),
                ..Default::default()
            },
            ..Default::default()
        },
        |_, _, args| Box::pin(get_weather_areas(args)),
    );
}

/// 気象情報を取得する。
async fn get_weather_report(args: &FuncArgs) -> Result<String> {
    const TIMEOUT: Duration = Duration::from_secs(10);
    let area = get_arg_str(args, "area")?;

    // 引数の都市名をコードに変換
    let code = weather::office_name_to_code(area).ok_or_else(|| {
        anyhow!(
            "Invalid area: {} - You should call get_weather_areas to get valid name list.",
            area
        )
    })?;

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

fn register_get_weather_report<T: 'static>(func_table: &mut FunctionTable<T>) {
    let mut properties = HashMap::new();
    properties.insert(
        "area".to_string(),
        ParameterElement {
            type_: vec![ParameterType::String],
            description: Some(
                "Area name that list can be obtained by get_weather_areas".to_string(),
            ),
            ..Default::default()
        },
    );

    func_table.register_function(
        Function {
            name: "get_weather_report".to_string(),
            description: Some("Get whether report data".to_string()),
            parameters: Parameters {
                properties,
                required: vec!["area".to_string()],
                ..Default::default()
            },
            ..Default::default()
        },
        |_, _, args| Box::pin(get_weather_report(args)),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

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
