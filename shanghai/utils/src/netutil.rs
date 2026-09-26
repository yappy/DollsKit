//! HTTP 通信や SHA 計算等のユーティリティ。
//!
use anyhow::{Context, Result, anyhow};
use hmac::{KeyInit, Mac, SimpleHmac};
use reqwest::{Client, RequestBuilder, Response};
use serde::Deserialize;
use sha2::Sha256;
use std::time::{Duration, Instant};
use thiserror::Error;

const RETRY_TIMEOUT: Duration = Duration::from_secs(5);
const RETRY_INTERVAL: Duration = Duration::from_millis(500);

pub async fn send_with_retry(mut build_req: impl FnMut() -> RequestBuilder) -> Result<Response> {
    let start = Instant::now();
    loop {
        let res = build_req().send().await.context("HTTP request failed");
        if let Err(ref err) = res
            && Instant::now() - start < RETRY_TIMEOUT
        {
            log::error!("{err:#?}");
            log::warn!("HTTP request failed, retrying...");
            tokio::time::sleep(RETRY_INTERVAL).await;
        } else {
            break res;
        }
    }
}

#[derive(Debug, Error)]
#[error("Http error {status} {body}")]
pub struct HttpStatusError {
    pub status: u16,
    pub body: String,
}

/// HTTP status が成功 (200 台) でなければ Err に変換する。
///
/// 成功ならば response body を文字列に変換して返す。
pub async fn check_http_resp(resp: reqwest::Response) -> Result<String> {
    let status = resp.status();
    let text = resp.text().await?;

    if status.is_success() {
        Ok(text)
    } else {
        Err(anyhow!(HttpStatusError {
            status: status.as_u16(),
            body: text
        }))
    }
}

/// HTTP status が成功 (200 台) でなければ Err に変換する。
///
/// 成功ならば response body をバイト列に変換して返す。
#[allow(unused)]
pub async fn check_http_resp_bin(resp: reqwest::Response) -> Result<Vec<u8>> {
    let status = resp.status();

    if status.is_success() {
        let bin = resp.bytes().await?.to_vec();
        Ok(bin)
    } else {
        let text = resp.text().await?;
        Err(anyhow!(HttpStatusError {
            status: status.as_u16(),
            body: text
        }))
    }
}

/// [send_with_retry] [check_http_resp] 付きの GET。
pub async fn checked_get_url(client: &Client, url: &str) -> Result<String> {
    let resp = send_with_retry(|| client.get(url)).await?;

    check_http_resp(resp).await
}

pub async fn checked_get_url_bin(client: &Client, url: &str) -> Result<Vec<u8>> {
    let resp = send_with_retry(|| client.get(url)).await?;

    check_http_resp_bin(resp).await
}

/// 文字列を JSON としてパースし、T 型に変換する。
///
/// 変換エラーが発生した場合はエラーにソース文字列を付加する。
pub fn convert_from_json<'a, T>(json_str: &'a str) -> Result<T>
where
    T: Deserialize<'a>,
{
    let obj = serde_json::from_str::<T>(json_str)
        .with_context(|| format!("JSON parse failed: {json_str}"))?;

    Ok(obj)
}

pub fn html_escape(src: &str) -> String {
    let mut result = String::new();
    for c in src.chars() {
        match c {
            '&' => result.push_str("&amp;"),
            '"' => result.push_str("&quot;"),
            '\'' => result.push_str("&apos;"),
            '<' => result.push_str("&lt;"),
            '>' => result.push_str("&gt;"),
            _ => result.push(c),
        }
    }

    result
}

pub type HmacSha256 = SimpleHmac<Sha256>;

/// HMAC SHA2 を計算して検証する。
pub fn hmac_sha256_verify(key: &[u8], data: &[u8], expected: &[u8]) -> Result<()> {
    let mut mac = HmacSha256::new_from_slice(key).unwrap();
    mac.update(data);
    mac.verify_slice(expected)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn html_escape_1() {
        let str = "\"<a href='test'>Test&Test</a>\"";
        let result = html_escape(str);
        let expected = "&quot;&lt;a href=&apos;test&apos;&gt;Test&amp;Test&lt;/a&gt;&quot;";
        assert_eq!(result, expected);
    }
}
