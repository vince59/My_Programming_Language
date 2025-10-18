// Vincent Pineau 04/10/2025
// My Programming Language
// lexer to read the language token and keyword

use crate::grammar::{self, Token};

use std::path::PathBuf;

// position in a source code
#[derive(Debug, Clone)]
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

    // check en of file
    fn eof(&self) -> bool {
        self.i >= self.src_code.len()
    }

    // see the next byte without increase the cursor
    fn peek(&self) -> Option<u8> {
        self.src_code.as_bytes().get(self.i).copied()
    }

    // return the next byte and increase the cursor
    fn bump(&mut self) -> Option<u8> {
        let b = self.peek()?;
        self.i += 1;
        if b == b'\n' {
            self.pos.line += 1;
            self.pos.col = 1;
        } else {
            self.pos.col += 1;
        }
        Some(b)
    }

    // skip spaces and other separators
    fn skip_ws_and_comments(&mut self) -> Result<(), LexError> {
        loop {
            // 1) skip spaces
            while let Some(b) = self.peek() {
                match b {
                    b' ' | b'\t' | b'\r' | b'\n' => {
                        self.bump();
                    }
                    _ => break,
                }
            }
            // 2) comments single line //
            if self.starts_with("//") {
                // consume '//'
                self.bump();
                self.bump();
                // advance until '\n' or EOF
                while let Some(b) = self.peek() {
                    if b == b'\n' {
                        break;
                    }
                    self.bump();
                }
                // restart the loop (in case there's whitespace + another comment right after)
                continue;
            }
            // 3) block comments /* ... */
            if self.starts_with("/*") {
                // consume '/*'
                self.bump();
                self.bump();
                // advance until '*/' or EOF
                while !self.eof() {
                    if self.starts_with("*/") {
                        self.bump();
                        self.bump(); // consume '*/'
                        break;
                    }
                    self.bump(); // consume byte by byte to keep line/col correct
                }
                if self.eof() && !self.starts_with("*/") {
                    return Err(LexError {
                        message: "block comment not terminated (*/ missing)".into(),
                        pos: self.pos.clone(),
                    });
                }
                // restart the loop (in case there's whitespace + another comment right after)
                continue;
            }
            // nothing else to skip
            break;
        }
        Ok(())
    }

    // check if the input starts with the searched token
    fn starts_with(&self, s: &str) -> bool {
        self.src_code[self.i..].starts_with(s)
    }

    // checks if the next token is the one being searched for (s)
    fn try_take(&mut self, s: &str) -> bool {
        if self.starts_with(s) {
            self.i += s.len();
            self.pos.col += s.len();
            true
        } else {
            false
        }
    }

    // try to see if the next token is a symbol
    fn try_symbol(&mut self) -> Option<Token> {
        if self.try_take(grammar::LPAREN) {
            return Some(Token::LParen);
        }
        if self.try_take(grammar::RPAREN) {
            return Some(Token::RParen);
        }
        if self.try_take(grammar::LBRACE) {
            return Some(Token::LBrace);
        }
        if self.try_take(grammar::RBRACE) {
            return Some(Token::RBrace);
        }
        if self.try_take(grammar::COMMA) {
            return Some(Token::Comma);
        }
        if self.try_take(grammar::PLUS) {
            return Some(Token::Plus);
        }
        if self.try_take(grammar::MINUS) {
            return Some(Token::Minus);
        }
        if self.try_take(grammar::STAR) {
            return Some(Token::Star);
        }
        if self.try_take(grammar::SLASH) {
            return Some(Token::Slash);
        }
        None
    }

    // read a valid string
    fn read_string(&mut self) -> Result<Token, LexError> {
        self.bump(); // eat the opening quote
        let start = self.i;
        while let Some(b) = self.peek() {
            if b == b'"' {
                let text = self.src_code[start..self.i].to_string();
                self.bump(); // eat the closing quote
                return Ok(Token::Str(text));
            }
            self.bump();
        }
        Err(LexError {
            message: "incomplete string (\" missing)".into(),
            pos: self.pos.clone(),
        })
    }

    // Read an integer literal
    fn read_integer(&mut self) -> (&str, usize, usize) {
        let s = self.i;
        while let Some(b) = self.peek() {
            if (b'0'..=b'9').contains(&b) {
                self.bump();
            } else {
                break;
            }
        }
        (&self.src_code[s..self.i], s, self.i)
    }

    // Read a number: integer or real decimal (e.g., 0, 42, 0.1, 3., 10.000)
    fn read_number(&mut self) -> (&str, usize, usize) {
        let s = self.i;

        // Read the integer part (at least one digit)
        while let Some(b) = self.peek() {
            if (b'0'..=b'9').contains(&b) {
                self.bump();
            } else {
                break;
            }
        }

        // Optional fractional part
        if self.peek() == Some(b'.') {
            self.bump(); // consume '.'

            // Read 0 or more digits after the dot (so "3." is valid)
            while let Some(b) = self.peek() {
                if (b'0'..=b'9').contains(&b) {
                    self.bump();
                } else {
                    break;
                }
            }
        }

        (&self.src_code[s..self.i], s, self.i)
    }

    // ident can start with a upper or lower case letter or underscore
    fn is_ident_start(b: u8) -> bool {
        (b'a'..=b'z').contains(&b) || (b'A'..=b'Z').contains(&b) || b == b'_'
    }

    // check the next characters of the ident same as ident_start plus digits
    fn is_ident_continue(b: u8) -> bool {
        Self::is_ident_start(b) || (b'0'..=b'9').contains(&b)
    }

    // read an ident (variable or function name)
    fn read_ident(&mut self) -> (&str, usize, usize) {
        let s = self.i;
        while let Some(b) = self.peek() {
            if Self::is_ident_continue(b) {
                self.bump();
            } else {
                break;
            }
        }
        (&self.src_code[s..self.i], s, self.i) // return the ident, start and end position
    }

    // read the next token
    pub fn next_token(&mut self) -> Result<(Token, Position), LexError> {
        let _ = self.skip_ws_and_comments();
        if self.eof() {
            return Ok((Token::Eof, self.pos.clone()));
        }
        if let Some(t) = self.try_symbol() {
            return Ok((t, self.pos.clone()));
        }
        if self.peek() == Some(b'"') {
            return Ok((self.read_string()?, self.pos.clone()));
        }
        // check if the token is an ident or a keyword
        if let Some(b) = self.peek() {
            if Self::is_ident_start(b) {
                let (id, _, _) = self.read_ident();
                return Ok((
                    match id {
                        // check if the id is a key word
                        grammar::KW_IMPORT => Token::Import,
                        grammar::KW_CALL => Token::Call,
                        grammar::KW_FN => Token::Fn,
                        grammar::KW_MAIN => Token::Main,
                        grammar::KW_PRINT => Token::Print,
                        grammar::KW_TO_STR => Token::ToStr,
                        grammar::KW_NL => Token::Nl,
                        _ => Token::Ident(id.to_string()), // if not it is an ident
                    },
                    self.pos.clone(),
                ));
            }
            // Number literal
            if (b'0'..=b'9').contains(&b) {
                let (lexeme, _, _) = self.read_number();

                if lexeme.contains('.') {
                    // Support numbers like "123." by adding a trailing zero for parsing
                    let value_str = if lexeme.ends_with('.') {
                        let mut s = String::from(lexeme);
                        s.push('0');
                        s
                    } else {
                        lexeme.to_string()
                    };

                    let value = value_str.parse::<f64>().map_err(|_| LexError {
                        message: "invalid real number format".to_string(),
                        pos: self.pos.clone(),
                    })?;

                    return Ok((Token::Real(value), self.pos.clone()));
                } else {
                    let value = lexeme.parse::<i32>().map_err(|_| LexError {
                        message: "invalid integer format".to_string(),
                        pos: self.pos.clone(),
                    })?;

                    return Ok((Token::Integer(value), self.pos.clone()));
                }
            }
        }
        let b = self.peek().unwrap();
        let msg = if b.is_ascii() {
            format!(
                "unexpected token: '{}' (0x{:02X})",
                (b as char).escape_default(),
                b
            )
        } else {
            // Non-ASCII byte; don't pretend it's a full Unicode char
            format!("unexpected octet: 0x{:02X} (non-ASCII)", b)
        };
        Err(LexError {
            message: msg,
            pos: self.pos.clone(),
        })
    }
}
