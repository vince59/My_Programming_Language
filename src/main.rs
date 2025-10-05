// Vincent Pineau 04/10/2025
// My Programming Language
// A simple compilator for a simple language

mod lexer;
mod parser;
mod grammar;

use std::{env, fs, path::{Path, PathBuf}, process};
use lexer::Lexer;
use parser::{Parser, Program};

fn resolve_rel(base_file: &Path, rel: &str) -> PathBuf {
    let base_dir = base_file.parent().unwrap_or_else(|| Path::new("."));
    base_dir.join(rel)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // read main source code given on the command line
    let main_src_file : PathBuf  = env::args().nth(1).map(PathBuf::from).unwrap_or_else(|| {
        eprintln!("Usage : mpl <path to main source code>");
        process::exit(1);
    });

    let main_src = fs::read_to_string(&main_src_file)?;
    let lex = Lexer::new(&main_src_file, main_src);
    let mut p = Parser::new(lex)?;
    let main_program = p.parse_main_program()?;
    let mut lib_functions = Vec::new();

    // parse imported source files
    for import in &main_program.imports {
        let import_src_file = resolve_rel(&main_src_file, &import); // build import full path from rel path
        let import_src = fs::read_to_string(&import_src_file)?;
        let lex = Lexer::new(import_src_file, import_src); // new lexer for the import
        let mut p = Parser::new(lex)?;
        let mut functions = p.parse_library()?; // parse imported library 
        lib_functions.append(&mut functions);
    }
    let program = Program {main_program,functions:lib_functions};
    println!("{:?}", program); 
    Ok(())
}
