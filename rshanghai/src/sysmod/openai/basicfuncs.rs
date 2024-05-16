//! OpenAI API - function 機能向け、
//! すべての場合に共通で提供される基本的な関数群。

mod math;
mod playtools;
mod system;
mod web;

use super::function::{BasicContext, FunctionTable};
use std::sync::Arc;

/// このモジュール以下の全ての関数を [FunctionTable] に登録する。
pub fn register_all<T: 'static>(func_table: &mut FunctionTable<T>, ctx: Arc<BasicContext>) {
    system::register_all(func_table, ctx);
    math::register_all(func_table);
    playtools::register_all(func_table);
    web::register_all(func_table);
}
