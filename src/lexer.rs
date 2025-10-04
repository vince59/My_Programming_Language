// Vincent Pineau 04/10/2025
// My Programming Language
// lexer to read the language token and keyword

use crate::grammar::Token;
use std::path::PathBuf;

// position in a source code
#[derive(Debug,Clone)]
pub struct Position {
    pub file_name: PathBuf, // source file name
    pub line: usize,        // line number
    pub col: usize,         // column number
}

impl Position {
    pub fn new(file_name: PathBuf) -> Self {
        Self {
            file_name,
            line: 1,
            col: 1,
        }
    }
}

// lexer error
#[derive(Debug)]
pub struct LexError {
    pub message: String,
    pub pos: Position,
}

// format the way the lex error is displayed
impl std::fmt::Display for LexError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            " Token error : {}\n in file {}\n at line {}\n col {}\n",
            self.message,
            self.pos.file_name.to_string_lossy(),
            self.pos.line,
            self.pos.col
        )
    }
}

impl std::error::Error for LexError {}

pub struct Lexer {
    pos: Position,    // current position
    src_code: String, // source code
    i: usize,         // index byte
}

impl Lexer {
    pub fn new(file_name: impl Into<PathBuf>, src_code: impl Into<String>) -> Self {
        Self {
            src_code: src_code.into(),
            i: 0,
            pos: Position::new(file_name.into()),
        }
    }

    pub fn next_token(&self) -> Result<(Token, Position), LexError> {
        print!("{}", self.src_code);
        Ok((Token::Eof, self.pos.clone()))
    }
}
