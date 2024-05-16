//! 計算関連。

use crate::sysmod::openai::function::{
    get_arg_str, BasicContext, FuncArgs, FuncBodyAsync, Function, FunctionTable, ParameterElement,
    Parameters,
};
use crate::utils::parser;
use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;

/// このモジュールの関数をすべて登録する。
pub fn register_all<T: 'static>(func_table: &mut FunctionTable<T>) {
    register_calculate(func_table);
}

/// 数式を計算する。
async fn calculate(args: &FuncArgs) -> Result<String> {
    let formula = get_arg_str(args, "formula")?;

    let toks = parser::lexical_analyze(formula)?;
    let ast = parser::parse_formula(toks)?;
    let result = parser::evaluate(ast)?;

    Ok(result.to_string())
}

fn calculate_pin<T>(_bctx: Arc<BasicContext>, _ctx: T, args: &FuncArgs) -> FuncBodyAsync {
    Box::pin(calculate(args))
}

fn register_calculate<T: 'static>(func_table: &mut FunctionTable<T>) {
    let mut properties = HashMap::new();
    properties.insert(
        "formula".to_string(),
        ParameterElement {
            type_: "string".to_string(),
            description: Some("Available only ( ) + - * / %".to_string()),
            ..Default::default()
        },
    );

    func_table.register_function(
        Function {
            name: "calculate".to_string(),
            description: Some("Calculate formula.".to_string()),
            parameters: Parameters {
                type_: "object".to_string(),
                properties,
                required: Default::default(),
            },
        },
        Box::new(calculate_pin),
    );
}
