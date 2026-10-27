use std::io;
use std::path::Path as FilePath;

pub(crate) use crate::core::common::diagnostic::{DiagnosticDescription, LabelSeverity};
pub(crate) use crate::core::common::span::{Span, TextSize};
pub(crate) use crate::core::db::BlueberryDatabase;
pub(crate) use crate::core::source_file_key::SourceFileKey;

use crate::core::common::diagnostic::{Diagnostic, DiagnosticAccumulator};
use crate::core::lexical_analysis::token_stream_dumper::TokenDumper;
use crate::core::lexical_analysis::tokens_of;
use crate::core::mir_construction::mir_dumper::MirBodyDumper;
use crate::core::module_map::{ModuleMap, ModulePath};
use crate::core::semantic_analysis::ast_lowering::dumper::HirBodyDumper;
use crate::core::semantic_analysis::ast_lowering::nodes::ExpressionHandle;
use crate::core::semantic_analysis::name_resolution::definition_list::Definition;
use crate::core::semantic_analysis::name_resolution::resolver::Resolution;
use crate::core::semantic_analysis::{
    all_definitions_of, body_source_map_of, definition_at, definition_diagnostics_of,
    entry_point_diagnostics_of, import_diagnostics_of, imports_of, module_file_of, types_of,
};
use crate::core::syntactic_analysis::cst::cst_dumper::CstDumper;
use crate::core::syntactic_analysis::cst_of;

pub(crate) struct Compiler<'db> {
    db: &'db dyn crate::Db,
}

impl<'db> Compiler<'db> {
    pub(crate) fn new(db: &'db dyn crate::Db) -> Self {
        Self { db }
    }

    pub(crate) fn load_modules(db: &dyn crate::Db, root: &FilePath) -> io::Result<()> {
        ModuleMap::load(db, root).map(|_| ())
    }

    pub(crate) fn file(&self, relative_path: &FilePath) -> Option<SourceFileKey> {
        let modules = ModuleMap::try_get(self.db)?;
        let module_path = ModulePath::from_file_path(relative_path);
        modules.files(self.db).get(&module_path).copied()
    }

    /// Returns `file`, then every file it imports, directly or not, each once.
    pub(crate) fn files_reachable_from(&self, file: SourceFileKey) -> Vec<SourceFileKey> {
        let mut files = vec![file];
        let mut index = 0;
        while let Some(&current) = files.get(index) {
            index += 1;
            for &path in imports_of(self.db, current).paths() {
                if let Some(imported) = *module_file_of(self.db, path)
                    && !files.contains(&imported)
                {
                    files.push(imported);
                }
            }
        }
        files
    }

    pub(crate) fn path(&self, file: SourceFileKey) -> &'db FilePath {
        file.path(self.db)
    }

    pub(crate) fn contents(&self, file: SourceFileKey) -> &'db str {
        file.contents(self.db)
    }

    pub(crate) fn lexical_diagnostics(&self, file: SourceFileKey) -> Vec<DiagnosticDescription> {
        tokens_of(self.db, file);
        tokens_of::accumulated::<DiagnosticAccumulator>(self.db, file)
            .into_iter()
            .map(|accumulated| accumulated.0.describe())
            .collect()
    }

    pub(crate) fn syntax_diagnostics(&self, file: SourceFileKey) -> Vec<DiagnosticDescription> {
        cst_of(self.db, file);
        cst_of::accumulated::<DiagnosticAccumulator>(self.db, file)
            .into_iter()
            .map(|accumulated| accumulated.0.describe())
            .collect()
    }

    pub(crate) fn semantic_diagnostics(&self, file: SourceFileKey) -> Vec<DiagnosticDescription> {
        let definition_diagnostics =
            all_definitions_of(self.db, file)
                .into_iter()
                .flat_map(|definition| {
                    definition_diagnostics_of(self.db, definition)
                        .iter()
                        .cloned()
                });
        import_diagnostics_of(self.db, file)
            .into_iter()
            .chain(definition_diagnostics)
            .map(|diagnostic| Diagnostic::Semantic(diagnostic).describe())
            .collect()
    }

    /// Returns the errors about `main`, for a program compiled from `file`.
    pub(crate) fn entry_point_diagnostics(
        &self,
        file: SourceFileKey,
    ) -> Vec<DiagnosticDescription> {
        entry_point_diagnostics_of(self.db, file)
            .into_iter()
            .map(|diagnostic| Diagnostic::Semantic(diagnostic).describe())
            .collect()
    }

    pub(crate) fn type_at(&self, file: SourceFileKey, offset: TextSize) -> Option<String> {
        let (definition, expression_handle) = self.expression_at(file, offset)?;
        let ty = types_of(self.db, definition)[expression_handle];
        Some(ty.display(self.db).to_string())
    }

    pub(crate) fn goto_definition(
        &self,
        file: SourceFileKey,
        offset: TextSize,
    ) -> Option<(SourceFileKey, Span)> {
        let (definition, expression_handle) = self.expression_at(file, offset)?;
        match types_of(self.db, definition).path_resolution(expression_handle)? {
            Resolution::Local(binding_handle) => {
                let span = body_source_map_of(self.db, definition)
                    .local_binding_span(binding_handle, definition.anchor(self.db));
                Some((file, span))
            }
            Resolution::Function(key) => {
                let span = Definition::Function(key).span(self.db);
                Some((key.file(self.db), span))
            }
            Resolution::Constant(key) => {
                let span = Definition::Constant(key).span(self.db);
                Some((key.file(self.db), span))
            }
            Resolution::Module(module_file) => {
                Some((module_file, Span::new(TextSize::new(0), TextSize::new(0))))
            }
            Resolution::Error => None,
        }
    }

    pub(crate) fn dump_tokens(&self, file: SourceFileKey) -> String {
        TokenDumper::new(file.contents(self.db), tokens_of(self.db, file).clone()).dump()
    }

    pub(crate) fn dump_cst(&self, file: SourceFileKey) -> String {
        CstDumper::new(cst_of(self.db, file)).dump()
    }

    pub(crate) fn dump_hir(&self, file: SourceFileKey) -> String {
        HirBodyDumper::dump_file_with_types(self.db, file)
    }

    pub(crate) fn dump_mir(&self, file: SourceFileKey) -> String {
        MirBodyDumper::dump_file(self.db, file).unwrap()
    }

    fn expression_at(
        &self,
        file: SourceFileKey,
        offset: TextSize,
    ) -> Option<(Definition<'db>, ExpressionHandle)> {
        let definition = definition_at(self.db, file, offset)?;
        let relative_offset = offset - definition.anchor(self.db);
        let expression_handle =
            body_source_map_of(self.db, definition).expression_at(relative_offset)?;
        Some((definition, expression_handle))
    }
}
