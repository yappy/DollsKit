//! Mine Sweeper

use anyhow::{Context, Result, bail, ensure};
use rand::seq::SliceRandom;
use serde::Serialize;
use std::ops::RangeInclusive;

pub const W_MAX: i32 = 30;
pub const H_MAX: i32 = 16;
const W_RANGE: RangeInclusive<i32> = 1..=W_MAX;
const H_RANGE: RangeInclusive<i32> = 1..=W_MAX;

const DXY: &[(i32, i32)] = &[
    (-1, -1),
    (-1, 0),
    (-1, 1),
    (0, -1),
    (0, 1),
    (1, -1),
    (1, 0),
    (1, 1),
];

#[derive(Debug, Clone, Copy, Default, Serialize)]
pub enum GameState {
    #[default]
    Initialized,
    Playing,
    Cleared,
    Failed,
}

#[derive(Debug, Clone, Copy)]
pub enum Cell {
    Mine,
    Number(u8),
}

#[derive(Debug, Clone, Copy)]
pub struct Config {
    pub width: i32,
    pub height: i32,
    pub mine_count: i32,
}

pub enum Level {
    Easy,
    Normal,
    Hard,
}

#[derive(Debug, Clone, Serialize)]
struct MsStateJson {
    state: GameState,
    width: i32,
    height: i32,
    mine_count: i32,
    board: Vec<String>,
}

impl Level {
    pub fn to_config(&self) -> Config {
        match self {
            Self::Easy => Config {
                width: 9,
                height: 9,
                mine_count: 10,
            },
            Self::Normal => Config {
                width: 16,
                height: 16,
                mine_count: 40,
            },
            Self::Hard => Config {
                width: 30,
                height: 16,
                mine_count: 99,
            },
        }
    }
}

pub struct MineSweeper {
    pub state: GameState,
    pub width: i32,
    pub height: i32,
    pub mine_count: i32,
    board: Vec<Cell>,
    revealed: Vec<bool>,
}

impl MineSweeper {
    pub fn new(config: Config) -> Result<Self> {
        let width = config.width;
        let height = config.height;
        ensure!(W_RANGE.contains(&width));
        ensure!(H_RANGE.contains(&height));
        // NG if =0 or =size
        let size = width * height;
        let mine_count = config.mine_count;
        ensure!((1..size).contains(&mine_count));

        // Put mines at random
        let mut rng = rand::rng();
        let mut board: Vec<_> = std::iter::repeat_n(Cell::Mine, mine_count as usize)
            .chain(std::iter::repeat_n(
                Cell::Number(0),
                (size - mine_count) as usize,
            ))
            .collect();
        board.shuffle(&mut rng);

        // Count mines around each cell
        for y in 0..height {
            for x in 0..width {
                let idx = Self::convert_raw(x, y, width, height).unwrap();
                if matches!(board[idx], Cell::Mine) {
                    continue;
                }
                let mut count = 0;
                for (dx, dy) in DXY {
                    let nx = x + dx;
                    let ny = y + dy;
                    if let Some(nidx) = Self::convert_raw(nx, ny, width, height) {
                        if matches!(board[nidx], Cell::Mine) {
                            count += 1;
                        }
                    }
                }
                board[idx] = Cell::Number(count);
            }
        }

        let obj = Self {
            state: GameState::Initialized,
            width,
            height,
            mine_count,
            board,
            revealed: vec![false; (config.width * config.height) as usize],
        };

        Ok(obj)
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(&self.to_json_raw()).unwrap()
    }

    pub fn to_json_pretty(&self) -> String {
        serde_json::to_string_pretty(&self.to_json_raw()).unwrap()
    }

    fn to_json_raw(&self) -> MsStateJson {
        let mut board = Vec::with_capacity(self.height as usize);
        for y in 0..self.height {
            let mut line = String::with_capacity(self.width as usize);
            for x in 0..self.width {
                let idx = self.convert(x, y).unwrap();
                let c = if self.revealed[idx] {
                    match self.board[idx] {
                        Cell::Mine => '*',
                        Cell::Number(n) => (b'0' + n) as char,
                    }
                } else {
                    '.'
                };
                line.push(c);
            }
            board.push(line);
        }

        MsStateJson {
            state: self.state,
            width: self.width,
            height: self.height,
            mine_count: self.mine_count,
            board,
        }
    }

    pub fn reveal(&mut self, x: i32, y: i32) -> Result<GameState> {
        if !(matches!(self.state, GameState::Initialized | GameState::Playing)) {
            bail!("Already finished");
        }

        let _ = self.convert(x, y).context("Invalid x y")?;

        self.reveal_raw(x, y).context("Already revealed")?;
        self.state = self.check_state();

        Ok(self.state)
    }

    fn reveal_raw(&mut self, x: i32, y: i32) -> Option<Cell> {
        let idx = self.convert(x, y)?;
        if !self.revealed[idx] {
            self.revealed[idx] = true;
            if let Cell::Number(n) = self.board[idx] {
                if n == 0 {
                    for (dy, dx) in DXY {
                        let nx = x + dx;
                        let ny = y + dy;
                        self.reveal_raw(nx, ny);
                    }
                }
            }
            Some(self.board[idx])
        } else {
            None
        }
    }

    fn check_state(&self) -> GameState {
        debug_assert_eq!(self.board.len(), self.revealed.len());

        let mut not_cleared = false;
        for (cell, revealed) in self.board.iter().zip(self.revealed.iter()) {
            if *revealed {
                if matches!(cell, Cell::Mine) {
                    return GameState::Failed;
                }
            } else if !matches!(cell, Cell::Mine) {
                not_cleared = true;
            }
        }

        if not_cleared {
            GameState::Playing
        } else {
            GameState::Cleared
        }
    }

    fn convert(&self, x: i32, y: i32) -> Option<usize> {
        Self::convert_raw(x, y, self.width, self.height)
    }

    fn convert_raw(x: i32, y: i32, w: i32, h: i32) -> Option<usize> {
        if (0..w).contains(&x) && (0..h).contains(&y) {
            Some((y * w + x) as usize)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    #[ignore]
    // cargo test ms_play -- --ignored --nocapture
    fn ms_play() -> Result<()> {
        let mut ms = MineSweeper::new(Level::Easy.to_config()).unwrap();
        loop {
            println!("{}", ms.to_json_pretty());

            let mut input = String::new();
            println!("Enter x y (or 'exit'):");
            std::io::stdin().read_line(&mut input).unwrap();
            let input = input.trim();
            if input == "exit" {
                break Ok(());
            }

            let mut iter = input.split_whitespace();
            let x: i32 = iter.next().unwrap().parse().unwrap();
            let y: i32 = iter.next().unwrap().parse().unwrap();
            match ms.reveal(x, y) {
                Ok(state) => match state {
                    GameState::Playing => println!("Playing"),
                    GameState::Cleared => {
                        println!("{}", ms.to_json_pretty());
                        println!("Cleared");
                        break Ok(());
                    }
                    GameState::Failed => {
                        println!("{}", ms.to_json_pretty());
                        println!("Failed");
                        break Ok(());
                    }
                    _ => {}
                },
                Err(e) => println!("Error: {}", e),
            }
        }
    }

    #[test]
    fn ms_init() {
        for _ in 0..100 {
            let _ms = MineSweeper::new(Level::Easy.to_config()).unwrap();
            let _ms = MineSweeper::new(Level::Normal.to_config()).unwrap();
            let _ms = MineSweeper::new(Level::Hard.to_config()).unwrap();
        }
    }
}
