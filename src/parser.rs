// Vincent Pineau 04/10/2025
// My Programming Language
// parser to analyse the language grammar

use std::path::PathBuf;

use crate::codegen::Ty;
use crate::grammar::{self, Token};
use crate::lexer::{LexError, Lexer, Position};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
}

#[derive(Debug, Clone)]
pub enum Expr {
    Num(NumExpr),
    Str(StrExpr),
}

#[derive(Debug, Clone)]
pub enum NumExpr {
    Int(i32),
    Float(f64),
    Binary {
        op: BinOp,
        left: Box<NumExpr>,
        right: Box<NumExpr>,
    },
    Var {
        var: Variable,
        pos: Position,
    },
    Neg(Box<NumExpr>),
}

#[derive(Debug, Clone)]
pub enum StrExpr {
    Str(String),
    NumToStr(Box<NumExpr>),
    Nl,
}

#[derive(Debug, Clone)]
pub enum Stadment {
    Print(Vec<StrExpr>),
    Println(Vec<StrExpr>),
    Call {
        name: String,
        pos: Position,
    },
    Assignment {
        var: Variable,
        expr: Expr,
        pos: Position,
    },
}

#[derive(Debug)]
pub struct Program {
    pub functions: Vec<Function>,
    pub main_program: MainProgram,
}

#[derive(Debug)]
pub struct MainProgram {
    pub imports: Vec<String>,
    pub functions: Vec<Function>,
    pub main: Function,
}

#[derive(Debug, Clone)]
pub struct Variable {
    pub name: String,
    pub ty: Ty,
}

pub fn find_variable_index(variables: &[Variable], name: &str) -> Option<usize> {
    variables.iter().position(|v| v.name == name)
}

#[derive(Debug)]
pub struct Function {
    pub name: String,
    pub body: Vec<Stadment>,
    pub variables: Vec<Variable>,
}

#[derive(Debug)]
pub enum ParseError {
    Lex(LexError),
    Unexpected {
        found: Token,
        expected: &'static str,
        pos: Position,
    },
    Generator {
        pos: Position,
        msg: String,
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
            Self::Generator { pos, msg } => write!(
                f,
                " Code generation error : {}\n in file {}\n at line {}\n col {}\n",
                msg,
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

    // library ::= [ functions ]
    pub fn parse_library(&mut self) -> Result<Vec<Function>, ParseError> {
        self.next_token()?; // Get the first token
        Ok(self.parse_functions()?)
    }

    // main_program ::= [ imports ]
    //                  [ functions ]
    //                  main_function
    pub fn parse_main_program(&mut self) -> Result<MainProgram, ParseError> {
        let imports = self.parse_imports()?;
        let functions = self.parse_functions()?;
        let main = self.parse_main_function()?;
        Ok(MainProgram {
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
            let (import_name, pos) =
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
    //                           [ { variable_declaration } ]
    //                           [ { stadment } ]
    //                       '}'
    pub fn parse_function(&mut self) -> Result<Function, ParseError> {
        let mut body = Vec::new();
        let mut variables = Vec::new();
        crate::expect!(self, Token::Fn, grammar::KW_FN)?;
        let (name, pos) =
            crate::expect!(self,Token::Ident(s) => s, "a valid function name after `fn`")?;
        crate::expect!(self, Token::LParen, grammar::LPAREN)?;
        crate::expect!(self, Token::RParen, grammar::RPAREN)?;
        crate::expect!(self, Token::LBrace, grammar::LBRACE)?;
        while matches!(self.token, Token::Local) {
            variables.push(self.parse_variable_declaration()?);
        }
        while !matches!(self.token, Token::RBrace) {
            body.push(self.parse_stadment(&variables)?); // gives the local variables to check assignments
        }
        crate::expect!(self, Token::RBrace, grammar::RBRACE)?;
        Ok(Function {
            name,
            body,
            variables,
        })
    }

    //stadment ::= call_function | print | assignment
    pub fn parse_stadment(&mut self, variables: &Vec<Variable>) -> Result<Stadment, ParseError> {
        match &self.token {
            Token::Call => self.parse_call_function(),
            Token::Print => self.parse_print(variables, false),
            Token::Println => self.parse_print(variables, true),
            Token::Let => self.parse_assignment(variables),
            _ => Err(ParseError::Unexpected {
                found: self.token.clone(),
                expected: "an instruction",
                pos: self.pos.clone(),
            }),
        }
    }

    // main_function ::=  MAIN '(' ')' '{'
    //                        [ { variable_declaration } ]
    //                        [ { stadment } ]
    //                    '}'
    //                    EOF
    pub fn parse_main_function(&mut self) -> Result<Function, ParseError> {
        let mut body = Vec::new();
        let mut variables = Vec::new();
        crate::expect!(self, Token::Main, grammar::KW_MAIN)?;
        crate::expect!(self, Token::LParen, grammar::LPAREN)?;
        crate::expect!(self, Token::RParen, grammar::RPAREN)?;
        crate::expect!(self, Token::LBrace, grammar::LBRACE)?;
        while matches!(self.token, Token::Local) {
            variables.push(self.parse_variable_declaration()?);
        }
        while !matches!(self.token, Token::RBrace) {
            body.push(self.parse_stadment(&variables)?);
        }
        crate::expect!(self, Token::RBrace, grammar::RBRACE)?;
        crate::expect!(self, Token::Eof, grammar::EOF)?;
        Ok(Function {
            name: grammar::KW_MAIN.to_string(),
            body,
            variables,
        })
    }

    // call_function ::=  CALL ident '(' ')'
    pub fn parse_call_function(&mut self) -> Result<Stadment, ParseError> {
        crate::expect!(self, Token::Call, grammar::KW_CALL)?;
        let (name, pos) =
            crate::expect!(self,Token::Ident(s) => s, "a valid function name after `call`")?;
        crate::expect!(self, Token::LParen, grammar::LPAREN)?;
        crate::expect!(self, Token::RParen, grammar::RPAREN)?;
        Ok(Stadment::Call { name, pos })
    }

    pub fn parse_expr(&mut self, variables: &Vec<Variable>) -> Result<Expr, ParseError> {
        let num_expr = self.parse_num_expr(variables)?;
        Ok(Expr::Num(num_expr))
    }

    // assignment ::=  LET ident '=' expr
    pub fn parse_assignment(&mut self, variables: &Vec<Variable>) -> Result<Stadment, ParseError> {
        crate::expect!(self, Token::Let, grammar::KW_LET)?;
        let (var_name, pos) =
            crate::expect!(self, Token::Ident(s) => s, "a valid variable name after `let`")?;
        crate::expect!(self, Token::Equal, grammar::EQUAL)?;
        // check if the variable exists
        let var_index =
            find_variable_index(variables, &var_name).ok_or_else(|| ParseError::Generator {
                pos: pos.clone(),
                msg: format!("Variable '{}' not declared", var_name),
            })?;
        let var = variables[var_index].clone();
        let expr = self.parse_expr(variables)?;
        Ok(Stadment::Assignment { var, expr, pos })
    }

    // print ::=  (PRINT | PRINTLN) '(' str_expr [',' str_expr] ')'
    
    pub fn parse_print(
        &mut self,
        variables: &Vec<Variable>,
        nl: bool,
    ) -> Result<Stadment, ParseError> {
        if nl {
            crate::expect!(self, Token::Println, grammar::KW_PRINTLN)?;
        } else {
            crate::expect!(self, Token::Print, grammar::KW_PRINT)?;
        }
        crate::expect!(self, Token::LParen, grammar::LPAREN)?;
        let mut str_expr: Vec<StrExpr> = Vec::new();
        str_expr.push(self.parse_str_expr(variables)?);
        while matches!(self.token, Token::Comma) {
            self.next_token()?;
            str_expr.push(self.parse_str_expr(variables)?);
        }
        crate::expect!(self, Token::RParen, grammar::RPAREN)?;
        if nl {
            Ok(Stadment::Println(str_expr))
        } else {
            Ok(Stadment::Print(str_expr))
        }
    }

    // str_expr ::= str | to_str(num_expr) | NL
    fn parse_str_expr(&mut self, variables: &Vec<Variable>) -> Result<StrExpr, ParseError> {
        let tok = self.token.clone();
        match tok {
            Token::Str(s) => {
                self.next_token()?;
                Ok(StrExpr::Str(s))
            }
            Token::ToStr => {
                self.next_token()?;
                crate::expect!(self, Token::LParen, grammar::LPAREN)?;
                let inner = self.parse_num_expr(variables)?;
                crate::expect!(self, Token::RParen, grammar::RPAREN)?;
                Ok(StrExpr::NumToStr(Box::new(inner)))
            }
            Token::Nl => {
                self.next_token()?;
                Ok(StrExpr::Nl)
            }
            _ => Err(ParseError::Unexpected {
                found: self.token.clone(),
                expected: "a string or to_str(num)",
                pos: self.pos.clone(),
            }),
        }
    }

    // expr ::= additive
    fn parse_num_expr(&mut self, variables: &Vec<Variable>) -> Result<NumExpr, ParseError> {
        self.parse_additive(variables)
    }

    // additive ::= multiplicative { ('+' | '-') multiplicative }
    fn parse_additive(&mut self, variables: &Vec<Variable>) -> Result<NumExpr, ParseError> {
        let mut node = self.parse_multiplicative(variables)?;
        loop {
            let op = match &self.token {
                Token::Plus => {
                    self.next_token()?;
                    BinOp::Add
                }
                Token::Minus => {
                    self.next_token()?;
                    BinOp::Sub
                }
                _ => break,
            };
            let rhs = self.parse_multiplicative(variables)?;
            node = NumExpr::Binary {
                op,
                left: Box::new(node),
                right: Box::new(rhs),
            };
        }
        Ok(node)
    }

    // multiplicative ::= unary { ('*' | '/') unary }
    fn parse_multiplicative(&mut self, variables: &Vec<Variable>) -> Result<NumExpr, ParseError> {
        let mut node = self.parse_unary(variables)?;
        loop {
            let op = match &self.token {
                Token::Star => {
                    self.next_token()?;
                    BinOp::Mul
                }
                Token::Slash => {
                    self.next_token()?;
                    BinOp::Div
                }
                _ => break,
            };
            let rhs = self.parse_unary(variables)?;
            node = NumExpr::Binary {
                op,
                left: Box::new(node),
                right: Box::new(rhs),
            };
        }
        Ok(node)
    }

    // unary ::= { '+' | '-' } primary
    fn parse_unary(&mut self, variables: &Vec<Variable>) -> Result<NumExpr, ParseError> {
        // Count/minimize leading signs; handle chains like - - + - x
        let mut minus_count = 0usize;
        loop {
            match self.token {
                Token::Minus => {
                    self.next_token()?;
                    minus_count ^= 1;
                } // flip parity
                Token::Plus => {
                    self.next_token()?; /* unary + is a no-op */
                }
                _ => break,
            }
        }

        let base = self.parse_primary(variables)?;
        if minus_count % 2 == 1 {
            Ok(NumExpr::Neg(Box::new(base)))
        } else {
            Ok(base)
        }
    }

    // primary ::= INT | FLOAT |'(' expr ')' | ident
    fn parse_primary(&mut self, variables: &Vec<Variable>) -> Result<NumExpr, ParseError> {
        let tok = self.token.clone();
        match tok {
            Token::Integer(n) => {
                self.next_token()?;
                Ok(NumExpr::Int(n))
            }
            Token::Float(n) => {
                self.next_token()?;
                Ok(NumExpr::Float(n))
            }
            Token::LParen => {
                self.next_token()?;
                let e = self.parse_num_expr(variables)?;
                crate::expect!(self, Token::RParen, grammar::RPAREN)?;
                Ok(e)
            }
            Token::Ident(ref var_name) => {
                self.next_token()?;
                let var_index = find_variable_index(variables, &var_name).ok_or_else(|| {
                    ParseError::Generator {
                        pos: self.pos.clone(),
                        msg: format!("Variable '{}' not declared", var_name),
                    }
                })?;
                let var = variables[var_index].clone();
                Ok(NumExpr::Var {
                    var,
                    pos: self.pos.clone(),
                })
            }
            _ => Err(ParseError::Unexpected {
                found: self.token.clone(),
                expected: "an expression",
                pos: self.pos.clone(),
            }),
        }
    }
    // type ::= INT | FLOAT
    fn parse_type(&mut self) -> Result<Ty, ParseError> {
        match self.token {
            Token::IntType => {
                self.next_token()?;
                Ok(Ty::I32)
            }
            Token::FloatType => {
                self.next_token()?;
                Ok(Ty::F64)
            }
            _ => Err(ParseError::Unexpected {
                found: self.token.clone(),
                expected: "a type (int or float)",
                pos: self.pos.clone(),
            }),
        }
    }

    // variable_declaration ::= LOCAL type ident
    fn parse_variable_declaration(&mut self) -> Result<Variable, ParseError> {
        crate::expect!(self, Token::Local, grammar::KW_LOCAL)?;
        let ty = self.parse_type()?;
        let (name, pos) =
            crate::expect!(self,Token::Ident(s) => s, "a valid variable name after `local type`")?;
        Ok(Variable { name, ty })
    }
}
