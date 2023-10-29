//! OpenAI API の会話コンテキストのトークン数制限付き管理。

use crate::sysmod::openai::ChatMessage;

use std::{collections::VecDeque, sync::OnceLock};
use tiktoken_rs::{cl100k_base, CoreBPE};

static CORE: OnceLock<CoreBPE> = OnceLock::new();

pub struct ChatHistory {
    token_limit: u32,
    token_count: u32,
    history: VecDeque<Element>,
}

struct Element {
    msg: ChatMessage,
    token_count: u32,
}

impl ChatHistory {
    pub fn new(token_limit: u32) -> Self {
        Self {
            token_limit,
            token_count: 0,
            history: Default::default(),
        }
    }

    /// ヒストリの最後にエントリを追加する。
    ///
    /// 合計サイズを超えた場合、先頭から削除する。
    /// 1エントリでサイズを超えてしまっている場合、超えないように内容をトリムする。
    pub fn push(&mut self, mut msg: ChatMessage) {
        let count = if let Some(text) = &msg.content {
            let tokens = tokenize(text);
            let count = tokens.len() as u32;
            if count > self.token_limit {
                let trimmed = decode(&tokens[0..self.token_limit as usize]);
                msg.content = Some(trimmed);

                self.token_limit
            } else {
                count
            }
        } else {
            0
        };

        self.history.push_back(Element {
            msg,
            token_count: count,
        });
        self.token_count += count;

        while self.token_count > self.token_limit {
            let front = self.history.pop_front().unwrap();
            self.token_count -= front.token_count;
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &ChatMessage> {
        self.history.iter().map(|elem| &elem.msg)
    }
}

fn init_core() -> CoreBPE {
    cl100k_base().unwrap()
}

fn tokenize(text: &str) -> Vec<usize> {
    let bpe = CORE.get_or_init(init_core);

    bpe.encode_with_special_tokens(text)
}

pub fn token_count(text: &str) -> u32 {
    tokenize(text).len() as u32
}

fn decode(tokens: &[usize]) -> String {
    let bpe = CORE.get_or_init(init_core);

    bpe.decode(tokens.to_vec()).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token() {
        let bpe = cl100k_base().unwrap();
        let tokens = bpe.encode_with_special_tokens("This is a sentence   with spaces");

        // https://platform.openai.com/tokenizer
        assert_eq!(7, tokens.len());
    }
}
