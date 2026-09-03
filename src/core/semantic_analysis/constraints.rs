/* Being redesigned from scratch alongside the new HIR -- kept here for
   reference during the rewrite. */
/*
use crate::core::common::text_size::TextRange;
use crate::core::common::types::TypeId;
use crate::core::semantic_analysis::hir::LoopSource;

pub(crate) enum Provenance {
    TypeMismatch {
        span: TextRange,
    },
    IfBranchMismatch {
        then_span: TextRange,
        else_span: TextRange,
    },
    IfWithoutElse {
        span: TextRange,
    },
    BinaryOperandMismatch {
        lhs_span: TextRange,
        rhs_span: TextRange,
    },
    BinaryOperandNotNumeric {
        span: TextRange,
    },
    BinaryOperandNotBool {
        span: TextRange,
    },
    UnaryOperandMismatch {
        operator: String,
        span: TextRange,
    },
    BlockMissingTail {
        span: TextRange,
    },
    ReturnMissingValue {
        span: TextRange,
    },
    LoopBodyNotUnit {
        source: LoopSource,
        span: TextRange,
    },
}

pub(crate) enum Constraint {
    Equality {
        expected_id: TypeId,
        actual_id: TypeId,
        provenance: Provenance,
    },
}

*/
