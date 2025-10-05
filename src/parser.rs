// Vincent Pineau 04/10/2025
// My Programming Language
// parser to analyse the language grammar

use std::path::PathBuf;

use crate::grammar::{self, Token};
use crate::lexer::{LexError, Lexer, Position};

#[derive(Debug, Clone)]
pub enum Stadment {
    Print { str: String },
    Call { name: String },
}

#[derive(Debug)]
pub struct Program {
    pub imports: Vec<String>,
    pub functions: Vec<Function>,
    pub main: Function,
}

#[derive(Debug)]
pub struct Function {
    pub name: String,
    pub body: Vec<Stadment>,
}

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
        let token = Token::Eof;
        let pos = Position::new(PathBuf::new());
        Ok(Self { lx, token, pos })
    }

    // Move one token forward
    fn next_token(&mut self) -> Result<(), ParseError> {
        (self.token, self.pos) = self.lx.next_token()?;
        Ok(())
    }

    // main_program ::= [ imports ]
    //                  [ functions ]
    //                  main_function
    pub fn parse_main_program(&mut self) -> Result<Program, ParseError> {
        let imports = self.parse_imports()?;
        let functions = self.parse_functions()?;
        let main = self.parse_main_function()?;
        Ok(Program {
            imports,
            functions,
            main,
        })
    }

    // imports ::= { "IMPORT" str }
    pub fn parse_imports(&mut self) -> Result<Vec<String>, ParseError> {
        let mut paths = Vec::new();
        self.next_token()?; // Get the first token
        while matches!(self.token, Token::Import) {
            self.next_token()?; // get the string after the keyword IMPORT
            let import_name =
                crate::expect!(self,Token::Str(s) => s, "a path string after `import`")?;
            paths.push(import_name);
        }
        Ok(paths)
    }

    // functions ::= { function }
    pub fn parse_functions(&mut self) -> Result<Vec<Function>, ParseError> {
        let mut functions = Vec::new();
        while matches!(self.token, Token::Fn) {
            functions.push(self.parse_function()?);
        }
        Ok(functions)
    }

    // function ::= FN ident '(' ')' '{'
    //                           [ { stadment } ]
    //                       '}'
    pub fn parse_function(&mut self) -> Result<Function, ParseError> {
        let mut body = Vec::new();
        crate::expect!(self, Token::Fn, grammar::KW_FN)?;
        let name = crate::expect!(self,Token::Str(s) => s, "a valid function name after `fn`")?;
        crate::expect!(self, Token::LParen, grammar::LPAREN)?;
        crate::expect!(self, Token::RParen, grammar::RPAREN)?;
        crate::expect!(self, Token::LBrace, grammar::LBRACE)?;
        while !matches!(self.token, Token::RBrace) {
            body.push(self.parse_stadment()?);
        }
        crate::expect!(self, Token::RBrace, grammar::RBRACE)?;
        Ok(Function { name, body })
    }

    //stadment ::= call_function | print
    pub fn parse_stadment(&mut self) -> Result<Stadment, ParseError> {
        match &self.token {
            Token::Call => self.parse_call_function(),
            Token::Print => self.parse_print(),
            _ => Err(ParseError::Unexpected {
                found: self.token.clone(),
                expected: "an instruction",
                pos: self.pos.clone(),
            }),
        }
    }

    // main_function ::=  MAIN '(' ')' '{'
    //                        [ { stadment } ]
    //                    '}'
    //                    EOF
    pub fn parse_main_function(&mut self) -> Result<Function, ParseError> {
        let mut body = Vec::new();

        crate::expect!(self, Token::Main, grammar::KW_MAIN)?;
        crate::expect!(self, Token::LParen, grammar::LPAREN)?;
        crate::expect!(self, Token::RParen, grammar::RPAREN)?;
        crate::expect!(self, Token::LBrace, grammar::LBRACE)?;
        while !matches!(self.token, Token::RBrace) {
            body.push(self.parse_stadment()?);
        }
        crate::expect!(self, Token::RBrace, grammar::RBRACE)?;
        crate::expect!(self, Token::Eof, grammar::EOF)?;
        Ok(Function {
            name: grammar::KW_MAIN.to_string(),
            body,
        })
    }

    // call_function ::=  CALL ident '(' ')'
    pub fn parse_call_function(&mut self) -> Result<Stadment, ParseError> {
        crate::expect!(self, Token::Call, grammar::KW_CALL)?;
        let name = crate::expect!(self,Token::Ident(s) => s, "a valid function name after `call`")?;
        crate::expect!(self, Token::LParen, grammar::LPAREN)?;
        crate::expect!(self, Token::RParen, grammar::RPAREN)?;
        Ok(Stadment::Call { name })
    }

    // print ::=  PRINT '(' str ')'
    pub fn parse_print(&mut self) -> Result<Stadment, ParseError> {
        crate::expect!(self, Token::Print, grammar::KW_PRINT)?;
        crate::expect!(self, Token::LParen, grammar::LPAREN)?;
        let str = crate::expect!(self,Token::Str(s) => s, "a string after `(`")?;
        crate::expect!(self, Token::RParen, grammar::RPAREN)?;
        Ok(Stadment::Print { str })
    }
}
