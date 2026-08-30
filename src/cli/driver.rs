use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;
use std::time::Instant;

use crate::cli::arg_parser::EmitKind;
use crate::lexical_analysis::token_stream_dumper::TokenDumper;
use crate::lexical_analysis::tokenizer::Tokenizer;
use crate::syntactic_analysis::cst_builder::CstBuilder;
use crate::syntactic_analysis::cst_dumper::CstDumper;
use crate::syntactic_analysis::parser::Parser;

pub fn build(path: PathBuf, emit: &HashSet<EmitKind>) {
    compile(path, emit);
}

pub fn run(path: PathBuf, emit: &HashSet<EmitKind>) {
    compile(path, emit);
}

pub fn check(path: PathBuf, emit: &HashSet<EmitKind>) {
    compile(path, emit);
}

fn compile(path: PathBuf, emit: &HashSet<EmitKind>) {
    let start = Instant::now();
    let file_stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string_lossy().into_owned());
    let display_path = path.to_string_lossy().to_string();

    // step 1: serialize
    let source = match fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error reading file {display_path}: {e}");
            std::process::exit(1);
        }
    };

    // step 2: tokenize
    let tokens = Tokenizer::new(&source).tokenize();
    if emit.contains(&EmitKind::Tokens) {
        println!("{}", TokenDumper::new(&source, tokens).dump());
        return;
    }

    // step 3: parse
    let (events, diagnostics) = Parser::new(&tokens).parse();
    let (cst, diagnostics) = CstBuilder::new(&source, &tokens, events, diagnostics).build();
    if emit.contains(&EmitKind::Cst) {
        println!("{}", CstDumper::new(&cst).dump());
        for diagnostic in &diagnostics {
            eprintln!("{:?}", diagnostic);
        }
        return;
    }

    println!(
        "Compiled {file_stem} in {:.2}s",
        start.elapsed().as_secs_f64()
    );
}
