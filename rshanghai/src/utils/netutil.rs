//! URL encoding や SHA 計算等のユーティリティ。
//!
use anyhow::{anyhow, Context, Result};
use hmac::{digest::CtOutput, Mac, SimpleHmac};
use percent_encoding::{utf8_percent_encode, AsciiSet};
use reqwest::Client;
use serde::Deserialize;
use sha1::Sha1;
use sha2::Sha256;
use thiserror::Error;

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

/// [check_http_resp] 付きの GET。
pub async fn checked_get_url(client: &Client, url: &str) -> Result<String> {
    let resp = client.get(url).send().await?;

    check_http_resp(resp).await
}

/// 文字列を JSON としてパースし、T 型に変換する。
///
/// 変換エラーが発生した場合はエラーにソース文字列を付加する。
pub fn convert_from_json<'a, T>(json_str: &'a str) -> Result<T>
where
    T: Deserialize<'a>,
{
    let obj = serde_json::from_str::<T>(json_str)
        .with_context(|| format!("JSON parse failed: {}", json_str))?;

    Ok(obj)
}

/// [percent_encode] で変換する文字セット。
///
///  curl_easy_escape() と同じ。
const FRAGMENT: &AsciiSet = &percent_encoding::NON_ALPHANUMERIC
    .remove(b'-')
    .remove(b'.')
    .remove(b'_')
    .remove(b'~');

pub fn percent_encode(input: &str) -> String {
    utf8_percent_encode(input, FRAGMENT).to_string()
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

pub type HmacSha1 = SimpleHmac<Sha1>;
pub type HmacSha256 = SimpleHmac<Sha256>;

/// HMAC SHA1 を計算する。
///
/// 返り値は constant time 比較可能なオブジェクト。
/// into_bytes() で内部のバイト列を取得できるが、一致検証に用いる場合は
/// タイミング攻撃回避のため通常の比較をしてはならない。
pub fn hmac_sha1(key: &[u8], data: &[u8]) -> CtOutput<HmacSha1> {
    let mut mac = HmacSha1::new_from_slice(key).unwrap();
    mac.update(data);

    mac.finalize()
}

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
    use hex_literal::hex;

    // https://developer.twitter.com/en/docs/authentication/oauth-1-0a/percent-encoding-parameters
    #[test]
    fn percent_encode_twitter_1() {
        let str = "Ladies + Gentlemen";
        let result = percent_encode(str);
        let expected = "Ladies%20%2B%20Gentlemen";
        assert_eq!(result, expected);
    }

    #[test]
    fn percent_encode_twitter_2() {
        let str = "An encoded string!";
        let result = percent_encode(str);
        let expected = "An%20encoded%20string%21";
        assert_eq!(result, expected);
    }

    #[test]
    fn percent_encode_twitter_3() {
        let str = "Dogs, Cats & Mice";
        let result = percent_encode(str);
        let expected = "Dogs%2C%20Cats%20%26%20Mice";
        assert_eq!(result, expected);
    }

    #[test]
    fn percent_encode_twitter_4() {
        let str = "☃";
        let result = percent_encode(str);
        let expected = "%E2%98%83";
        assert_eq!(result, expected);
    }

    #[test]
    fn html_escape_1() {
        let str = "\"<a href='test'>Test&Test</a>\"";
        let result = html_escape(str);
        let expected = "&quot;&lt;a href=&apos;test&apos;&gt;Test&amp;Test&lt;/a&gt;&quot;";
        assert_eq!(result, expected);
    }

    #[test]
    fn hmac_sha1_rfc2202_1() {
        let key = &hex!("0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b");
        let data = b"Hi There";
        let result = hmac_sha1(key, data).into_bytes();
        let expected = &hex!("b617318655057264e28bc0b6fb378c8ef146be00");
        assert_eq!(result[..], expected[..]);
    }

    #[test]
    fn hmac_sha1_rfc2202_2() {
        let key = b"Jefe";
        let data = b"what do ya want for nothing?";
        let result = hmac_sha1(key, data).into_bytes();
        let expected = &hex!("effcdf6ae5eb2fa2d27416d5f184df9c259a7c79");
        assert_eq!(result[..], expected[..]);
    }

    #[test]
    fn hmac_sha1_rfc2202_3() {
        let key = &hex!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
        // 0xdd repeated 50 times
        let data = &[0xdd_u8; 50];
        let result = hmac_sha1(key, data).into_bytes();
        let expected = &hex!("125d7342b9ac11cd91a39af48aa17b4f63f175d3");
        assert_eq!(result[..], expected[..]);
    }

    #[test]
    fn hmac_sha1_rfc2202_4() {
        let key = &hex!("0102030405060708090a0b0c0d0e0f10111213141516171819");
        // 0xcd repeated 50 times
        let data = &[0xcd_u8; 50];
        let result = hmac_sha1(key, data).into_bytes();
        let expected = &hex!("4c9007f4026250c6bc8414f9bf50c86c2d7235da");
        assert_eq!(result[..], expected[..]);
    }

    #[test]
    fn hmac_sha1_rfc2202_5() {
        let key = &hex!("0c0c0c0c0c0c0c0c0c0c0c0c0c0c0c0c0c0c0c0c");
        let data = b"Test With Truncation";
        let result = hmac_sha1(key, data).into_bytes();
        let expected = &hex!("4c1a03424b55e07fe7f27be1d58bb9324a9a5a04");
        assert_eq!(result[..], expected[..]);
    }

    #[test]
    fn hmac_sha1_rfc2202_6() {
        // 0xaa repeated 80 times
        let key = &[0xaa_u8; 80];
        let data = b"Test Using Larger Than Block-Size Key - Hash Key First";
        let result = hmac_sha1(key, data).into_bytes();
        let expected = &hex!("aa4ae5e15272d00e95705637ce8a3b55ed402112");
        assert_eq!(result[..], expected[..]);
    }

    #[test]
    fn hmac_sha1_rfc2202_7() {
        // 0xaa repeated 80 times
        let key = &[0xaa_u8; 80];
        let data = b"Test Using Larger Than Block-Size Key and Larger Than One Block-Size Data";
        let result = hmac_sha1(key, data).into_bytes();
        let expected = &hex!("e8e99d0f45237d786d6bbaa7965c7808bbff1a91");
        assert_eq!(result[..], expected[..]);
    }
}
