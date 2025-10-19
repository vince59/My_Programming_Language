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
    Integer(i32),
    Float(f64),
    ToStr,
    LParen,
    RParen,
    LBrace,
    RBrace,
    Comma,
    Plus,
    Minus,
    Star,
    Slash,
    Nl,
    Local,
    True,
    False,
    Equal,
    IntType,
    FloatType,
    Let,
    Eof,
}

pub const KW_IMPORT: &str = "import";
pub const KW_FN: &str = "fn";
pub const KW_MAIN: &str = "main";
pub const KW_PRINT: &str = "print";
pub const KW_CALL: &str = "call";
pub const KW_TO_STR: &str = "to_str";
pub const KW_NL: &str = "nl";
pub const KW_LOCAL: &str = "local";
pub const KW_TRUE: &str = "true";
pub const KW_FALSE: &str = "false";
pub const KW_INT_TYPE: &str = "int";
pub const KW_FLOAT_TYPE: &str = "float";
pub const KW_LET: &str = "let";

pub const LPAREN: &str = "(";
pub const RPAREN: &str = ")";
pub const LBRACE: &str = "{";
pub const RBRACE: &str = "}";
pub const COMMA: &str = ",";
pub const PLUS: &str = "+";
pub const MINUS: &str = "-";
pub const STAR: &str = "*";
pub const SLASH: &str = "/";
pub const EQUAL: &str = "=";

pub const EOF: &str = "end of file";

// macro to check if current token is the right one and return an error if it's not the case
// if the token is right one it go to the next token and if necessary deserializes the token to get the enum param
// ex : Ident(s) get s if current token is Ident
#[macro_export]
macro_rules! expect {
    // --- With payload: Ok((out, pos))
    ($self:ident, $pat:pat => $out:expr, $expected:expr) => {{
        // Move the current token out to match on it, keep a snapshot of its position.
        let tok = ::std::mem::replace(&mut $self.token, Token::Eof);
        let pos_snapshot = $self.pos.clone(); // position of the consumed token

        match tok {
            $pat => {
                // Bind the payload (e.g. an identifier String)
                let __v = $out;

                // Advance to the next token after successful match
                $self.next_token()?;

                // Return both the extracted value and the position of the consumed token
                Ok((__v, pos_snapshot))
            }
            other => {
                // Build a precise error using the current position
                let err = ParseError::Unexpected {
                    found: other.clone(),
                    expected: $expected,
                    pos: $self.pos.clone(),
                };

                // Restore the token to keep parser state consistent
                $self.token = other;

                Err(err)
            }
        }
    }};
    // --- Without payload: Ok(pos)
    ($self:ident, $pat:pat, $expected:expr) => {{
        // Same as above, but we only care about the position
        let tok = ::std::mem::replace(&mut $self.token, Token::Eof);
        let pos_snapshot = $self.pos.clone();

        match tok {
            $pat => {
                // Advance to the next token on success
                $self.next_token()?;
                // Return the position of the consumed token
                Ok(pos_snapshot)
            }
            other => {
                let err = ParseError::Unexpected {
                    found: other.clone(),
                    expected: $expected,
                    pos: $self.pos.clone(),
                };

                // Restore parser state
                $self.token = other;

                Err(err)
            }
        }
    }};
}