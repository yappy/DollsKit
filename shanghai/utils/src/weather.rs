//! 気象情報。
//! 気象庁の非公式 API へアクセスする。
//!
//! * office-code: 気象台のコード。北海道以外は概ね都道府県ごと。
//! * class10-code: 一次細分区域。天気予報を行う区分。
//! * class20-code: 二次細分区域。天気予報を行う区分。
//!
//! <https://www.jma.go.jp/jma/kishou/know/saibun/>
//!
//! 参考
//! <https://github.com/misohena/el-jma/blob/main/docs/how-to-get-jma-forecast.org>

use anyhow::{Result, anyhow, ensure};
use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashMap},
    sync::LazyLock,
};

/// <https://www.jma.go.jp/bosai/common/const/area.json>
///
/// 例: 東京都: 130000
///
/// 移転または何らかの理由で 404 のものがある。
/// 復活するかもしれないので削除はしないものとする。
///
/// Note: 2023/10/29
///
/// ```text
/// Not found: JmaOfficeInfo { code: "014030", name: "十勝地方", en_name: "Tokachi", office_name: "帯広測候所" }
/// Not found: JmaOfficeInfo { code: "460040", name: "奄美地方", en_name: "Amami", office_name: "名瀬測候所" }
/// ```
const JMA_AREA_JSON: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/res/area.json"));

/// <https://www.jma.go.jp/bosai/forecast/>
///
/// JavaScript 上の定数データ。
/// ブラウザのコンソールで Forecast.Const.TELOPS を JSON.stringify() して入手。
///
/// `[昼画像,夜画像,?,日本語,英語]`
const JMA_TELOPLS_JSON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/res/forecast_telops.json"
));

#[derive(Clone, Debug, Serialize, Deserialize)]
struct JmaAreaDef {
    offices: BTreeMap<String, JmaOfficeInfo>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JmaOfficeInfo {
    /// JSON 中には存在しない。後でキーを入れる。
    #[serde(default)]
    pub code: String,
    pub name: String,
    pub en_name: String,
    pub office_name: String,
}

static OFFICE_LIST: LazyLock<Vec<JmaOfficeInfo>> = LazyLock::new(office_list);
static WEATHER_CODE_MAP: LazyLock<HashMap<String, String>> = LazyLock::new(weather_code_map);

fn office_list() -> Vec<JmaOfficeInfo> {
    let root: JmaAreaDef = serde_json::from_str(JMA_AREA_JSON).unwrap();

    let list: Vec<_> = root
        .offices
        .iter()
        .map(|(code, info)| {
            let mut modified = info.clone();
            modified.code = code.to_string();
            modified
        })
        .collect();

    list
}

pub fn offices() -> &'static Vec<JmaOfficeInfo> {
    &OFFICE_LIST
}

pub fn office_name_to_code(name: &str) -> Option<String> {
    offices()
        .iter()
        .find(|&info| info.name == name)
        .map(|info| info.code.to_string())
}

fn weather_code_map() -> HashMap<String, String> {
    let mut result = HashMap::new();

    type RawObj = HashMap<String, [String; 5]>;
    let obj: RawObj = serde_json::from_str(JMA_TELOPLS_JSON).unwrap();
    for (k, v) in obj.iter() {
        // 日本語名称
        result.insert(k.to_string(), v[3].to_string());
    }

    result
}

pub fn weather_code_to_string(code: &str) -> Result<&str> {
    WEATHER_CODE_MAP
        .get(code)
        .map(String::as_str)
        .ok_or_else(|| anyhow!("Weather code not found: {code}"))
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OverviewForecast {
    pub publishing_office: String,
    pub report_datetime: String,
    pub target_area: String,
    pub headline_text: String,
    pub text: String,
}

pub type ForecastRoot = Vec<Forecast>;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Forecast {
    pub publishing_office: String,
    pub report_datetime: String,
    pub time_series: Vec<TimeSeries>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimeSeries {
    pub time_defines: Vec<String>,
    pub areas: Vec<AreaArrayElement>,
    pub temp_average: Option<TempPrecipAverage>,
    pub precip_average: Option<TempPrecipAverage>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AreaArrayElement {
    pub area: Area,
    #[serde(flatten)]
    pub data: AreaData,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Area {
    pub name: String,
    pub code: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AreaData {
    // [1]
    /// 明日から7日間
    #[serde(rename_all = "camelCase")]
    WheatherPop {
        weather_codes: Vec<String>,
        pops: Vec<String>,
        reliabilities: Vec<String>,
    },
    /// 明日から7日間
    #[serde(rename_all = "camelCase")]
    DetailedTempreture {
        temps_min: Vec<String>,
        temps_min_upper: Vec<String>,
        temps_min_lower: Vec<String>,
        temps_max: Vec<String>,
        temps_max_upper: Vec<String>,
        temps_max_lower: Vec<String>,
    },

    // [0]
    // 今日から3日分
    #[serde(rename_all = "camelCase")]
    Wheather {
        weather_codes: Vec<String>,
        weathers: Vec<String>,
        winds: Vec<String>,
        // 海がない地方がある
        #[serde(default)]
        waves: Vec<String>,
    },
    /// 今日から6時間ごと、5回分
    #[serde(rename_all = "camelCase")]
    Pop { pops: Vec<String> },
    /// 明日の 0:00+9:00 と 9:00+9:00 (最低気温と最高気温)
    #[serde(rename_all = "camelCase")]
    Tempreture { temps: Vec<String> },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TempPrecipAverage {
    area: Area,
    min: String,
    max: String,
}

/// office_code から overview_forecast URL を得る。
pub fn url_overview_forecast(office_code: &str) -> String {
    format!("https://www.jma.go.jp/bosai/forecast/data/overview_forecast/{office_code}.json")
}

/// office_code から forecast URL を得る。
pub fn url_forecast(office_code: &str) -> String {
    format!("https://www.jma.go.jp/bosai/forecast/data/forecast/{office_code}.json")
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AiReadableWeather {
    url_for_more_info: String,
    now: String,

    publishing_office: String,
    report_datetime: String,

    /// DateStr => DateDataElem
    date_data: BTreeMap<String, DateDataElem>,

    target_area: String,
    headline: String,
    overview: String,
}

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
struct DateDataElem {
    #[serde(skip_serializing_if = "Option::is_none")]
    weather_pop_area: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    weather: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pop: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tempreture_area: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temp_min: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temp_max: Option<String>,
}

/// AI にも読みやすい JSON に整形する。
pub fn weather_to_ai_readable(
    office_code: &str,
    ov: &OverviewForecast,
    fcr: &ForecastRoot,
) -> Result<AiReadableWeather> {
    let mut date_data: BTreeMap<String, DateDataElem> = BTreeMap::new();

    for fc in fcr.iter() {
        for ts in fc.time_series.iter() {
            let td = &ts.time_defines;
            let areas = &ts.areas;
            if areas.is_empty() {
                continue;
            }
            // "areas" データの中で最初のものを代表して使う
            let area = &areas[0];
            for (i, dt_str) in td.iter().enumerate() {
                fill_by_element(&mut date_data, i, dt_str, area)?;
            }
        }
    }

    let now: DateTime<Local> = Local::now();
    Ok(AiReadableWeather {
        url_for_more_info: format!(
            "https://www.jma.go.jp/bosai/forecast/#area_type=offices&area_code={office_code}"
        ),
        now: now.to_string(),

        publishing_office: ov.publishing_office.to_string(),
        report_datetime: ov.report_datetime.to_string(),

        date_data,

        target_area: ov.target_area.to_string(),
        headline: ov.headline_text.to_string(),
        overview: ov.text.to_string(),
    })
}

fn fill_by_element(
    result: &mut BTreeMap<String, DateDataElem>,
    idx: usize,
    dt_str: &str,
    area: &AreaArrayElement,
) -> Result<()> {
    match &area.data {
        AreaData::WheatherPop {
            weather_codes,
            pops,
            reliabilities: _,
        } => {
            // 日時文字列で検索、なければデフォルトで新規作成する
            let v = result.entry(dt_str.to_string()).or_default();
            let weather_code = weather_codes
                .get(idx)
                .ok_or_else(|| anyhow!("Parse error"))?;
            let pop = pops.get(idx).ok_or_else(|| anyhow!("Parse error"))?;

            v.weather_pop_area = Some(area.area.name.to_string());
            if v.weather.is_none() && !weather_code.is_empty() {
                v.weather = Some(weather_code_to_string(weather_code)?.to_string());
            }
            if v.pop.is_none() && !pop.is_empty() {
                v.pop = Some(format!("{}%", &pop));
            }
        }
        AreaData::DetailedTempreture {
            temps_min,
            temps_min_upper: _,
            temps_min_lower: _,
            temps_max,
            temps_max_upper: _,
            temps_max_lower: _,
        } => {
            let v = result.entry(dt_str.to_string()).or_default();
            let min = temps_min.get(idx).ok_or_else(|| anyhow!("Parse error"))?;
            let max = temps_max.get(idx).ok_or_else(|| anyhow!("Parse error"))?;

            v.tempreture_area = Some(area.area.name.to_string());
            if !min.is_empty() {
                v.temp_min = Some(min.to_string());
            }
            if !max.is_empty() {
                v.temp_max = Some(max.to_string());
            }
        }
        AreaData::Wheather {
            weather_codes: _,
            weathers,
            winds: _,
            waves: _,
        } => {
            let v = result.entry(dt_str.to_string()).or_default();
            let weather = weathers.get(idx).ok_or_else(|| anyhow!("Parse error"))?;

            v.weather_pop_area = Some(area.area.name.to_string());
            v.weather = Some(weather.to_string());
        }
        AreaData::Pop { pops } => {
            let pop = pops.get(idx).ok_or_else(|| anyhow!("Parse error"))?;

            // 既にキーがあるときのみ
            result.entry(dt_str.to_string()).and_modify(|v| {
                v.pop = Some(pop.to_string());
            });
        }
        AreaData::Tempreture { temps } => {
            let dt = format!("{}T00:00:00+09:00", &dt_str[0..10]);
            let temp = temps.get(idx).ok_or_else(|| anyhow!("Parse error"))?;
            result.entry(dt).and_modify(|v| {
                if idx == 0 {
                    v.temp_min = Some(temp.to_string());
                } else if idx == 1 {
                    v.temp_max = Some(temp.to_string());
                }
            });
        }
    }
    Ok(())
}

#[allow(unused)]
fn edit_distance_normalized(a: &str, b: &str) -> Result<f32> {
    let maxlen = a.len().max(b.len());
    if maxlen == 0 {
        return Ok(0.0);
    }

    let dis = edit_distance(a, b)?;
    Ok(dis as f32 / maxlen as f32)
}

/// <https://ja.wikipedia.org/wiki/%E3%83%AC%E3%83%BC%E3%83%99%E3%83%B3%E3%82%B7%E3%83%A5%E3%82%BF%E3%82%A4%E3%83%B3%E8%B7%9D%E9%9B%A2>
///
/// O(mn)
#[allow(unused)]
fn edit_distance(a: &str, b: &str) -> Result<u32> {
    ensure!(a.len() < 1024);
    ensure!(b.len() < 1024);

    let a: Vec<_> = a.chars().collect();
    let b: Vec<_> = b.chars().collect();
    let pitch = b.len() + 1;
    let mut dp = vec![0u16; (a.len() + 1) * (b.len() + 1)];
    let idx = |ia: usize, ib: usize| -> usize { ia * pitch + ib };

    for ia in 0..=a.len() {
        dp[idx(ia, 0)] = ia as u16;
    }
    for ib in 0..=b.len() {
        dp[idx(0, ib)] = ib as u16;
    }

    for ia in 1..=a.len() {
        for ib in 1..=b.len() {
            let cost = if a[ia - 1] == b[ib - 1] { 0 } else { 1 };
            let d1 = dp[idx(ia - 1, ib)] + 1;
            let d2 = dp[idx(ia, ib - 1)] + 1;
            let d3 = dp[idx(ia - 1, ib - 1)] + cost;
            dp[idx(ia, ib)] = d1.min(d2).min(d3);
        }
    }

    Ok(dp[idx(a.len(), b.len())] as u32)
}

#[cfg(test)]
mod tests {
    use std::{
        fs::{self, File},
        io::Write,
    };

    use super::*;
    use reqwest::Client;
    use serde_json::Value;

    #[tokio::test]
    #[ignore]
    // cargo test weather_test_json -- --ignored --nocapture
    async fn weather_test_json() -> Result<()> {
        let olist = offices();
        println!("Office count: {}", olist.len());
        let client = Client::new();
        for info in olist.iter() {
            let url = format!(
                "https://www.jma.go.jp/bosai/forecast/data/overview_forecast/{}.json",
                info.code
            );
            let resp = client.get(url).send().await?;
            if resp.status().is_success() {
                let mut file = File::create(format!(
                    "{}/res/test/weather/overview_forecast/{}.json",
                    env!("CARGO_MANIFEST_DIR"),
                    info.code
                ))?;
                let value: Value = serde_json::from_str(&resp.text().await?)?;
                let text = serde_json::to_string_pretty(&value)? + "\n";
                file.write_all(text.as_bytes())?;
            } else {
                println!("overview_forecast not found: {:?}", info);
            }

            let url = format!(
                "https://www.jma.go.jp/bosai/forecast/data/forecast/{}.json",
                info.code
            );
            let resp = client.get(url).send().await?;
            if resp.status().is_success() {
                let mut file = File::create(format!(
                    "{}/res/test/weather/forecast/{}.json",
                    env!("CARGO_MANIFEST_DIR"),
                    info.code
                ))?;
                let value: Value = serde_json::from_str(&resp.text().await?)?;
                let text = serde_json::to_string_pretty(&value)? + "\n";
                file.write_all(text.as_bytes())?;
            } else {
                println!("forecast not found: {:?}", info);
            }

            let url = format!(
                "https://www.jma.go.jp/bosai/forecast/data/overview_week/{}.json",
                info.code
            );
            let resp = client.get(url).send().await?;
            if resp.status().is_success() {
                let mut file = File::create(format!(
                    "{}/res/test/weather/overview_week/{}.json",
                    env!("CARGO_MANIFEST_DIR"),
                    info.code
                ))?;
                let value: Value = serde_json::from_str(&resp.text().await?)?;
                let text = serde_json::to_string_pretty(&value)? + "\n";
                file.write_all(text.as_bytes())?;
            } else {
                println!("overview_week not found: {:?}", info);
            }
        }

        Ok(())
    }

    #[test]
    fn parse_overview_forecast() -> Result<()> {
        let ents = fs::read_dir(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/res/test/weather/overview_forecast"
        ))?;

        let mut count = 0;
        for ent in ents {
            let src = fs::read_to_string(ent?.path())?;
            let _: OverviewForecast = serde_json::from_str(&src)?;
            count += 1;
        }
        assert!(count > 40);

        Ok(())
    }

    #[test]
    fn parse_forecast() -> Result<()> {
        let ents = fs::read_dir(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/res/test/weather/forecast"
        ))?;

        let mut count = 0;
        for ent in ents {
            let src = fs::read_to_string(ent?.path())?;
            let _: ForecastRoot = serde_json::from_str(&src)?;
            count += 1;
        }
        assert!(count > 40);

        Ok(())
    }

    #[test]
    fn weather_code() -> Result<()> {
        assert_eq!("晴", weather_code_to_string("100")?);
        assert_eq!("晴時々曇", weather_code_to_string("101")?);
        assert_eq!("雪で雷を伴う", weather_code_to_string("450")?);

        Ok(())
    }

    #[test]
    // cargo test ai_readable -- --nocapture
    fn ai_readable() -> Result<()> {
        let src = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/res/test/weather/overview_forecast/130000.json"
        ));
        let ov: OverviewForecast = serde_json::from_str(src)?;
        let src = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/res/test/weather/forecast/130000.json"
        ));
        let fcr: ForecastRoot = serde_json::from_str(src)?;

        let obj = weather_to_ai_readable("130000", &ov, &fcr)?;
        println!("{}", serde_json::to_string_pretty(&obj).unwrap());

        Ok(())
    }

    #[test]
    fn edit_distance_test() {
        assert_eq!(3, edit_distance("", "abc").unwrap());
        assert_eq!(3, edit_distance("def", "").unwrap());
        assert_eq!(3, edit_distance("kitten", "sitting").unwrap());

        assert_eq!(4, edit_distance("カラクリ", "ボンゴレ").unwrap());
        assert_eq!(4, edit_distance("テスト", "テストパターン").unwrap());
    }
}
