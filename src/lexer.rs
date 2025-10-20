// Vincent Pineau 04/10/2025
// My Programming Language
// Lexer to read tokens and keywords

use crate::grammar::{self, Token};
use std::path::PathBuf;

// Position in a source file
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

// Lexer error
#[derive(Debug)]
pub struct LexError {
    pub message: String,
    pub pos: Position,
}

// Format how a lex error is displayed
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
    pos: Position,    // current position (file, line, col)
    src_code: String, // full source code
    i: usize,         // byte index (always on a UTF-8 char boundary)
}

impl Lexer {
    pub fn new(file_name: impl Into<PathBuf>, src_code: impl Into<String>) -> Self {
        Self {
            src_code: src_code.into(),
            i: 0,
            pos: Position::new(file_name.into()),
        }
    }

    // --- UTF-8 safe helpers ---

    #[inline]
    fn rest(&self) -> &str {
        // Always return a valid slice (or empty if out of range)
        self.src_code.get(self.i..).unwrap_or("")
    }

    // End-of-file?
    #[inline]
    fn eof(&self) -> bool {
        self.i >= self.src_code.len()
    }

    // Peek next char without consuming it
    #[inline]
    fn peek_char(&self) -> Option<char> {
        self.rest().chars().next()
    }

    // Lookahead by 1 (second char)
    #[inline]
    fn peek_next_char(&self) -> Option<char> {
        let mut it = self.rest().chars();
        let _ = it.next()?;
        it.next()
    }

    // Consume one char and advance by char.len_utf8() bytes
    #[inline]
    fn bump(&mut self) -> Option<char> {
        let ch = self.peek_char()?;
        self.i += ch.len_utf8();
        if ch == '\n' {
            self.pos.line += 1;
            self.pos.col = 1;
        } else {
            self.pos.col += 1;
        }
        Some(ch)
    }

    // Check if remaining input starts with a given ASCII prefix (byte-based)
    #[inline]
    fn starts_with(&self, s: &str) -> bool {
        let tail = self.src_code.as_bytes().get(self.i..).unwrap_or(&[]);
        tail.starts_with(s.as_bytes())
    }

    // Consume an exact prefix if present; updates line/col per chars in the prefix
    #[inline]
    fn eat_prefix(&mut self, s: &str) -> bool {
        if self.starts_with(s) {
            self.i += s.len(); // advance in bytes
            // Update line/col using the chars of the prefix
            for ch in s.chars() {
                if ch == '\n' {
                    self.pos.line += 1;
                    self.pos.col = 1;
                } else {
                    self.pos.col += 1;
                }
            }
            true
        } else {
            false
        }
    }

    // --- whitespace & comments ---

    fn skip_ws_and_comments(&mut self) -> Result<(), LexError> {
        loop {
            // 1) Skip ASCII whitespace
            while let Some(ch) = self.peek_char() {
                match ch {
                    ' ' | '\t' | '\r' | '\n' => {
                        self.bump();
                    }
                    _ => break,
                }
            }

            // 2) Line comments: //
            if self.starts_with("//") {
                self.eat_prefix("//");
                while let Some(ch) = self.peek_char() {
                    if ch == '\n' {
                        break;
                    }
                    self.bump();
                }
                continue;
            }

            // 3) Block comments: /* ... */
            if self.starts_with("/*") {
                self.eat_prefix("/*");
                let mut closed = false;
                while let Some(ch) = self.peek_char() {
                    if ch == '*' && self.peek_next_char() == Some('/') {
                        // Consume "*/"
                        self.bump(); // '*'
                        self.bump(); // '/'
                        closed = true;
                        break;
                    } else {
                        self.bump(); // advance by one UTF-8 char
                    }
                }
                if !closed {
                    return Err(LexError {
                        message: "block comment not terminated (*/ missing)".into(),
                        pos: self.pos.clone(),
                    });
                }
                continue;
            }

            break;
        }
        Ok(())
    }

    // --- ASCII symbols / fixed tokens ---

    #[inline]
    fn try_take(&mut self, s: &str) -> bool {
        self.eat_prefix(s)
    }

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
        if self.try_take(grammar::EQUAL) {
            return Some(Token::Equal);
        }
        None
    }

    // --- literals ---

    // Read a string literal: "...." (UTF-8 content)
    fn read_string(&mut self) -> Result<Token, LexError> {
        // consume opening "
        match self.bump() {
            Some('"') => {}
            _ => {
                return Err(LexError {
                    message: "internal: expected opening '\"'".into(),
                    pos: self.pos.clone(),
                })
            }
        }

        let start = self.i;
        while let Some(ch) = self.peek_char() {
            match ch {
                '"' => {
                    // Safe slice: start..i are UTF-8 boundaries
                    let text = self.src_code[start..self.i].to_string();
                    self.bump(); // consume closing "
                    return Ok(Token::Str(text));
                }
                // (optional) handle escapes here if needed
                _ => {
                    self.bump();
                }
            }
        }

        Err(LexError {
            message: "incomplete string (\" missing)".into(),
            pos: self.pos.clone(),
        })
    }

    // ASCII digit check
    #[inline]
    fn is_digit(ch: char) -> bool {
        ch.is_ascii_digit()
    }

    // Read a number: integer or float (e.g., 0, 42, 0.1, 3., 10.000)
    fn read_number(&mut self) -> (&str, usize, usize) {
        let s = self.i;

        // integer part (>= 0 digits; caller ensures at least one)
        while let Some(ch) = self.peek_char() {
            if Self::is_digit(ch) {
                self.bump();
            } else {
                break;
            }
        }

        // optional fractional part
        if self.peek_char() == Some('.') {
            self.bump(); // consume '.'
            // 0+ digits after the dot (so "3." is valid)
            while let Some(ch) = self.peek_char() {
                if Self::is_digit(ch) {
                    self.bump();
                } else {
                    break;
                }
            }
        }

        (&self.src_code[s..self.i], s, self.i)
    }

    // Identifier start: ASCII letter or underscore
    #[inline]
    fn is_ident_start(ch: char) -> bool {
        ch == '_' || ch.is_ascii_alphabetic()
    }

    // Identifier continue: letter/underscore/digit
    #[inline]
    fn is_ident_continue(ch: char) -> bool {
        Self::is_ident_start(ch) || ch.is_ascii_digit()
    }

    // Read an identifier (variable or function name)
    fn read_ident(&mut self) -> (&str, usize, usize) {
        let s = self.i;
        while let Some(ch) = self.peek_char() {
            if Self::is_ident_continue(ch) {
                self.bump();
            } else {
                break;
            }
        }
        (&self.src_code[s..self.i], s, self.i) // return ident slice, start and end indices
    }

    // --- main tokenization entry point ---

    pub fn next_token(&mut self) -> Result<(Token, Position), LexError> {
        self.skip_ws_and_comments()?; // propagate comment/whitespace errors

        if self.eof() {
            return Ok((Token::Eof, self.pos.clone()));
        }

        if let Some(t) = self.try_symbol() {
            return Ok((t, self.pos.clone()));
        }

        if self.peek_char() == Some('"') {
            let tok = self.read_string()?;
            return Ok((tok, self.pos.clone()));
        }

        if let Some(ch) = self.peek_char() {
            // identifier or keyword
            if Self::is_ident_start(ch) {
                let (id, _, _) = self.read_ident();
                let token = match id {
                    // keywords
                    grammar::KW_IMPORT => Token::Import,
                    grammar::KW_CALL => Token::Call,
                    grammar::KW_FN => Token::Fn,
                    grammar::KW_MAIN => Token::Main,
                    grammar::KW_PRINT => Token::Print,
                    grammar::KW_PRINTLN => Token::Println,
                    grammar::KW_TO_STR => Token::ToStr,
                    grammar::KW_NL => Token::Nl,
                    grammar::KW_LOCAL => Token::Local,
                    grammar::KW_TRUE => Token::True,
                    grammar::KW_FALSE => Token::False,
                    grammar::KW_INT_TYPE => Token::IntType,
                    grammar::KW_FLOAT_TYPE => Token::FloatType,
                    grammar::KW_LET => Token::Let,
                    // otherwise, plain identifier
                    _ => Token::Ident(id.to_string()),
                };
                return Ok((token, self.pos.clone()));
            }

            // number literal
            if ch.is_ascii_digit() {
                let (lexeme, _, _) = self.read_number();

                if lexeme.contains('.') {
                    // Support numbers like "123." by appending a trailing zero for parsing
                    let value_str = if lexeme.ends_with('.') {
                        let mut s = String::from(lexeme);
                        s.push('0');
                        s
                    } else {
                        lexeme.to_string()
                    };

                    let value = value_str.parse::<f64>().map_err(|_| LexError {
                        message: "invalid float number format".to_string(),
                        pos: self.pos.clone(),
                    })?;

                    return Ok((Token::Float(value), self.pos.clone()));
                } else {
                    let value = lexeme.parse::<i32>().map_err(|_| LexError {
                        message: "invalid integer format".to_string(),
                        pos: self.pos.clone(),
                    })?;

                    return Ok((Token::Integer(value), self.pos.clone()));
                }
            }
        }

        // Unexpected character: show readable char + code point
        if let Some(ch) = self.peek_char() {
            let cp = ch as u32;
            let msg = if ch.is_ascii() {
                format!("unexpected token: '{}' (0x{:02X})", ch.escape_default(), cp)
            } else {
                format!("unexpected char: '{}' (U+{:04X})", ch, cp)
            };
            Err(LexError {
                message: msg,
                pos: self.pos.clone(),
            })
        } else {
            Err(LexError {
                message: "unexpected end of input".into(),
                pos: self.pos.clone(),
            })
        }
    }
}
