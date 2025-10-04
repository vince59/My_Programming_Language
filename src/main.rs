// Vincent Pineau 04/10/2025
// My Programming Language
// A simple compilator for a simple language

use std::{env, fs, process};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let main_src = env::args().nth(1).unwrap_or_else(|| {
        eprintln!("Usage : mpl <path to main source code>");
        process::exit(1);
    });

    let content = fs::read_to_string(main_src)?;
    print!("{}", content);
    Ok(())
}
