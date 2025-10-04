// Vincent Pineau 04/10/2025
// My Programming Language
// parser to analyse the language grammar

use crate::grammar::Token;
use crate::lexer::{LexError, Lexer, Position};

#[derive(Debug)]
pub enum ParseError {
    Lex(LexError),
    Unexpected {
        found: Token,
        expected: &'static str,
        pos: Position,
    },
}

impl From<LexError> for ParseError {
    fn from(e: LexError) -> Self {
        Self::Lex(e)
    }
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Lex(e) => write!(f, "{}", e),
            Self::Unexpected {
                found,
                expected,
                pos,
            } => write!(
                f,
                " Grammar error : Expected {}, found {:?}\n in file {}\n at line {}\n col {}\n",
                expected,
                found,
                pos.file_name.to_string_lossy(),
                pos.line,
                pos.col,
            ),
        }
    }
}

impl std::error::Error for ParseError {}

pub struct Parser {
    lx: Lexer,     // lexer
    token: Token,  // current token
    pos: Position, // current position
}

impl Parser {
    pub fn new(lx: Lexer) -> Result<Self, ParseError> {
        let (token, pos) = lx.next_token()?;
        Ok(Self { lx, token, pos })
    }

    pub fn parse_main_program(&self) {

    }
}
