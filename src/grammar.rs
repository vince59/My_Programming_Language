// Vincent Pineau 04/10/2025
// My Programming Language
// all the keywords, operators ...

#[derive(Debug,Clone)]
pub enum Token {
    Import,
    Fn,
    Main,
    Log,
    Call,
    Ident(String),
    Str(String),
    LParen,
    RParen,
    LBrace,
    RBrace,
    Comma,
    Eof,
}

pub const KW_IMPORT: &str = "import";
pub const KW_FN:     &str = "fn";
pub const KW_MAIN:   &str = "main";
pub const KW_LOG:    &str = "log";
pub const KW_CALL:   &str = "call"; 

pub const LPAREN:  &str = "(";
pub const RPAREN:  &str = ")";
pub const LBRACE:  &str = "{";
pub const RBRACE:  &str = "}";
pub const COMMA:   &str = ",";

pub const EOF:   &str = "end of file";
