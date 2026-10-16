pub(in crate::core) mod cfg_builder;
pub(in crate::core) mod lowerer;
pub(in crate::core) mod mir;
pub(in crate::core) mod mir_dumper;

use std::sync::Arc;

use crate::core::mir_construction::lowerer::MirLowerer;
use crate::core::mir_construction::mir::MirBody;
use crate::core::semantic_analysis::name_resolution::definition_list::Definition;
use crate::core::semantic_analysis::{body_of, signature_type_of, types_of};

#[salsa::tracked]
pub(in crate::core) fn mir_of<'db>(
    db: &'db dyn crate::Db,
    definition: Definition<'db>,
) -> Option<Arc<MirBody<'db>>> {
    let signature_type = *signature_type_of(db, definition);
    let types = types_of(db, definition);
    if signature_type.is_poisoned(db) || types.has_errors(db) {
        return None;
    }
    let body = body_of(db, definition);
    let lowerer = MirLowerer::new(db, body, types);
    Some(Arc::new(match definition {
        Definition::Function(_) => lowerer.lower_function(signature_type),
        Definition::Constant(_) => lowerer.lower_constant(signature_type),
    }))
}
