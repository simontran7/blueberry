use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::Instant;

use crate::batch::arg_parser::EmitKind;
use crate::batch::diagnostic_render::render_diagnostic;
use crate::core::program::{BlueberryDatabase, DiagnosticDescription, Program};

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
    let file_stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string_lossy().into_owned());
    let db = BlueberryDatabase::default();

    // ---- Stage 0: Scanning ----
    let root = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    if let Err(error) = Program::load_modules(&db, root) {
        eprintln!(
            "Error reading directory {}: {}",
            root.to_string_lossy(),
            error
        );
        std::process::exit(1);
    }
    let program = Program::new(&db);
    let Some(file) = program.file(Path::new(path.file_name().unwrap())) else {
        eprintln!("Error reading file {}", path.to_string_lossy());
        std::process::exit(1);
    };
    let source = program.contents(file);
    let report = |diagnostics: Vec<DiagnosticDescription>| {
        let has_diagnostics = !diagnostics.is_empty();
        for diagnostic in diagnostics {
            render_diagnostic(diagnostic, &file_stem, source);
        }
        has_diagnostics
    };

    // ---- Stage 1: Lexical Program ----
    if report(program.lexical_diagnostics(file)) {
        return;
    }
    if emit.contains(&EmitKind::Tokens) {
        println!("{}", program.dump_tokens(file));
    }

    // ---- Stage 2: Syntactic Program ----
    if report(program.syntax_diagnostics(file)) {
        return;
    }
    if emit.contains(&EmitKind::Cst) {
        println!("{}", program.dump_cst(file));
    }

    // ---- Stage 3: Semantic Program ----
    let mut semantic_diagnostics = program.semantic_diagnostics(file);
    semantic_diagnostics.sort_by_key(|diagnostic| diagnostic.span.start());
    if emit.contains(&EmitKind::Hir) {
        print!("{}", program.dump_hir(file));
    }
    if report(semantic_diagnostics) {
        return;
    }

    // ---- Stage 4: MIR Construction ----
    if emit.contains(&EmitKind::Mir) {
        print!("{}", program.dump_mir(file));
    }

    // print compile time
    println!(
        "Compiled {file_stem} in {:.2}s",
        start.elapsed().as_secs_f64()
    );
}
