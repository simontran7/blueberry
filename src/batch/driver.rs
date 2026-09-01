//! Consumer: the batch compiler. Implements batch-specific policy on top of
//! the core compiler's producer queries -- stop before semantic analysis if
//! lexing/parsing already found errors, and print diagnostics to stderr.
//! A future `lsp` consumer would sit alongside this one, sharing the exact
//! same producer queries, with the opposite policy: never stop early (an
//! editor wants every diagnostic it can get while the user is mid-edit),
//! and publish diagnostics over the wire instead of printing them.

use std::collections::HashSet;
use std::path::PathBuf;
use std::time::Instant;

use crate::batch::arg_parser::EmitKind;
use crate::core::common::diagnostic::DiagnosticAccumulator;
use crate::core::db::BlueberryDatabase;
use crate::core::file_scanning::{source_file_of, SourceFile};
use crate::core::lexical_analysis::token_stream_dumper::TokenDumper;
use crate::core::lexical_analysis::tokens_of;
use crate::core::semantic_analysis::hir_dumper::HirDumper;
use crate::core::semantic_analysis::{body_hir_of, definition_keys_of, full_diagnostics_of};
use crate::core::syntactic_analysis::cst_dumper::CstDumper;
use crate::core::syntactic_analysis::cst_of;

pub fn build(path: PathBuf, emit: &HashSet<EmitKind>) {
    todo!()
}

pub fn run(path: PathBuf, emit: &HashSet<EmitKind>) {
    todo!()
}

pub fn check(path: PathBuf, emit: &HashSet<EmitKind>) {
    batch_compile(path, emit);
}

fn batch_compile(path: PathBuf, emit: &HashSet<EmitKind>) {
    // intialize
    let start = Instant::now();
    let file_stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string_lossy().into_owned());
    let db = BlueberryDatabase::default();

    // scan the file
    let display_path = path.to_string_lossy().to_string();
    let file = match source_file_of(&db, path) {
        Ok(file) => file,
        Err(error) => {
            eprintln!("Error reading file {display_path}: {error}");
            std::process::exit(1);
        }
    };

    // dump IRs
    if emit.contains(&EmitKind::Tokens) {
        let tokens = tokens_of(&db, file);

        let dumper = TokenDumper::new(file.contents(&db), tokens.clone());
        println!("{}", dumper.dump());

        return;
    }
    if emit.contains(&EmitKind::Cst) {
        let cst = cst_of(&db, file);

        let mut dumper = CstDumper::new(cst);
        println!("{}", dumper.dump());

        report_diagnostics(&db, file, &file_stem);

        return;
    }
    if emit.contains(&EmitKind::Hir) {
        let keys = definition_keys_of(&db, file);

        for key in keys {
            let (hir, ctx) = body_hir_of(&db, file, key.clone());
            let dumper = HirDumper::new(hir, ctx);
            println!("{}", dumper.dump().unwrap());
        }

        report_diagnostics(&db, file, &file_stem);

        return;
    }

    // call the main query
    report_diagnostics(&db, file, &file_stem);

    // print compile time
    println!(
        "Compiled {file_stem} in {:.2}s",
        start.elapsed().as_secs_f64()
    );
}

fn report_diagnostics(db: &BlueberryDatabase, file: SourceFile, file_stem: &str) {
    cst_of(db, file);
    let lex_and_parse_diagnostics = cst_of::accumulated::<DiagnosticAccumulator>(db, file);
    if !lex_and_parse_diagnostics.is_empty() {
        for diagnostic in lex_and_parse_diagnostics {
            diagnostic.0.render(file_stem, file.contents(db));
        }
        return;
    }

    full_diagnostics_of(db, file);
    for diagnostic in full_diagnostics_of::accumulated::<DiagnosticAccumulator>(db, file) {
        diagnostic.0.render(file_stem, file.contents(db));
    }
}
