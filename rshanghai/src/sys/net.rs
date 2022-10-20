use percent_encoding::{utf8_percent_encode, AsciiSet};
use sha1::Sha1;
use hmac::{SimpleHmac, Mac, digest::CtOutput};

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

pub type HmacSha1 = SimpleHmac<Sha1>;

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


#[cfg(test)]
mod tests {
    use super::*;
    use hex_literal::hex;

    #[test]
    fn percent_encode_twitter_sample() {
        let str = "Hello Ladies + Gentlemen, a signed OAuth request!";
        let result = percent_encode(&str);
        let expected = "Hello%20Ladies%20%2b%20Gentlemen%2c%20a%20signed%20OAuth%20request%21";
        assert_eq!(result.to_ascii_lowercase(), expected.to_ascii_lowercase());
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
