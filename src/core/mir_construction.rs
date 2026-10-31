pub(crate) mod cfg_builder;
pub(crate) mod lowerer;
pub(crate) mod mir;
pub(crate) mod mir_dumper;
pub(crate) mod ssa_variables;

use std::sync::Arc;

use crate::core::mir_construction::lowerer::MirLowerer;
use crate::core::mir_construction::mir::MirBody;
use crate::core::semantic_analysis::name_resolution::definition_list::Definition;
use crate::core::semantic_analysis::{body, signature};

#[salsa::tracked]
pub(crate) fn mir_of<'db>(
    db: &'db dyn crate::Db,
    definition: Definition<'db>,
) -> Option<Arc<MirBody<'db>>> {
    let signature_type = *signature::type_of(db, definition);
    let semantics = body::semantics_of(db, definition);
    if signature_type.is_poisoned(db) || semantics.has_errors(db) {
        return None;
    }
    let body = body::hir_of(db, definition);
    let lowerer = MirLowerer::new(db, body, semantics);
    Some(Arc::new(match definition {
        Definition::Function(_) => lowerer.lower_function(signature_type),
        Definition::Constant(_) => lowerer.lower_constant(signature_type),
    }))
}
