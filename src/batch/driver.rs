use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;
use std::time::Instant;

use crate::batch::arg_parser::EmitKind;
use crate::batch::diagnostic_render::render_diagnostic;
use crate::core::common::diagnostic::DiagnosticAccumulator;
use crate::core::db::BlueberryDatabase;
use crate::core::file_scanning::SourceFile;
use crate::core::lexical_analysis::token_stream_dumper::TokenDumper;
use crate::core::lexical_analysis::tokens_of;
use crate::core::semantic_analysis::hir_dumper::HirDumper;
use crate::core::semantic_analysis::{body_hir_of, definition_keys_of, hir_of};
use crate::core::syntactic_analysis::cst_dumper::CstDumper;
use crate::core::syntactic_analysis::cst_of;

pub fn build(path: PathBuf, emit: &HashSet<EmitKind>) {
    todo!();
}

pub fn run(path: PathBuf, emit: &HashSet<EmitKind>) {
    todo!()
}

pub fn check(path: PathBuf, emit: &HashSet<EmitKind>) {
    compile(path, emit);
}

fn compile(path: PathBuf, emit: &HashSet<EmitKind>) {
    // intialize
    let start = Instant::now();
    let file_stem = path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| path.to_string_lossy().into_owned());
    let db = BlueberryDatabase::default();

    // ---- Stage 0: Scanning ----
    let file = match fs::read_to_string(&path) {
        Ok(contents) => SourceFile::new(&db, path, contents),
        Err(error) => {
            eprintln!("Error reading file {}: {}", path.to_string_lossy().to_string(), error);
            std::process::exit(1);
        }
    };

    // ---- Stage 1: Lexical Analysis ----
    tokens_of(&db, file);
    let lexical_diagnostics = tokens_of::accumulated::<DiagnosticAccumulator>(&db, file);
    if !lexical_diagnostics.is_empty() {
        for lexical_diagnostic in lexical_diagnostics {
            render_diagnostic(&lexical_diagnostic.0, &file_stem, file.contents(&db));
        }
        return;
    }
    if emit.contains(&EmitKind::Tokens) {
        let dumper = TokenDumper::new(file.contents(&db), tokens_of(&db, file).clone());
        println!("{}", dumper.dump());
    }

    // ---- Stage 2: Syntactic Analysis ----
    cst_of(&db, file);
    let syntactic_diagnostics = cst_of::accumulated::<DiagnosticAccumulator>(&db, file);
    if !syntactic_diagnostics.is_empty() {
        for syntactic_diagnostic in syntactic_diagnostics {
            render_diagnostic(&syntactic_diagnostic.0, &file_stem, file.contents(&db));
        }
        return;
    }
    if emit.contains(&EmitKind::Cst) {
        let mut dumper = CstDumper::new(cst_of(&db, file));
        println!("{}", dumper.dump());
    }

    // ---- Stage 3: Semantic Analysis ----
    hir_of(&db, file);
    let semantic_diagnostics = hir_of::accumulated::<DiagnosticAccumulator>(&db, file);
    if !semantic_diagnostics.is_empty() {
        for semantic_diagnostic in semantic_diagnostics {
            render_diagnostic(&semantic_diagnostic.0, &file_stem, file.contents(&db));
        }
        return;
    }
    if emit.contains(&EmitKind::Hir) {
        for key in definition_keys_of(&db, file) {
            let (hir, _, ctx) = body_hir_of(&db, file, key.clone());
            let dumper = HirDumper::new(hir, ctx);
            println!("{}", dumper.dump().unwrap());
        }
    }

    // print compile time
    println!(
        "Compiled {file_stem} in {:.2}s",
        start.elapsed().as_secs_f64()
    );
}

