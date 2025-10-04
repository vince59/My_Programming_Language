// Vincent Pineau 04/10/2025
// My Programming Language
// A simple compilator for a simple language

mod lexer;
use std::{env, fs, process};
use lexer::Lexer;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // read main source code given on the command line
    let main_src_file = env::args().nth(1).unwrap_or_else(|| {
        eprintln!("Usage : mpl <path to main source code>");
        process::exit(1);
    });

    let src_main_code = fs::read_to_string(&main_src_file)?;
    let lex = Lexer::new(&main_src_file, &src_main_code);
    lex.test();
    Ok(())
}
