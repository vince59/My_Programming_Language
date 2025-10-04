// Vincent Pineau 04/10/2025
// My Programming Language
// lexer to read the language token and keyword

use std::path::PathBuf;

pub struct Lexer {
    file_name: PathBuf,  // source file name
    src_code: String,    // source code
    i: usize,            // index byte
    line: usize,         // current line source code
    col: usize,          // current column source code
}

impl Lexer {

    pub fn new(file_name: impl Into<PathBuf>, src_code: impl Into<String>) -> Self {
        Self {
            src_code: src_code.into(),
            i: 0,
            line: 1,
            col: 1,
            file_name: file_name.into(),
        }
    }

    pub fn test(&self) {
        print!("{}", self.src_code);
    }
}