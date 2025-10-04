// Vincent Pineau 04/10/2025
// My Programming Language
// parser to analyse the language grammar

use crate::lexer::Lexer;
pub struct Parser {
    lx: Lexer, // lexer
}

impl Parser  {
    pub fn new(lx: Lexer) {
        lx.test();
        Self {lx};
    }
}