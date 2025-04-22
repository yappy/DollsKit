//! ゲーム。

use crate::sysmod::openai::ParameterType;
use crate::sysmod::openai::function::{
    FuncArgs, Function, FunctionTable, ParameterElement, Parameters, get_arg_i64, get_arg_str,
};
use utils::game::mine_sweeper::{self, MineSweeper};

use anyhow::{Result, bail};
use std::collections::HashMap;

/// このモジュールの関数をすべて登録する。
pub fn register_all<T: 'static>(func_table: &mut FunctionTable<T>) {
    register_mine_sweeper(func_table);
}

/// マインスイーパ。
async fn mine_sweeper(args: &FuncArgs) -> Result<String> {
    static GAME: std::sync::Mutex<Option<MineSweeper>> = std::sync::Mutex::new(None);

    let action = get_arg_str(args, "action")?;

    let mut game = GAME.lock().unwrap();
    if game.is_none() {
        *game = Some(MineSweeper::new(mine_sweeper::Level::Easy.to_config()).unwrap());
    }

    let result = match action {
        "start" => {
            *game = Some(MineSweeper::new(mine_sweeper::Level::Easy.to_config()).unwrap());

            game.as_ref().unwrap().to_json_pretty()
        }
        "status" => {
            let game = game.as_ref().unwrap();

            game.to_json_pretty()
        }
        "open" => {
            let game = game.as_mut().unwrap();

            let x = get_arg_i64(args, "x", 1..=game.width as i64)? as i32 - 1;
            let y = get_arg_i64(args, "y", 1..=game.height as i64)? as i32 - 1;

            game.reveal(x, y)?;

            game.to_json_pretty()
        }
        _ => {
            bail!("Invalid action: {}", action);
        }
    };

    Ok(result)
}

fn register_mine_sweeper<T: 'static>(func_table: &mut FunctionTable<T>) {
    let mut properties = HashMap::new();
    properties.insert(
        "action".to_string(),
        ParameterElement {
            type_: vec![ParameterType::String],
            description: None,
            enum_: Some(vec![
                "start".to_string(),
                "status".to_string(),
                "open".to_string(),
            ]),
            ..Default::default()
        },
    );
    properties.insert(
        "x".to_string(),
        ParameterElement {
            type_: vec![ParameterType::Integer, ParameterType::Null],
            description: Some("x to be opened (1 <= x <= width)".to_string()),
            ..Default::default()
        },
    );
    properties.insert(
        "y".to_string(),
        ParameterElement {
            type_: vec![ParameterType::Integer, ParameterType::Null],
            description: Some("y to be opened (1 <= y <= height)".to_string()),
            ..Default::default()
        },
    );

    func_table.register_function(
        Function {
            name: "mine_sweeper".to_string(),
            description: Some("Play Mine Sweeper".to_string()),
            parameters: Parameters {
                properties,
                required: vec!["action".to_string(), "x".to_string(), "y".to_string()],
                ..Default::default()
            },
            ..Default::default()
        },
        |_, _, args| Box::pin(mine_sweeper(args)),
    );
}
