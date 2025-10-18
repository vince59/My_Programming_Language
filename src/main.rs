// Vincent Pineau 04/10/2025
// My Programming Language
// A simple compilator for a simple language

// Main entry point for MPL CLI
// All comments are in English per requirement.

mod lexer;
mod parser;
mod grammar;
mod codegen;
mod runner;

use clap::{Arg, ArgAction, ArgGroup, Command};
use codegen::CodeGenerator;
use lexer::Lexer;
use parser::{Parser, Program};
use std::{
    fs,
    path::{Path, PathBuf},
    process,
};

fn resolve_rel(base_file: &Path, rel: &str) -> PathBuf {
    // Resolve a relative path against the base file directory.
    let base_dir = base_file.parent().unwrap_or_else(|| Path::new("."));
    base_dir.join(rel)
}

fn file_stem_string(p: &Path) -> String {
    // Return file stem as String; fallback to "main" if none.
    match p.file_stem() {
        Some(s) => s.to_string_lossy().into_owned(),
        None => "main".to_string(),
    }
}

fn build_cli() -> Command {
    Command::new("mpl")
        .about("MPL compiler/runner")
        .version("0.1.0")
        // Clear, English usage with mutually exclusive modes (help is auto by clap).
        .override_usage(
            "mpl (-c | -r | -rw) [OPTIONS] <INPUT>\n\
             mpl -c  <source.mpl> [-o <wasm_name>] [-a [wat_name]]\n\
             mpl -r  <source.mpl>\n\
             mpl -rw <wasm_name>",
        )
        // Modes (mutually exclusive). We also add explicit conflicts for clarity.
        .arg(
            Arg::new("compile")
                .short('c')
                .long("compile")
                .help("Compile the source to WebAssembly (WASM); optional WAT via -a")
                .action(ArgAction::SetTrue)
                .conflicts_with_all(["run", "runwasm"]),
        )
        .arg(
            Arg::new("run")
                .short('r')
                .long("run")
                .help("Compile the source and run it without writing files to disk")
                .action(ArgAction::SetTrue)
                .conflicts_with_all(["compile", "runwasm"]),
        )
        .arg(
            Arg::new("runwasm")
                .long("rw")
                .value_name("WASM")
                .help("Run an existing WebAssembly binary file")
                .num_args(1)
                .conflicts_with_all(["compile", "run"]),
        )
        // Options
        .arg(
            Arg::new("output")
                .short('o')
                .long("output")
                .value_name("WASM_OUT")
                .help("Force the output name for the WebAssembly file (used with -c)"),
        )
        .arg(
            Arg::new("wat")
                .short('a')
                .long("wat")
                .value_name("WAT_OUT")
                .help("Also produce a WAT file; if no value is provided, defaults to <source>.wat")
                // Allow -a with optional value: -a or -a out.wat
                .num_args(0..=1),
        )
        // Positional that may be required depending on the mode.
        .arg(
            Arg::new("input")
                .value_name("INPUT")
                .help("Input file: <source.mpl> for -c/-r; omitted for -rw")
                .required(false),
        )
        // Require that exactly one mode is chosen among compile/run/runwasm.
        // ArgGroup(required) enforces "at least one"; conflicts enforce "only one".
        .group(
            ArgGroup::new("mode")
                .args(["compile", "run", "runwasm"])
                .required(true),
        )
        .after_help(
"EXAMPLES:
  mpl -c main.mpl                 Compile to main.wasm
  mpl -c main.mpl -o out.wasm     Compile to out.wasm
  mpl -c main.mpl -a              Also emit main.wat
  mpl -c main.mpl -a dump.wat     Also emit dump.wat
  mpl -r main.mpl                 Compile in-memory and run (no files written)
  mpl -rw program.wasm            Run an existing WASM binary

RULES:
  -c, -r, -rw are mutually exclusive (pick exactly one).",
        )
}


fn main() {
    if let Err(e) = real_main() {
        // Use Display, not Debug
        eprintln!("{e}");
        std::process::exit(1);
    }
}

fn real_main() -> Result<(), Box<dyn std::error::Error>> {
    let matches = build_cli().get_matches();

    let compile_mode = matches.get_flag("compile");
    let run_mode = matches.get_flag("run");
    let runwasm_arg = matches.get_one::<String>("runwasm").cloned();

    let input_path: Option<PathBuf> = matches
        .get_one::<String>("input")
        .map(|s| PathBuf::from(s));

    // Validate mode-specific requirements
    if compile_mode || run_mode {
        if input_path.is_none() {
            eprintln!(
                "Error: missing <source.mpl>.\n\nUSAGE:\n  mpl -c <source.mpl> [-o <wasm_name>] [-a [wat_name]]\n  mpl -r <source.mpl>"
            );
            process::exit(2);
        }
    }
    if runwasm_arg.is_some() && input_path.is_some() {
        eprintln!("Error: -rw does not use <INPUT> positional.\n\nUSAGE:\n  mpl -rw <wasm_name>");
        process::exit(2);
    }

    // Dispatch per mode
    if compile_mode {
        // --- Compile to WASM (and optionally WAT), write files, do not run.
        let src_file = input_path.unwrap();
        let src_text = fs::read_to_string(&src_file)?;
        let lex = Lexer::new(&src_file, src_text);
        let mut parser = Parser::new(lex)?;
        let main_program = parser.parse_main_program()?;
        let mut lib_functions = Vec::new();

        // Parse imports
        for import in &main_program.imports {
            let import_src_file = resolve_rel(&src_file, import);
            let import_src = fs::read_to_string(&import_src_file)?;
            let lex = Lexer::new(import_src_file, import_src);
            let mut p = Parser::new(lex)?;
            let mut functions = p.parse_library()?;
            lib_functions.append(&mut functions);
        }
        let program = Program {
            main_program,
            functions: lib_functions,
        };

        // Generate WASM bytes
        let prog_name = file_stem_string(&src_file);
        let mut generator = CodeGenerator::new();
        let wasm = generator.generate_wasm(prog_name, &program)?;

        // Determine WASM output path
        let wasm_out = if let Some(o) = matches.get_one::<String>("output") {
            PathBuf::from(o)
        } else {
            src_file.with_extension("wasm")
        };
        fs::write(&wasm_out, &wasm)?;

        // Optionally produce WAT
        if matches.contains_id("wat") {
            // If a value is provided to -a, use it; else default to <source>.wat
            let wat_out = if let Some(name) = matches.get_one::<String>("wat") {
                PathBuf::from(name)
            } else {
                src_file.with_extension("wat")
            };
            let wat = wasmprinter::print_bytes(&wasm).expect("WAT print failed");
            fs::write(&wat_out, wat)?;
        }

        // Optional: print program debug (as in your original main)
        // println!("{:?}", program);

        Ok(())
    } else if run_mode {
        // --- Compile in-memory and run without writing files.
        let src_file = input_path.unwrap();
        let src_text = fs::read_to_string(&src_file)?;
        let lex = Lexer::new(&src_file, src_text);
        let mut parser = Parser::new(lex)?;
        let main_program = parser.parse_main_program()?;
        let mut lib_functions = Vec::new();

        // Parse imports
        for import in &main_program.imports {
            let import_src_file = resolve_rel(&src_file, import);
            let import_src = fs::read_to_string(&import_src_file)?;
            let lex = Lexer::new(import_src_file, import_src);
            let mut p = Parser::new(lex)?;
            let mut functions = p.parse_library()?;
            lib_functions.append(&mut functions);
        }
        let program = Program {
            main_program,
            functions: lib_functions,
        };

        // Generate WASM bytes
        let prog_name = file_stem_string(&src_file);
        let mut generator = CodeGenerator::new();
        let wasm = generator.generate_wasm(prog_name, &program)?;

        // Run directly from memory (no disk write).
        // NOTE: ensure runner exposes `run_wasm_bytes(&[u8]) -> Result<(), E>`.
        runner::run_wasm_bytes(&wasm)?;

        Ok(())
    } else if let Some(wasm_path) = runwasm_arg {
        // --- Run an existing WASM file from disk.
        runner::run_wasm_file(&wasm_path)?;
        Ok(())
    } else {
        // Should not happen due to ArgGroup(required=true), but keep a safe fallback.
        eprintln!("Error: one mode must be selected (-c | -r | -rw).");
        process::exit(2);
    }
}
