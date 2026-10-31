use std::path::Path;
use std::time::Instant;

use crate::batch::diagnostic_render::render_diagnostic;
use crate::cli::arg_parser::{EmitKind, Flags};
use crate::core::compiler::{BlueberryDatabase, Compiler, Diagnostic, SourceFileKey};

pub fn build(_flags: &Flags) {
    todo!();
}

pub fn run(_flags: &Flags) {
    todo!()
}

pub fn check(flags: &Flags) {
    compile(flags);
}

fn compile(flags: &Flags) {
    // intialize
    let start = Instant::now();
    let file_stem = flags.path.file_stem().unwrap().to_string_lossy();
    let db = BlueberryDatabase::default();

    // ---- Scanning ----
    let compiler = BatchCompiler::load(&db, flags);

    // ---- Lexical Analysis ----
    compiler.emit_tokens_if_requested();
    if compiler.report_lexical_errors() {
        std::process::exit(1);
    }

    // ---- Syntactic Analysis ----
    compiler.emit_cst_if_requested();
    if compiler.report_syntax_errors() {
        std::process::exit(1);
    }

    // ---- Semantic Analysis ----
    compiler.emit_hir_if_requested();
    if compiler.report_semantic_errors() {
        std::process::exit(1);
    }

    // ---- MIR Construction ----
    compiler.emit_mir_if_requested();

    // print compile time
    println!(
        "Compiled {file_stem} in {:.2}s",
        start.elapsed().as_secs_f64()
    );
}

/// A single run of the compiler from the command line.
struct BatchCompiler<'db> {
    compiler: Compiler<'db>,
    emit: &'db [EmitKind],
    /// the compiled file
    entry: SourceFileKey,
    /// the compiled file, then every file it imports, each named by its path relative to the root
    files: Vec<(SourceFileKey, String)>,
}

impl<'db> BatchCompiler<'db> {
    fn load(db: &'db BlueberryDatabase, flags: &'db Flags) -> Self {
        let path = flags.path.as_path();
        let root = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        if let Err(error) = Compiler::load_modules(db, root) {
            eprintln!(
                "Error reading directory {}: {}",
                root.to_string_lossy(),
                error
            );
            std::process::exit(1);
        }
        let compiler = Compiler::new(db);
        let Some(entry) = compiler.file(Path::new(path.file_name().unwrap())) else {
            eprintln!("Error reading file {}", path.to_string_lossy());
            std::process::exit(1);
        };
        let files = compiler
            .files_reachable_from(entry)
            .into_iter()
            .map(|file| {
                let path = compiler.path(file);
                let name = path.strip_prefix(root).unwrap_or(path).with_extension("");
                (file, name.to_string_lossy().into_owned())
            })
            .collect();
        Self {
            compiler,
            emit: &flags.emit,
            entry,
            files,
        }
    }

    fn report_lexical_errors(&self) -> bool {
        self.report(|file| self.compiler.lexical_diagnostics(file))
    }

    fn report_syntax_errors(&self) -> bool {
        self.report(|file| self.compiler.syntax_diagnostics(file))
    }

    fn report_semantic_errors(&self) -> bool {
        self.report(|file| {
            let mut diagnostics = self.compiler.semantic_diagnostics(file);
            diagnostics.sort_by_key(|diagnostic| diagnostic.range.start());
            diagnostics
        })
    }

    /// Renders a stage's diagnostics in every file, returning whether there were any.
    fn report(&self, diagnostics_of: impl Fn(SourceFileKey) -> Vec<Diagnostic>) -> bool {
        let mut has_diagnostics = false;
        for (file, name) in &self.files {
            for diagnostic in diagnostics_of(*file) {
                render_diagnostic(diagnostic, name, self.compiler.contents(*file));
                has_diagnostics = true;
            }
        }
        has_diagnostics
    }

    fn emit_tokens_if_requested(&self) {
        if self.emit.contains(&EmitKind::Tokens) {
            println!("{}", self.compiler.dump_tokens(self.entry));
        }
    }

    fn emit_cst_if_requested(&self) {
        if self.emit.contains(&EmitKind::Cst) {
            println!("{}", self.compiler.dump_cst(self.entry));
        }
    }

    fn emit_hir_if_requested(&self) {
        if self.emit.contains(&EmitKind::Hir) {
            print!("{}", self.compiler.dump_hir(self.entry));
        }
    }

    fn emit_mir_if_requested(&self) {
        if self.emit.contains(&EmitKind::Mir) {
            print!("{}", self.compiler.dump_mir(self.entry));
        }
    }
}
