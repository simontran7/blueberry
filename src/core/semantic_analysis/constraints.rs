/* Being redesigned from scratch alongside the new HIR -- kept here for
   reference during the rewrite. */
/*
use crate::core::common::span::Span;
use crate::core::common::types::TypeId;
use crate::core::semantic_analysis::hir::LoopSource;

pub(crate) enum Provenance {
    TypeMismatch {
        span: Span,
    },
    IfBranchMismatch {
        then_span: Span,
        else_span: Span,
    },
    IfWithoutElse {
        span: Span,
    },
    BinaryOperandMismatch {
        lhs_span: Span,
        rhs_span: Span,
    },
    BinaryOperandNotNumeric {
        span: Span,
    },
    BinaryOperandNotBool {
        span: Span,
    },
    UnaryOperandMismatch {
        operator: String,
        span: Span,
    },
    BlockMissingTail {
        span: Span,
    },
    ReturnMissingValue {
        span: Span,
    },
    LoopBodyNotUnit {
        source: LoopSource,
        span: Span,
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
