//! OpenAI API の会話コンテキストのトークン数制限付き管理。

use crate::sysmod::openai::{InputItem, Role};

use anyhow::{Result, ensure};
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
    /// メッセージのリスト。
    /// 削除は [Element] 単位で行われる。
    items: Vec<InputItem>,
    /// [Self::msg] の総トークン数。
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

    pub fn push_message(&mut self, role: Role, content: &str) -> Result<()> {
        let tokens = self.tokenize(content);
        let token_count = tokens.len();

        let item = InputItem::Message {
            role,
            content: content.to_string(),
        };

        self.push(vec![item], token_count)
    }

    pub fn push_function(
        &mut self,
        call_id: &str,
        name: &str,
        arguments: &str,
        output: &str,
    ) -> Result<()> {
        let item1 = InputItem::FunctionCall {
            call_id: call_id.to_string(),
            name: name.to_string(),
            arguments: arguments.to_string(),
        };
        let item2 = InputItem::FunctionCallOutput {
            call_id: call_id.to_string(),
            output: output.to_string(),
        };
        // call_id も含めるべきかは不明。
        let token_count = self.tokenize(name).len()
            + self.tokenize(arguments).len()
            + self.tokenize(output).len();

        self.push(vec![item1, item2], token_count)
    }

    /// ヒストリの最後にエントリを追加する。
    ///
    /// 合計サイズを超えた場合、超えなくなるように先頭から削除する。
    /// このエントリだけでサイズを超えてしまっている場合、エラー。
    fn push(&mut self, items: Vec<InputItem>, token_count: usize) -> Result<()> {
        ensure!(token_count <= self.token_limit, "Too long message");

        self.history.push_back(Element { items, token_count });
        self.token_count += token_count;

        while self.token_count > self.token_limit {
            let front = self.history.pop_front().unwrap();
            self.token_count -= front.token_count;
        }

        Ok(())
    }

    /// 全履歴をクリアする。
    pub fn clear(&mut self) {
        self.history.clear();
        self.token_count = 0;
    }

    /// 全履歴を走査するイテレータを返す。
    pub fn iter(&self) -> impl Iterator<Item = &InputItem> {
        self.history.iter().flat_map(|e| e.items.iter())
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token() {
        let hist = ChatHistory::new("gpt-4o");
        let count = hist.token_count("こんにちは、管理人形さん。");

        // https://platform.openai.com/tokenizer
        assert_eq!(7, count);
    }
}
