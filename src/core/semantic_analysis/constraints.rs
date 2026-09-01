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
        then_span: TextRange,
    },
    BinaryOperandMismatch {
        lhs_span: TextRange,
        rhs_span: TextRange,
    },
    BinaryOperandNotNumeric {
        operand_span: TextRange,
    },
    BinaryOperandNotBool {
        operand_span: TextRange,
    },
    UnaryOperandMismatch {
        operator: String,
        operand_span: TextRange,
    },
    BlockMissingTail {
        block_span: TextRange,
    },
    ReturnMissingValue {
        return_span: TextRange,
    },
    LoopBodyNotUnit {
        source: LoopSource,
        body_span: TextRange,
    },
}

pub(crate) enum Constraint {
    Equality {
        expected_id: TypeId,
        actual_id: TypeId,
        provenance: Provenance,
    },
}
