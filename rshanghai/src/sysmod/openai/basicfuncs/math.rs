//! 計算関連。

use crate::sysmod::openai::function::{
    FuncArgs, Function, FunctionTable, ParameterElement, Parameters, get_arg_str,
};
use crate::utils::parser;
use anyhow::Result;
use std::collections::HashMap;

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
                required: vec!["formula".to_string()],
            },
        },
        |_, _, args| Box::pin(calculate(args)),
    );
}
