//! システム情報取得。

use crate::sysmod::openai::function::{
    get_arg_i64, BasicContext, FuncArgs, FuncBodyAsync, Function, FunctionTable, ParameterElement,
    Parameters,
};
use crate::utils::playtools::dice;
use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;

/// このモジュールの関数をすべて登録する。
pub fn register_all<T: 'static>(func_table: &mut FunctionTable<T>) {
    register_flip_coin(func_table);
    register_role_dice(func_table);
}

/// ダイスまたはコイン数の最小値。
const COUNT_MIN: i64 = 1;
/// ダイスまたはコイン数の最大値。
const COUNT_MAX: i64 = 100;
/// ダイスの面数の最小値。
const FACE_MIN: i64 = 1;
/// ダイスの面数の最大値。
const FACE_MAX: i64 = 100;

/// サイコロを振る。
async fn flip_coin(args: &FuncArgs) -> Result<String> {
    let count: i64 = get_arg_i64(args, "count", COUNT_MIN..=COUNT_MAX)?;
    let result = dice::roll(2_u64, count as u32)?;

    let mut text = String::from("[");
    let mut first = true;
    for &n in result.iter() {
        if first {
            first = false;
        } else {
            text.push(',');
        }
        text.push_str(if n == 1 { "\"H\"" } else { "\"T\"" });
    }
    text.push(']');

    Ok(text)
}

fn flip_coin_pin<T>(_bctx: Arc<BasicContext>, _ctx: T, args: &FuncArgs) -> FuncBodyAsync {
    Box::pin(flip_coin(args))
}

fn register_flip_coin<T: 'static>(func_table: &mut FunctionTable<T>) {
    let mut properties = HashMap::new();
    properties.insert(
        "count".to_string(),
        ParameterElement {
            type_: "integer".to_string(),
            description: Some("How many coins do you flip? (default is 1)".to_string()),
            minumum: Some(COUNT_MIN),
            maximum: Some(COUNT_MAX),
            ..Default::default()
        },
    );

    func_table.register_function(
        Function {
            name: "flip_coin".to_string(),
            description: Some("Flip coin(s). H means Head. T means Tail.".to_string()),
            parameters: Parameters {
                type_: "object".to_string(),
                properties,
                required: Default::default(),
            },
        },
        Box::new(flip_coin_pin),
    );
}

/// サイコロを振る。
async fn role_dice(args: &FuncArgs) -> Result<String> {
    let face: i64 = get_arg_i64(args, "face", FACE_MIN..=FACE_MAX)?;
    let count: i64 = get_arg_i64(args, "count", COUNT_MIN..=COUNT_MAX)?;
    let result = dice::roll(face as u64, count as u32)?;

    Ok(format!("{:?}", result))
}

fn role_dice_pin<T>(_bctx: Arc<BasicContext>, _ctx: T, args: &FuncArgs) -> FuncBodyAsync {
    Box::pin(role_dice(args))
}

fn register_role_dice<T: 'static>(func_table: &mut FunctionTable<T>) {
    let mut properties = HashMap::new();
    properties.insert(
        "face".to_string(),
        ParameterElement {
            type_: "integer".to_string(),
            description: Some("Face count of the dice (default is 6)".to_string()),
            minumum: Some(FACE_MIN),
            maximum: Some(FACE_MAX),
            ..Default::default()
        },
    );
    properties.insert(
        "count".to_string(),
        ParameterElement {
            type_: "integer".to_string(),
            description: Some("How many dice do you roll? (default is 1)".to_string()),
            minumum: Some(COUNT_MIN),
            maximum: Some(COUNT_MAX),
            ..Default::default()
        },
    );

    func_table.register_function(
        Function {
            name: "role_dice".to_string(),
            description: Some(
                "Role dice with specified number of faces specified number of times".to_string(),
            ),
            parameters: Parameters {
                type_: "object".to_string(),
                properties,
                required: Default::default(),
            },
        },
        Box::new(role_dice_pin),
    );
}
