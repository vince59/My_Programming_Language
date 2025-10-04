// Vincent Pineau 04/10/2025
// My Programming Language
// lexer to read the language token and keyword

pub struct Lexer<'a> {
    src_code: &'a str,   // source code
    i: usize,            // index byte
    line: usize,         // current line source code
    col: usize,          // current column source code
    file_name: &'a str,  // source file name
}


impl<'a> Lexer<'a> {

    pub fn new(file_name: &'a str, src_code: &'a str) -> Self {
        Self {
            src_code,
            i: 0,
            line: 1,
            col: 1,
            file_name,
        }
    }

    pub fn test(&self) {
        print!("{}", self.src_code);
    }
}