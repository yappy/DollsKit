//! OpenAI API の会話コンテキストのトークン数制限付き管理。

use crate::sysmod::openai::InputElement;

use std::collections::VecDeque;
use tiktoken_rs::CoreBPE;

/// 会話履歴管理。
pub struct ChatHistory {
    /// トークナイザ。
    core: CoreBPE,

    /// トークン数。
    total_token_limit: usize,
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
    msg: InputElement,
    /// [Self::msg] のトークン数。
    token_count: usize,
}

impl ChatHistory {
    /// コンストラクタ。
    ///
    /// * `model` - OpenAI API モデル名。
    pub fn new(model: &str) -> Self {
        let core = tiktoken_rs::get_bpe_from_model(model).unwrap();
        let total_token_limit = tiktoken_rs::model::get_context_size(model);

        Self {
            core,
            total_token_limit,
            token_limit: total_token_limit,
            token_count: 0,
            history: Default::default(),
        }
    }

    /// トークン数合計上限を減らす。
    pub fn reserve_tokens(&mut self, token_count: usize) {
        if self.token_limit < token_count {
            panic!("Invalid reserve size");
        }
        self.token_limit -= token_count;
    }

    /// ヒストリの最後にエントリを追加する。
    ///
    /// 合計サイズを超えた場合、先頭から削除する。
    /// 1エントリでサイズを超えてしまっている場合、超えないように内容をトリムする。
    pub fn push(&mut self, mut msg: InputElement) {
        let trim_and_size = |text: &str| {
            let tokens = self.tokenize(&text);
            let count = tokens.len();
            if count > self.token_limit {
                let trimmed = self.decode(&tokens[0..self.token_limit]);

                (trimmed, self.token_limit)
            } else {
                (text.to_string(), count)
            }
        };

        let count = match &mut msg {
            InputElement::Message { role: _, content } => {
                let (trimmed, count) = trim_and_size(content);
                *content = trimmed;

                count
            }
            InputElement::FunctionCallOutput { call_id: _, output } => {
                let (trimmed, count) = trim_and_size(output);
                *output = trimmed;

                count
            }
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
    pub fn iter(&self) -> impl Iterator<Item = &InputElement> {
        self.history.iter().map(|elem| &elem.msg)
    }

    /// 履歴の数を返す。
    pub fn len(&self) -> usize {
        self.history.len()
    }

    /// トークン制限総量を返す。
    pub fn get_total_limit(&self) -> usize {
        self.total_token_limit
    }

    /// 現在のトークン数使用量を (usage / total) のタプルで返す。
    pub fn usage(&self) -> (usize, usize) {
        (self.token_count, self.token_limit)
    }

    /// 文章をトークン化する。
    fn tokenize(&self, text: &str) -> Vec<u32> {
        self.core.encode_with_special_tokens(text)
    }

    /// 文章のトークン数を数える。
    pub fn token_count(&self, text: &str) -> usize {
        self.tokenize(text).len()
    }

    /// トークン列から文字列に復元する。
    fn decode(&self, tokens: &[u32]) -> String {
        self.core.decode(tokens.to_vec()).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token() {
        let hist = ChatHistory::new("gpt-4");
        let count = hist.token_count("This is a sentence   with spaces");

        // https://platform.openai.com/tokenizer
        assert_eq!(7, count);
    }
}
