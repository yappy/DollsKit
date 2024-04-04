//! Web アクセス関連。

use crate::sysmod::openai::function::{
    get_arg_str, FuncArgs, FuncBodyAsync, Function, FunctionTable, ParameterElement, Parameters,
};
use crate::utils::netutil;
use crate::utils::weather::{self, ForecastRoot, OverviewForecast};
use anyhow::{anyhow, bail, Result};
use reqwest::Client;
use std::{collections::HashMap, time::Duration};

/// このモジュールの関数をすべて登録する。
pub fn register_all<T: 'static>(func_table: &mut FunctionTable<T>) {
    register_request_url(func_table);
    register_get_weather_report(func_table);
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
    let url = get_arg_str(args, "url")?;

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

fn request_url_pin<T>(_ctx: T, args: &FuncArgs) -> FuncBodyAsync {
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

    let ov: OverviewForecast = serde_json::from_str(&s1)?;
    let fc: ForecastRoot = serde_json::from_str(&s2)?;
    let obj = weather::weather_to_ai_readable(&code, &ov, &fc)?;

    Ok(serde_json::to_string(&obj).unwrap())
}

fn get_weather_report_pin<T>(_ctx: T, args: &FuncArgs) -> FuncBodyAsync {
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
