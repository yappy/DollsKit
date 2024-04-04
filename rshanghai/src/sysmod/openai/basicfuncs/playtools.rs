//! システム情報取得。

use crate::sysmod::openai::function::{
    get_ranged_arg, FuncArgs, FuncBodyAsync, Function, FunctionTable, ParameterElement, Parameters,
};
use crate::utils::playtools::dice;
use anyhow::Result;
use std::collections::HashMap;

/// このモジュールの関数をすべて登録する。
pub fn register_all<T: 'static>(func_table: &mut FunctionTable<T>) {
    register_role_dice(func_table);
}

const FACE_MIN: i32 = 1;
const FACE_MAX: i32 = 100;
const COUNT_MIN: i32 = 1;
const COUNT_MAX: i32 = 100;

/// サイコロを振る。
async fn role_dice(args: &FuncArgs) -> Result<String> {
    let face: i32 = get_ranged_arg(args, "face", FACE_MIN..=FACE_MAX)?;
    let count: i32 = get_ranged_arg(args, "count", COUNT_MIN..=COUNT_MAX)?;
    let result = dice::roll(face as u64, count as u32)?;

    Ok(format!("{:?}", result))
}

fn role_dice_pin<T>(_ctx: T, args: &FuncArgs) -> FuncBodyAsync {
    Box::pin(role_dice(args))
}

fn register_role_dice<T: 'static>(func_table: &mut FunctionTable<T>) {
    let mut properties = HashMap::new();
    properties.insert(
        "face".to_string(),
        ParameterElement {
            type_: "integer".to_string(),
            description: Some("Face count of the dices (default is 6)".to_string()),
            minumum: Some(FACE_MIN),
            maximum: Some(FACE_MAX),
            ..Default::default()
        },
    );
    properties.insert(
        "count".to_string(),
        ParameterElement {
            type_: "integer".to_string(),
            description: Some("How many dices do you roll? (default is 1)".to_string()),
            minumum: Some(COUNT_MIN),
            maximum: Some(COUNT_MAX),
            ..Default::default()
        },
    );

    func_table.register_function(
        Function {
            name: "role_dice".to_string(),
            description: Some("Get the current date and time".to_string()),
            parameters: Parameters {
                type_: "object".to_string(),
                properties,
                required: vec!["tz".to_string()],
            },
        },
        Box::new(role_dice_pin),
    );
}
