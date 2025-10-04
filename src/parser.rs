// Vincent Pineau 04/10/2025
// My Programming Language
// parser to analyse the language grammar

use std::path::PathBuf;

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
        let token= Token::Eof;
        let pos= Position::new(PathBuf::new());
        Ok(Self { lx, token, pos })
    }

    // Move one token forward
    fn next_token(&mut self) -> Result<(), ParseError> {
        (self.token, self.pos) = self.lx.next_token()?;
        Ok(())
    }

    // main_program ::= [ import ]
    pub fn parse_main_program(&mut self) -> Result<Vec<String>, ParseError> {
        let imports = self.parse_imports()?;
        Ok(imports)
    }

    // import ::= { "IMPORT" str }
    pub fn parse_imports(&mut self) -> Result<Vec<String>, ParseError> {
        let mut paths = Vec::new();
        self.next_token()?; // Get the first token
        while matches!(self.token, Token::Import) {
            self.next_token()?; // get the string after the keyword IMPORT
            // extract the string from the token else return an error
            let Token::Str(s) = &self.token else {
                return Err(ParseError::Unexpected {
                    found: self.token.clone(),
                    expected: "a path string after `import`",
                    pos: self.pos.clone(),
                });
            };
            // push the name of the imported file in a vector
            paths.push(s.clone());
            self.next_token()?; // Try next import
        }
        Ok(paths)
    }
}
