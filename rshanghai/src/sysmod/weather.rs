use anyhow::Result;
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
}
