use crate::core::file_scanning::SourceFile;
use crate::core::semantic_analysis::semantic_analyzer::DefinitionKey;

#[salsa::tracked]
pub(crate) fn llvm_ir_of(db: &dyn crate::Db, file: SourceFile, key: DefinitionKey) -> String {
    String::new()
}
