//! 字句解析・構文解析関連。

use std::{collections::VecDeque, str::Chars};

use anyhow::{anyhow, bail, ensure, Result};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Position {
    line: u32,
    column: u32,
}

impl std::fmt::Display for Position {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}:{}", self.line, self.column)?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Token {
    LParen,
    RParen,
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Integer(i64),
    Eof,
}

pub type TokenWithPos = (Token, Position);

struct Lexer<'a> {
    iter: Chars<'a>,
    buf: Option<char>,
    pos: Position,
    newline: bool,
}

impl Lexer<'_> {
    fn new(src: &str) -> Lexer {
        Lexer {
            iter: src.chars(),
            buf: None,
            pos: Position { line: 1, column: 0 },
            newline: false,
        }
    }

    /// 次の1文字を取得するが消費しない。
    fn peekc(&mut self) -> Option<char> {
        if self.buf.is_none() {
            let c = self.iter.next();
            self.buf = c;
        }

        self.buf
    }

    /// 次の1文字を取得し消費する。
    fn getc(&mut self) -> Option<char> {
        let c = if self.buf.is_some() {
            let c = self.buf;
            self.buf = None;
            c
        } else {
            self.iter.next()
        };

        if self.newline {
            self.pos.column = 0;
            self.pos.line += 1;
            self.newline = false;
        }

        if let Some(c) = c {
            if c == '\n' {
                self.newline = true;
            }
            self.pos.column += 1;
        }

        c
    }

    /// 次のトークンを取得する。None は EOF を示す。
    fn next_token(&mut self) -> Result<Option<TokenWithPos>> {
        // skip (whitespace)*
        loop {
            let c = self.peekc();
            if let Some(c) = c {
                if !c.is_ascii_whitespace() {
                    break;
                }
            } else {
                // EOF
                return Ok(None);
            }
            // consume whitespace
            self.getc();
        }

        let c = self.getc().unwrap();
        let pos = self.pos;

        let matched = match c {
            '(' => Some((Token::LParen, pos)),
            ')' => Some((Token::RParen, pos)),

            '+' => Some((Token::Add, pos)),
            '-' => Some((Token::Sub, pos)),
            '*' => Some((Token::Mul, pos)),
            '/' => Some((Token::Div, pos)),
            '%' => Some((Token::Rem, pos)),

            _ => None,
        };
        if matched.is_some() {
            return Ok(matched);
        }

        // Integer
        let range = '0'..='9';
        if range.contains(&c) {
            let mut str = String::from(c);
            loop {
                if let Some(c) = self.peekc() {
                    if range.contains(&c) {
                        str.push(self.getc().unwrap());
                        continue;
                    }
                }
                break;
            }
            if let Ok(n) = str.parse::<i64>() {
                return Ok(Some((Token::Integer(n), pos)));
            } else {
                bail!("{}:{} Invalid number", pos.line, pos.column);
            };
        }

        bail!("{}:{} Invalid character", pos.line, pos.column);
    }
}

pub fn lexical_analyze(src: &str) -> Result<Vec<TokenWithPos>> {
    let src = src.to_owned();
    let mut lexer = Lexer::new(&src);
    let mut result = Vec::new();

    while let Some(tok) = lexer.next_token()? {
        result.push(tok);
    }

    Ok(result)
}

//------------------------------------------------------------------------------

pub enum Ast {
    Operation(Operation),
    Literal(RuntimeValue),
}

pub enum Operation {
    Minus(Box<Ast>),
    Add(Box<Ast>, Box<Ast>),
    Sub(Box<Ast>, Box<Ast>),
    Mul(Box<Ast>, Box<Ast>),
    Div(Box<Ast>, Box<Ast>),
    Rem(Box<Ast>, Box<Ast>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum RuntimeValue {
    Integer(i64),
}

impl std::fmt::Display for RuntimeValue {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            RuntimeValue::Integer(n) => f.write_str(&n.to_string())?,
        }
        Ok(())
    }
}

struct Parser {
    src: VecDeque<TokenWithPos>,
}

impl Parser {
    fn new(src: Vec<TokenWithPos>) -> Self {
        let mut src = VecDeque::from(src);
        src.push_back((Token::Eof, Position { line: 0, column: 0 }));
        Self { src }
    }

    fn peek(&self) -> &TokenWithPos {
        self.src.front().unwrap()
    }

    fn get(&mut self) -> TokenWithPos {
        self.src.pop_front().unwrap()
    }

    /// expr = expr <EOF>
    fn parse_formula(&mut self) -> Result<Ast> {
        let root = self.parse_term();
        let (next, pos) = self.get();
        ensure!(matches!(next, Token::Eof), "Invalid token {}", pos);

        root
    }

    /// expr = term
    fn parse_expr(&mut self) -> Result<Ast> {
        self.parse_term()
    }

    /// term = factor ((<+>|<-> factor)*
    fn parse_term(&mut self) -> Result<Ast> {
        let mut lh = self.parse_factor()?;
        while let (Token::Add | Token::Sub, _) = self.peek() {
            let plh = Box::new(lh);
            let (op, _) = self.get();
            let prh = Box::new(self.parse_factor()?);
            lh = match op {
                Token::Add => Ast::Operation(Operation::Add(plh, prh)),
                Token::Sub => Ast::Operation(Operation::Sub(plh, prh)),
                _ => panic!("logic error"),
            };
        }

        Ok(lh)
    }

    /// factor = unary  (<*>|</>|<%> unary)*
    fn parse_factor(&mut self) -> Result<Ast> {
        let mut lh = self.parse_unary()?;
        while let (Token::Mul | Token::Div | Token::Rem, _) = self.peek() {
            let plh = Box::new(lh);
            let (op, _) = self.get();
            let prh = Box::new(self.parse_unary()?);
            lh = match op {
                Token::Mul => Ast::Operation(Operation::Mul(plh, prh)),
                Token::Div => Ast::Operation(Operation::Div(plh, prh)),
                Token::Rem => Ast::Operation(Operation::Rem(plh, prh)),
                _ => panic!("logic error"),
            };
        }

        Ok(lh)
    }

    /// unary = (<+>|<->)* primary
    fn parse_unary(&mut self) -> Result<Ast> {
        let mut minus: bool = false;
        while let (Token::Add | Token::Sub, _) = self.peek() {
            let (op, _) = self.get();
            match op {
                Token::Add => {}
                Token::Sub => {
                    minus = !minus;
                }
                _ => panic!("logic error"),
            };
        }

        let operand = self.parse_primary()?;
        if !minus {
            Ok(operand)
        } else {
            Ok(Ast::Operation(Operation::Minus(Box::new(operand))))
        }
    }

    /// primary = <(> expr <)> | <INTEGER>
    fn parse_primary(&mut self) -> Result<Ast> {
        let (tok, _) = self.peek();
        match *tok {
            Token::LParen => {
                assert!(matches!(self.get(), (Token::LParen, _)),);
                let ast = self.parse_expr()?;
                ensure!(matches!(self.get(), (Token::RParen, _)), "RPAREN required");

                Ok(ast)
            }
            Token::Integer(n) => {
                assert!(matches!(self.get(), (Token::Integer(_), _)),);

                Ok(Ast::Literal(RuntimeValue::Integer(n)))
            }
            _ => {
                bail!("Parse error");
            }
        }
    }
}

pub fn parse_formula(src: Vec<TokenWithPos>) -> Result<Ast> {
    let mut parser = Parser::new(src);
    let root = parser.parse_formula()?;

    Ok(root)
}

//------------------------------------------------------------------------------

impl RuntimeValue {
    fn minus(self) -> Result<Self> {
        match self {
            Self::Integer(n) => n
                .checked_neg()
                .ok_or_else(|| anyhow!("overflow"))
                .map(Self::Integer),
        }
    }

    fn add(self, rh: Self) -> Result<Self> {
        match self {
            Self::Integer(a) => match rh {
                Self::Integer(b) => a
                    .checked_add(b)
                    .ok_or_else(|| anyhow!("overflow"))
                    .map(Self::Integer),
            },
        }
    }

    fn sub(self, rh: Self) -> Result<Self> {
        match self {
            Self::Integer(a) => match rh {
                Self::Integer(b) => a
                    .checked_sub(b)
                    .ok_or_else(|| anyhow!("overflow"))
                    .map(Self::Integer),
            },
        }
    }

    fn mul(self, rh: Self) -> Result<Self> {
        match self {
            Self::Integer(a) => match rh {
                Self::Integer(b) => a
                    .checked_mul(b)
                    .ok_or_else(|| anyhow!("overflow"))
                    .map(Self::Integer),
            },
        }
    }

    fn div(self, rh: Self) -> Result<Self> {
        match self {
            Self::Integer(a) => match rh {
                Self::Integer(b) => a
                    .checked_div_euclid(b)
                    .ok_or_else(|| {
                        anyhow!(if b == 0 {
                            "division by zero"
                        } else {
                            "overflow"
                        })
                    })
                    .map(Self::Integer),
            },
        }
    }

    fn rem(self, rh: Self) -> Result<Self> {
        match self {
            Self::Integer(a) => match rh {
                Self::Integer(b) => a
                    .checked_rem_euclid(b)
                    .ok_or_else(|| anyhow!("overflow"))
                    .map(Self::Integer),
            },
        }
    }
}

fn evaluate_operation(op: Operation) -> Result<RuntimeValue> {
    match op {
        Operation::Minus(operand) => {
            let v = evaluate(*operand)?;
            v.minus()
        }
        Operation::Add(lh, rh) => evaluate(*lh)?.add(evaluate(*rh)?),
        Operation::Sub(lh, rh) => evaluate(*lh)?.sub(evaluate(*rh)?),
        Operation::Mul(lh, rh) => evaluate(*lh)?.mul(evaluate(*rh)?),
        Operation::Div(lh, rh) => evaluate(*lh)?.div(evaluate(*rh)?),
        Operation::Rem(lh, rh) => evaluate(*lh)?.rem(evaluate(*rh)?),
    }
}

pub fn evaluate(ast: Ast) -> Result<RuntimeValue> {
    match ast {
        Ast::Literal(v) => Ok(v),
        Ast::Operation(op) => evaluate_operation(op),
    }
}

//------------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lex() {
        let src = "
(
123 + 456
)";
        let toks = lexical_analyze(src).unwrap();
        assert_eq!(5, toks.len());

        assert_eq!((Token::LParen, Position { line: 2, column: 1 }), toks[0]);
        assert_eq!(
            (Token::Integer(123), Position { line: 3, column: 1 }),
            toks[1]
        );
        assert_eq!((Token::Add, Position { line: 3, column: 5 }), toks[2]);
        assert_eq!(
            (Token::Integer(456), Position { line: 3, column: 7 }),
            toks[3]
        );
        assert_eq!((Token::RParen, Position { line: 4, column: 1 }), toks[4]);
    }

    #[test]
    fn parse_eval() {
        let src = "
(((1 + 2) * 3) - --(1 + 2 * 3))
";
        let toks = lexical_analyze(src).unwrap();
        let root = parse_formula(toks).unwrap();
        let v = evaluate(root).unwrap();
        assert_eq!(RuntimeValue::Integer(2), v);
    }
}
