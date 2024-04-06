//! 乱数生成によるダイスロール。
//!
//! コインフリップにも応用可能。

use anyhow::{ensure, Result};
use rand::Rng;
use static_assertions::const_assert;

/// ダイスの面数の最大値。
pub const FACE_MAX: u64 = 1u64 << 56;
/// ダイスの個数の最大値。
pub const COUNT_MAX: u32 = 100;

const_assert!(FACE_MAX < u64::MAX / COUNT_MAX as u64);

/// ダイスを振る。
///
/// * `face` - 何面のダイスを振るか。
/// * `count` - 何個のダイスを振るか。
pub fn roll(face: u64, count: u32) -> Result<Vec<u64>> {
    ensure!(
        (1..=FACE_MAX).contains(&face),
        "face must be 1 <= face <= {FACE_MAX}",
    );
    ensure!(
        (1..=COUNT_MAX).contains(&count),
        "count must be 1 <= count <= {COUNT_MAX}",
    );

    let mut result = vec![];
    let mut rng = rand::thread_rng();
    for _ in 0..count {
        result.push(rng.gen_range(1..=face));
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dice_6_many_times() {
        let mut result = roll(6, COUNT_MAX).unwrap();
        assert_eq!(result.len(), COUNT_MAX as usize);

        // 100 回も振れば 1..=6 が 1 回ずつは出る
        result.sort();
        for x in 1..=6 {
            assert!(result.binary_search(&x).is_ok());
        }
    }

    #[test]
    #[should_panic]
    fn dice_invalid_dice() {
        let _ = roll(FACE_MAX + 1, 1).unwrap();
    }

    #[test]
    #[should_panic]
    fn dice_invalid_count() {
        let _ = roll(6, COUNT_MAX + 1).unwrap();
    }
}
