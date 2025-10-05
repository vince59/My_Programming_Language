// Vincent Pineau 04/10/2025
// My Programming Language
// all the keywords, operators ...

#[derive(Debug,Clone)]
pub enum Token {
    Import,
    Fn,
    Main,
    Print,
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
pub const KW_PRINT:    &str = "print";
pub const KW_CALL:   &str = "call"; 

pub const LPAREN:  &str = "(";
pub const RPAREN:  &str = ")";
pub const LBRACE:  &str = "{";
pub const RBRACE:  &str = "}";
pub const COMMA:   &str = ",";

pub const EOF:   &str = "end of file";

#[macro_export]
macro_rules! expect {
    // --- Avec payload: retourne une valeur possédée (pas de clone) ---
    ($self:ident, $pat:pat => $out:expr, $expected:expr) => {{
        let tok = ::std::mem::replace(&mut $self.token, Token::Eof); // move
        match tok {
            $pat => {
                let __v = $out;             // ex: s (String)
                $self.next_token()?;        // avance au token suivant
                Ok(__v)
            }
            other => {
                let err = ParseError::Unexpected {
                    found: other.clone(),
                    expected: $expected,
                    pos: $self.pos.clone(),
                };
                $self.token = other;        // restaure l'état
                Err(err)
            }
        }
    }};
    // --- Sans payload: juste vérifie et avance, retourne () ---
    ($self:ident, $pat:pat, $expected:expr) => {{
        let tok = ::std::mem::replace(&mut $self.token, Token::Eof);
        match tok {
            $pat => {
                $self.next_token()?;        // avance
                Ok(())
            }
            other => {
                let err = ParseError::Unexpected {
                    found: other.clone(),
                    expected: $expected,
                    pos: $self.pos.clone(),
                };
                $self.token = other;        // restaure l'état
                Err(err)
            }
        }
    }};
}