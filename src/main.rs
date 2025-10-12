// Vincent Pineau 04/10/2025
// My Programming Language
// A simple compilator for a simple language

mod lexer;
mod parser;
mod grammar;
mod codegen;
mod runner;

use std::{env, fs, path::{Path, PathBuf}, process};
use lexer::Lexer;
use parser::{Parser, Program};
use codegen::CodeGenerator;

fn resolve_rel(base_file: &Path, rel: &str) -> PathBuf {
    let base_dir = base_file.parent().unwrap_or_else(|| Path::new("."));
    base_dir.join(rel)
}

fn file_stem_string(p: &Path) -> String {
    match p.file_stem() {
        Some(s) => s.to_string_lossy().into_owned(), // gère UTF-8 non valide
        None => "main".to_string(),
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // read main source code given on the command line
    let main_src_file : PathBuf  = env::args().nth(1).map(PathBuf::from).unwrap_or_else(|| {
        eprintln!("Usage : mpl <main.mpl> [out.wat]");
        process::exit(1);
    });
    let wasm_path = env::args().nth(2);
    let wat_path = env::args().nth(3);
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
 
    // generate wasm binary code
    let prog_name = file_stem_string(&main_src_file); 
    let mut generator = CodeGenerator::new();
    let wasm = generator.generate_wasm(prog_name,&program);
    let default_wasm = main_src_file.with_extension("wasm");
    let wasm_file = wasm_path.unwrap_or_else(|| default_wasm.to_string_lossy().into_owned());
    std::fs::write(&wasm_file, &wasm)?;

    // generate wat code
    let wat = wasmprinter::print_bytes(&wasm).unwrap();
    let default_wat = main_src_file.with_extension("wat");
    let wat_file = wat_path.unwrap_or_else(|| default_wat.to_string_lossy().into_owned());
    fs::write(&wat_file, wat)?;
    
    // run the wasm code
    runner::run_wasm_file(&wasm_file)?;
    println!("{:?}", program); 
    Ok(())
}
