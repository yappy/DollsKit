//! すべての場合に共通で提供される基本的な関数群。

mod system;
mod web;

use super::function::FunctionTable;

pub fn register_all<T: 'static>(func_table: &mut FunctionTable<T>) {
    system::register_all(func_table);
    web::register_all(func_table);
}
