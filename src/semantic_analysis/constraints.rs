use crate::common::span::Span;
use crate::common::types::TypeId;
use crate::front_end::semantic_analysis::hir::LoopSource;

pub(crate) enum Provenance {
    TypeMismatch {
        span: Span,
    },
    IfBranchMismatch {
        then_span: Span,
        else_span: Span,
    },
    IfWithoutElse {
        then_span: Span,
    },
    BinaryOperandMismatch {
        lhs_span: Span,
        rhs_span: Span,
    },
    BinaryOperandNotNumeric {
        operand_span: Span,
    },
    BinaryOperandNotBool {
        operand_span: Span,
    },
    UnaryOperandMismatch {
        operator: String,
        operand_span: Span,
    },
    BlockMissingTail {
        block_span: Span,
    },
    ReturnMissingValue {
        return_span: Span,
    },
    LoopBodyNotUnit {
        source: LoopSource,
        body_span: Span,
    },
}

pub(crate) enum Constraint {
    Equality {
        expected_id: TypeId,
        actual_id: TypeId,
        provenance: Provenance,
    },
}
