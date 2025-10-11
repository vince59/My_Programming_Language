// Vincent Pineau 04/10/2025
// My Programming Language
// all the keywords, operators ...

#[derive(Debug, Clone)]
pub enum Token {
    Import,
    Fn,
    Main,
    Print,
    Call,
    Ident(String),
    Str(String),
    Integer(String),
    LParen,
    RParen,
    LBrace,
    RBrace,
    Comma,
    Plus,
    Minus,
    Star,
    Slash,
    NumCast,
    Eof,
}

pub const KW_IMPORT: &str = "import";
pub const KW_FN: &str = "fn";
pub const KW_MAIN: &str = "main";
pub const KW_PRINT: &str = "print";
pub const KW_CALL: &str = "call";
pub const KW_NUM_CAST: &str = "to_str";

pub const LPAREN: &str = "(";
pub const RPAREN: &str = ")";
pub const LBRACE: &str = "{";
pub const RBRACE: &str = "}";
pub const COMMA: &str = ",";
pub const PLUS: &str = "+";
pub const MINUS: &str = "-";
pub const STAR: &str = "*";
pub const SLASH: &str = "/";

pub const EOF: &str = "end of file";

// macro to check if current token is the right one and return an error if it's not the case
// if the token is right one it go to the next token and if necessary deserializes the token to get the enum param
// ex : Ident(s) get s if current token is Ident
#[macro_export]
macro_rules! expect {
    // --- With payload
    ($self:ident, $pat:pat => $out:expr, $expected:expr) => {{
        let tok = ::std::mem::replace(&mut $self.token, Token::Eof); // move
        match tok {
            $pat => {
                let __v = $out; // ex: s (String)
                $self.next_token()?; // go to the next token
                Ok(__v)
            }
            other => {
                let err = ParseError::Unexpected {
                    found: other.clone(),
                    expected: $expected,
                    pos: $self.pos.clone(),
                };
                $self.token = other; // restores the state
                Err(err)
            }
        }
    }};
    // --- Without payload
    ($self:ident, $pat:pat, $expected:expr) => {{
        let tok = ::std::mem::replace(&mut $self.token, Token::Eof);
        match tok {
            $pat => {
                $self.next_token()?; // go to the next token
                Ok(())
            }
            other => {
                let err = ParseError::Unexpected {
                    found: other.clone(),
                    expected: $expected,
                    pos: $self.pos.clone(),
                };
                $self.token = other; // restores the state
                Err(err)
            }
        }
    }};
}
