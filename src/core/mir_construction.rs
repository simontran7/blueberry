pub(crate) mod cfg_builder;
pub(crate) mod lowerer;
pub(crate) mod mir;
pub(crate) mod mir_dumper;

use std::sync::Arc;

use crate::core::mir_construction::lowerer::MirLowerer;
use crate::core::mir_construction::mir::MirBody;
use crate::core::semantic_analysis::ids::keys::{ConstantKey, FunctionKey};
use crate::core::semantic_analysis::{
    constant_body_of, constant_scopes_of, constant_signature_type_of, constant_types_of,
    function_body_of, function_scopes_of, function_signature_type_of, function_types_of,
};

#[salsa::tracked]
pub(crate) fn function_mir_of<'db>(
    db: &'db dyn crate::Db,
    key: FunctionKey<'db>,
) -> Option<Arc<MirBody<'db>>> {
    let signature_type = *function_signature_type_of(db, key);
    let types = function_types_of(db, key);
    if signature_type.is_poisoned(db) || types.has_errors(db) {
        return None;
    }
    let body = function_body_of(db, key);
    let scopes = function_scopes_of(db, key);
    let lowerer = MirLowerer::new(db, body, scopes, types);
    Some(Arc::new(lowerer.lower_function(signature_type)))
}

#[salsa::tracked]
pub(crate) fn constant_mir_of<'db>(
    db: &'db dyn crate::Db,
    key: ConstantKey<'db>,
) -> Option<Arc<MirBody<'db>>> {
    let declared_type = *constant_signature_type_of(db, key);
    let types = constant_types_of(db, key);
    if declared_type.is_poisoned(db) || types.has_errors(db) {
        return None;
    }
    let body = constant_body_of(db, key);
    let scopes = constant_scopes_of(db, key);
    let lowerer = MirLowerer::new(db, body, scopes, types);
    Some(Arc::new(lowerer.lower_constant(declared_type)))
}
