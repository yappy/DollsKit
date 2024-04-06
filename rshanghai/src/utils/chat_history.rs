//! OpenAI API の会話コンテキストのトークン数制限付き管理。

use crate::sysmod::openai::ChatMessage;

use std::{collections::VecDeque, sync::OnceLock};
use tiktoken_rs::{cl100k_base, CoreBPE};

static CORE: OnceLock<CoreBPE> = OnceLock::new();

/// 会話履歴管理。
pub struct ChatHistory {
    /// トークン数合計上限。
    token_limit: usize,
    /// 現在のトークン数合計。
    token_count: usize,
    /// 履歴データのキュー。
    history: VecDeque<Element>,
}

/// 履歴データ。
struct Element {
    /// メッセージ。
    msg: ChatMessage,
    /// [Self::msg] のトークン数。
    token_count: usize,
}

impl ChatHistory {
    /// コンストラクタ。
    ///
    /// * `token_limit` - トークン数上限。
    pub fn new(token_limit: usize) -> Self {
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
            let count = tokens.len();
            if count > self.token_limit {
                let trimmed = decode(&tokens[0..self.token_limit]);
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

    /// 全履歴をクリアする。
    pub fn clear(&mut self) {
        self.history.clear();
        self.token_count = 0;
    }

    /// 全履歴を走査するイテレータを返す。
    pub fn iter(&self) -> impl Iterator<Item = &ChatMessage> {
        self.history.iter().map(|elem| &elem.msg)
    }

    /// 履歴の数を返す。
    pub fn len(&self) -> usize {
        self.history.len()
    }

    /// 現在のトークン数使用量を (usage / total) のタプルで返す。
    pub fn usage(&self) -> (usize, usize) {
        (self.token_count, self.token_limit)
    }
}

/// トークン化ライブラリを初期化する。
fn init_core() -> CoreBPE {
    cl100k_base().unwrap()
}

/// 文章をトークン化する。
fn tokenize(text: &str) -> Vec<usize> {
    let bpe = CORE.get_or_init(init_core);

    bpe.encode_with_special_tokens(text)
}

/// 文章のトークン数を数える。
pub fn token_count(text: &str) -> usize {
    tokenize(text).len()
}

/// トークン列から文字列に復元する。
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
