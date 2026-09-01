//! Pure, memoized, per-definition -- same query shape as `core`'s producers
//! -- but lives here, not in `core`, because nothing but `batch` actually
//! uses it: an `lsp` consumer has no use for LLVM IR (IDE features only need
//! semantic information -- hover, go-to-def, diagnostics -- never a
//! compiled artifact; real rust-analyzer never does codegen at all, only
//! rustc's separate batch pipeline does). `core` is for genuine, current
//! sharing, not code that merely *could* be shared someday.

use crate::core::file_scanning::SourceFile;
use crate::core::semantic_analysis::semantic_analyzer::DefinitionKey;

#[salsa::tracked]
pub(crate) fn llvm_ir_of(db: &dyn crate::Db, file: SourceFile, key: DefinitionKey) -> String {
    todo!()
}
