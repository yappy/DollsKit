use anyhow::{ensure, Result};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::OnceLock};

/// <https://www.jma.go.jp/bosai/common/const/area.json>
///
/// 移転または何らかの理由で 404 のものがある。
/// 復活するかもしれないので削除はしないものとする。
///
/// Note: 2023/10/29
///
/// ```
/// Not found: JmaOfficeInfo { code: "014030", name: "十勝地方", en_name: "Tokachi", office_name: "帯広測候所" }
/// Not found: JmaOfficeInfo { code: "460040", name: "奄美地方", en_name: "Amami", office_name: "名瀬測候所" }
/// ```
const JMA_AREA_JSON: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/res/area.json"));

#[derive(Clone, Debug, Serialize, Deserialize)]
struct JmaAreaDef {
    offices: HashMap<String, JmaOfficeInfo>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct JmaOfficeInfo {
    /// JSON 中には存在しない。後でキーを入れる。
    #[serde(default)]
    code: String,
    name: String,
    en_name: String,
    office_name: String,
}

static OFFICE_LIST: OnceLock<Vec<JmaOfficeInfo>> = OnceLock::new();

fn offices() -> &'static Vec<JmaOfficeInfo> {
    OFFICE_LIST.get_or_init(|| {
        let root: JmaAreaDef = serde_json::from_str(JMA_AREA_JSON).unwrap();

        let mut list: Vec<_> = root
            .offices
            .iter()
            .map(|(code, info)| {
                let mut modified = info.clone();
                modified.code = code.to_string();
                modified
            })
            .collect();
        list.sort_by(|a, b| a.code.cmp(&b.code));

        list
    })
}

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
    use super::*;
    use reqwest::Client;

    #[tokio::test]
    #[ignore]
    // cargo test office_list -- --ignored --nocapture
    async fn office_list() -> Result<()> {
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
                //println!("{}", resp.text().await?);
            } else {
                println!("overview_forecast not found: {:?}", info);
            }

            let url = format!(
                "https://www.jma.go.jp/bosai/forecast/data/forecast/{}.json",
                info.code
            );
            let resp = client.get(url).send().await?;
            if resp.status().is_success() {
                //println!("{}", resp.text().await?);
            } else {
                println!("forecast not found: {:?}", info);
            }
        }

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
