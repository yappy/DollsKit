//! OpenAI API - function 機能向け、
//! すべての場合に共通で提供される基本的な関数群。

mod math;
mod playtools;
mod system;
mod web;

use super::function::FunctionTable;

/// このモジュール以下の全ての関数を [FunctionTable] に登録する。
pub fn register_all<T: 'static>(func_table: &mut FunctionTable<T>) {
    system::register_all(func_table);
    math::register_all(func_table);
    playtools::register_all(func_table);
    web::register_all(func_table);
}
