use std::path::Path;
use std::time::Instant;

use crate::batch::arg_parser::EmitKind;
use crate::batch::diagnostic_render::render_diagnostic;
use crate::core::compiler::{BlueberryDatabase, Compiler, DiagnosticDescription, SourceFileKey};

pub fn build(_path: &Path, _emit: &[EmitKind]) {
    todo!();
}

pub fn run(_path: &Path, _emit: &[EmitKind]) {
    todo!()
}

pub fn check(path: &Path, emit: &[EmitKind]) {
    compile(path, emit);
}

fn compile(path: &Path, emit: &[EmitKind]) {
    // intialize
    let start = Instant::now();
    let file_stem = path.file_stem().map_or_else(
        || path.to_string_lossy().into_owned(),
        |s| s.to_string_lossy().into_owned(),
    );
    let db = BlueberryDatabase::default();

    // ---- Stage 0: Scanning ----
    let root = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    if let Err(error) = Compiler::load_modules(&db, root) {
        eprintln!(
            "Error reading directory {}: {}",
            root.to_string_lossy(),
            error
        );
        std::process::exit(1);
    }
    let compiler = Compiler::new(&db);
    let Some(file) = compiler.file(Path::new(path.file_name().unwrap())) else {
        eprintln!("Error reading file {}", path.to_string_lossy());
        std::process::exit(1);
    };
    // the compiled file, then every file it imports, each named by its path relative to the root
    let files: Vec<(SourceFileKey, String)> = compiler
        .files_reachable_from(file)
        .into_iter()
        .map(|file| {
            let path = compiler.path(file);
            let name = path.strip_prefix(root).unwrap_or(path).with_extension("");
            (file, name.to_string_lossy().into_owned())
        })
        .collect();
    // reports a stage's diagnostics in every file, returning whether there were any
    let report = |diagnostics_of: &dyn Fn(SourceFileKey) -> Vec<DiagnosticDescription>| {
        let mut has_diagnostics = false;
        for (file, name) in &files {
            for diagnostic in diagnostics_of(*file) {
                render_diagnostic(diagnostic, name, compiler.contents(*file));
                has_diagnostics = true;
            }
        }
        has_diagnostics
    };

    // ---- Stage 1: Lexical Program ----
    if report(&|file| compiler.lexical_diagnostics(file)) {
        return;
    }
    if emit.contains(&EmitKind::Tokens) {
        println!("{}", compiler.dump_tokens(file));
    }

    // ---- Stage 2: Syntactic Program ----
    if report(&|file| compiler.syntax_diagnostics(file)) {
        return;
    }
    if emit.contains(&EmitKind::Cst) {
        println!("{}", compiler.dump_cst(file));
    }

    // ---- Stage 3: Semantic Program ----
    if emit.contains(&EmitKind::Hir) {
        print!("{}", compiler.dump_hir(file));
    }
    if report(&|checked| {
        let mut diagnostics = compiler.semantic_diagnostics(checked);
        // only the compiled file's `main` is the program's entry point
        if checked == file {
            diagnostics.extend(compiler.entry_point_diagnostics(checked));
        }
        diagnostics.sort_by_key(|diagnostic| diagnostic.span.start());
        diagnostics
    }) {
        return;
    }

    // ---- Stage 4: MIR Construction ----
    if emit.contains(&EmitKind::Mir) {
        print!("{}", compiler.dump_mir(file));
    }

    // print compile time
    println!(
        "Compiled {file_stem} in {:.2}s",
        start.elapsed().as_secs_f64()
    );
}
