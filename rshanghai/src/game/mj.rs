/*
Basics:
9 + 9 + 9 + 7 = 34
34 * 4 = 136
*/

use anyhow::{bail, ensure, Result};
use std::collections::HashMap;

const HAI_ID_COUNT: usize = 34;

#[derive(Debug, Clone, Copy)]
pub struct Menzi {
    id: u8,
    furo: bool,
    order: bool,
    kan: bool,
}

#[derive(Debug, Clone)]
pub struct Hand {
    size: u32,
    bucket: [u8; HAI_ID_COUNT],
    draw: u8,
    head: Option<u8>,
    menzi: Vec<Menzi>,
}

fn id_to_number(id: u8) -> Option<u8> {
    if id < 27 {
        Some(id % 9 + 1)
    } else {
        None
    }
}

pub fn parse(from: &str) -> Result<Hand> {
    const EMOJI: &str = "🀇🀈🀉🀊🀋🀌🀍🀎🀏🀙🀚🀛🀜🀝🀞🀟🀠🀡🀐🀑🀒🀓🀔🀕🀖🀗🀘🀀🀁🀂🀃🀆🀅🀄";
    let mut table = HashMap::new();
    for (i, c) in EMOJI.chars().enumerate() {
        table.insert(c, i);
    }
    assert!(table.len() == HAI_ID_COUNT);

    let mut hand = Hand {
        size: 0,
        bucket: [0; HAI_ID_COUNT],
        draw: 0xff,
        head: None,
        menzi: Vec::new(),
    };
    for (i, c) in from.chars().enumerate() {
        if let Some(&id) = table.get(&c) {
            ensure!(hand.bucket[id] < 0xff, "Overflow at {}", i + 1);
            hand.bucket[id] += 1;
            hand.size += 1;
            hand.draw = id as u8;
            continue;
        }
        bail!("Error at {}", i + 1);
    }

    Ok(hand)
}

pub fn all_agari(hand: &Hand) -> Vec<Hand> {
    let mut all = Vec::new();
    all.extend(all_normal_agari(&mut hand.clone(), 0));

    all
}

fn all_normal_agari(hand: &mut Hand, idstart: usize) -> Vec<Hand> {
    let mut all = Vec::new();

    if hand.size == 0 {
        all.push(hand.clone());
    } else if hand.head.is_none() {
        for id in idstart..HAI_ID_COUNT {
            if hand.bucket[id] >= 2 {
                hand.head = Some(id as u8);
                hand.bucket[id] -= 2;
                hand.size -= 2;
                all.extend(all_normal_agari(hand, idstart));
                hand.size += 2;
                hand.bucket[id] += 2;
                hand.head = None;
            }
        }
    } else {
        for id in idstart..HAI_ID_COUNT {
            if hand.bucket[id] >= 3 {
                hand.menzi.push(Menzi {
                    id: id as u8,
                    furo: false,
                    order: false,
                    kan: false,
                });
                hand.bucket[id] -= 3;
                hand.size -= 3;
                all.extend(all_normal_agari(hand, id));
                hand.size += 3;
                hand.bucket[id] += 3;
                hand.menzi.pop();
            }
            if hand.bucket[id] >= 1 {
                let num = id_to_number(id as u8);
                if num.is_some_and(|num| num <= 7)
                    && hand.bucket[id + 1] >= 1
                    && hand.bucket[id + 2] >= 1
                {
                    hand.menzi.push(Menzi {
                        id: id as u8,
                        furo: false,
                        order: true,
                        kan: false,
                    });
                    hand.bucket[id] -= 1;
                    hand.bucket[id + 1] -= 1;
                    hand.bucket[id + 2] -= 1;
                    hand.size -= 3;
                    all.extend(all_normal_agari(hand, id));
                    hand.size += 3;
                    hand.bucket[id + 2] += 1;
                    hand.bucket[id + 1] += 1;
                    hand.bucket[id] += 1;
                    hand.menzi.pop();
                }
            }
        }
    }
    all
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse1() {
        let teststr = "🀇🀈🀉🀊🀋🀌🀍🀎🀏🀙🀚🀛🀜🀝🀞🀟🀠🀡🀐🀑🀒🀓🀔🀕🀖🀗🀘🀀🀁🀂🀃🀆🀅🀄";
        let hand = parse(teststr).unwrap();
        assert!(hand.size == 34);
        for &count in hand.bucket.iter() {
            assert!(count == 1);
        }
    }

    #[test]
    fn parse_error() {
        let teststr = "🀇🀈🀉🀊🀋🀌🀍🀎🀏🀙🀚a🀛🀜🀝🀞🀟🀠🀡🀐🀑🀒🀓🀔🀕🀖🀗🀘🀀🀁🀂🀃🀆🀅🀄";
        let hand = parse(teststr);
        assert!(hand.is_err());
    }

    #[test]
    fn anko4() {
        let hand = parse("🀙🀙🀙🀡🀡🀡🀇🀇🀇🀏🀏🀏🀘🀘").unwrap();
        let agari = all_agari(&hand);
        assert!(agari.len() == 1);
    }

    #[test]
    fn order4() {
        let hand = parse("🀇🀈🀉🀍🀎🀏🀙🀚🀛🀟🀠🀡🀃🀃").unwrap();
        let agari = all_agari(&hand);
        assert!(agari.len() == 1);
    }
}
