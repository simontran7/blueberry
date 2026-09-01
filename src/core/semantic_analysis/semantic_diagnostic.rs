use crate::core::common::diagnostic::{DiagnosticDescription, DiagnosticLabel, LabelSeverity};
use crate::core::common::text_size::TextRange;
use crate::core::semantic_analysis::hir::LoopSource;

#[derive(Debug, Clone)]
pub(crate) enum SemanticDiagnostic {
    TypeMismatch {
        expected: String,
        found: String,
        span: TextRange,
    },
    ArityMismatch {
        expected: usize,
        found: usize,
        call_span: TextRange,
        callee_span: TextRange,
        extra_argument_spans: Vec<TextRange>,
    },
    DuplicateDefinition {
        name: String,
        span: TextRange,
        previous_span: TextRange,
    },
    UnknownType {
        name: String,
        span: TextRange,
    },
    UnresolvedName {
        name: String,
        span: TextRange,
    },
    NotCallable {
        found: String,
        callee_span: TextRange,
        call_span: TextRange,
    },
    InvalidAssignTarget {
        span: TextRange,
    },
    IfBranchMismatch {
        then_ty: String,
        else_ty: String,
        then_span: TextRange,
        else_span: TextRange,
    },
    IfWithoutElse {
        found: String,
        then_span: TextRange,
    },
    BinaryOperandMismatch {
        lhs_ty: String,
        rhs_ty: String,
        lhs_span: TextRange,
        rhs_span: TextRange,
    },
    BinaryOperandNotNumeric {
        found: String,
        operand_span: TextRange,
    },
    BinaryOperandNotBool {
        expected: String,
        found: String,
        operand_span: TextRange,
    },
    UnaryOperandMismatch {
        operator: String,
        expected: String,
        found: String,
        operand_span: TextRange,
    },
    BlockMissingTail {
        expected: String,
        block_span: TextRange,
    },
    ReturnMissingValue {
        expected: String,
        return_span: TextRange,
    },
    ReturnOutsideFunction {
        span: TextRange,
    },
    NonConstantValue {
        span: TextRange,
    },
    CaptureInFunction {
        span: TextRange,
    },
    LoopBodyNotUnit {
        source: LoopSource,
        found: String,
        body_span: TextRange,
    },
    BreakOutsideLoop {
        span: TextRange,
    },
    ContinueOutsideLoop {
        span: TextRange,
    },
    BreakWithValueFromWhile {
        span: TextRange,
    },
    LetMissingTypeOrValue {
        span: TextRange,
    },
    InvalidIntegerLiteral {
        found: String,
        span: TextRange,
    },
}

fn label(span: TextRange, message: impl Into<String>, severity: LabelSeverity) -> DiagnosticLabel {
    DiagnosticLabel {
        span,
        message: Some(message.into()),
        severity,
    }
}

fn unlabeled(span: TextRange, severity: LabelSeverity) -> DiagnosticLabel {
    DiagnosticLabel {
        span,
        message: None,
        severity,
    }
}

impl SemanticDiagnostic {
    pub(crate) fn describe(&self) -> DiagnosticDescription {
        match self {
            Self::TypeMismatch {
                expected,
                found,
                span,
            } => DiagnosticDescription {
                code: "E0201",
                message: "mismatched types".to_string(),
                anchor: *span,
                labels: vec![label(
                    *span,
                    format!("expected `{expected}`, found `{found}`"),
                    LabelSeverity::Primary,
                )],
            },
            Self::ArityMismatch {
                expected,
                found,
                call_span,
                callee_span,
                extra_argument_spans,
            } => {
                let mut labels: Vec<DiagnosticLabel> = extra_argument_spans
                    .iter()
                    .enumerate()
                    .map(|(i, span)| {
                        label(
                            *span,
                            format!("unexpected argument #{}", expected + i + 1),
                            LabelSeverity::Primary,
                        )
                    })
                    .collect();
                labels.push(label(
                    *callee_span,
                    "function defined here",
                    LabelSeverity::Secondary,
                ));
                DiagnosticDescription {
                    code: "E0202",
                    message: format!(
                        "this function takes {} argument{} but {} {} supplied",
                        expected,
                        if *expected == 1 { "" } else { "s" },
                        found,
                        if *found == 1 { "was" } else { "were" },
                    ),
                    anchor: *call_span,
                    labels,
                }
            }
            Self::DuplicateDefinition {
                name,
                span,
                previous_span,
            } => DiagnosticDescription {
                code: "E0204",
                message: format!("the name `{name}` is defined multiple times"),
                anchor: *span,
                labels: vec![
                    label(
                        *span,
                        format!("`{name}` redefined here"),
                        LabelSeverity::Primary,
                    ),
                    label(
                        *previous_span,
                        format!("previous definition of `{name}` here"),
                        LabelSeverity::Secondary,
                    ),
                ],
            },
            Self::UnknownType { name, span } => DiagnosticDescription {
                code: "E0205",
                message: format!("cannot find type `{name}` in this scope"),
                anchor: *span,
                labels: vec![label(
                    *span,
                    "not found in this scope",
                    LabelSeverity::Primary,
                )],
            },
            Self::UnresolvedName { name, span } => DiagnosticDescription {
                code: "E0203",
                message: format!("cannot find value `{name}` in this scope"),
                anchor: *span,
                labels: vec![label(
                    *span,
                    "not found in this scope",
                    LabelSeverity::Primary,
                )],
            },
            Self::NotCallable {
                found,
                callee_span,
                call_span,
            } => DiagnosticDescription {
                code: "E0207",
                message: format!("expected function, found `{found}`"),
                anchor: *call_span,
                labels: vec![
                    label(
                        *callee_span,
                        format!("has type `{found}`"),
                        LabelSeverity::Secondary,
                    ),
                    label(
                        *call_span,
                        "call expression requires function",
                        LabelSeverity::Primary,
                    ),
                ],
            },
            Self::InvalidAssignTarget { span } => DiagnosticDescription {
                code: "E0216",
                message: "invalid left-hand side of assignment".to_string(),
                anchor: *span,
                labels: vec![label(
                    *span,
                    "cannot assign to this expression",
                    LabelSeverity::Primary,
                )],
            },
            Self::IfBranchMismatch {
                then_ty,
                else_ty,
                then_span,
                else_span,
            } => DiagnosticDescription {
                code: "E0209",
                message: "if and else branches have incompatible types".to_string(),
                anchor: *then_span,
                labels: vec![
                    label(
                        *then_span,
                        format!("then branch has type `{then_ty}`"),
                        LabelSeverity::Secondary,
                    ),
                    label(
                        *else_span,
                        format!("else branch has type `{else_ty}`"),
                        LabelSeverity::Primary,
                    ),
                ],
            },
            Self::IfWithoutElse { found, then_span } => DiagnosticDescription {
                code: "E0210",
                message: "if without else must evaluate to `()`".to_string(),
                anchor: *then_span,
                labels: vec![label(
                    *then_span,
                    format!("found type `{found}`, expected `()`"),
                    LabelSeverity::Primary,
                )],
            },
            Self::BinaryOperandMismatch {
                lhs_ty,
                rhs_ty,
                lhs_span,
                rhs_span,
            } => DiagnosticDescription {
                code: "E0211",
                message: "binary operation applied to mismatched types".to_string(),
                anchor: *lhs_span,
                labels: vec![
                    label(
                        *lhs_span,
                        format!("this has type `{lhs_ty}`"),
                        LabelSeverity::Secondary,
                    ),
                    label(
                        *rhs_span,
                        format!("this has type `{rhs_ty}`"),
                        LabelSeverity::Primary,
                    ),
                ],
            },
            Self::BinaryOperandNotNumeric {
                found,
                operand_span,
            } => DiagnosticDescription {
                code: "E0212",
                message: "binary operator requires integer operands".to_string(),
                anchor: *operand_span,
                labels: vec![label(
                    *operand_span,
                    format!("found type `{found}`"),
                    LabelSeverity::Primary,
                )],
            },
            Self::BinaryOperandNotBool {
                expected,
                found,
                operand_span,
            } => DiagnosticDescription {
                code: "E0206",
                message: "binary operator requires boolean operands".to_string(),
                anchor: *operand_span,
                labels: vec![label(
                    *operand_span,
                    format!("expected `{expected}`, found `{found}`"),
                    LabelSeverity::Primary,
                )],
            },
            Self::UnaryOperandMismatch {
                operator,
                expected,
                found,
                operand_span,
            } => DiagnosticDescription {
                code: "E0213",
                message: format!("cannot apply unary operator `{operator}` to type `{found}`"),
                anchor: *operand_span,
                labels: vec![label(
                    *operand_span,
                    format!("expected `{expected}`, found `{found}`"),
                    LabelSeverity::Primary,
                )],
            },
            Self::BlockMissingTail {
                expected,
                block_span,
            } => DiagnosticDescription {
                code: "E0214",
                message: format!("block is missing a tail expression of type `{expected}`"),
                anchor: *block_span,
                labels: vec![label(
                    *block_span,
                    format!("expected `{expected}`, found `()`"),
                    LabelSeverity::Primary,
                )],
            },
            Self::ReturnMissingValue {
                expected,
                return_span,
            } => DiagnosticDescription {
                code: "E0215",
                message: format!("return without value in function expecting `{expected}`"),
                anchor: *return_span,
                labels: vec![label(
                    *return_span,
                    format!("expected `{expected}`"),
                    LabelSeverity::Primary,
                )],
            },
            Self::ReturnOutsideFunction { span } => DiagnosticDescription {
                code: "E0217",
                message: "return statement outside of function body".to_string(),
                anchor: *span,
                labels: vec![unlabeled(*span, LabelSeverity::Primary)],
            },
            Self::NonConstantValue { span } => DiagnosticDescription {
                code: "E0218",
                message: "attempt to use a non-constant value in a constant".to_string(),
                anchor: *span,
                labels: vec![label(*span, "non-constant value", LabelSeverity::Primary)],
            },
            Self::CaptureInFunction { span } => DiagnosticDescription {
                code: "E0219",
                message: "cannot capture variable from enclosing function".to_string(),
                anchor: *span,
                labels: vec![label(
                    *span,
                    "not accessible inside nested function",
                    LabelSeverity::Primary,
                )],
            },
            Self::LoopBodyNotUnit {
                source,
                found,
                body_span,
            } => DiagnosticDescription {
                code: "E0220",
                message: format!("{} body must evaluate to `()`", source.diagnostic_name()),
                anchor: *body_span,
                labels: vec![label(
                    *body_span,
                    format!("found type `{found}`, expected `()`"),
                    LabelSeverity::Primary,
                )],
            },
            Self::BreakOutsideLoop { span } => DiagnosticDescription {
                code: "E0221",
                message: "`break` outside of a loop".to_string(),
                anchor: *span,
                labels: vec![unlabeled(*span, LabelSeverity::Primary)],
            },
            Self::ContinueOutsideLoop { span } => DiagnosticDescription {
                code: "E0222",
                message: "`continue` outside of a loop".to_string(),
                anchor: *span,
                labels: vec![unlabeled(*span, LabelSeverity::Primary)],
            },
            Self::BreakWithValueFromWhile { span } => DiagnosticDescription {
                code: "E0223",
                message: "`break` with value from a `while` loop".to_string(),
                anchor: *span,
                labels: vec![label(
                    *span,
                    "can only break with a value inside `loop`",
                    LabelSeverity::Primary,
                )],
            },
            Self::LetMissingTypeOrValue { span } => DiagnosticDescription {
                code: "E0224",
                message: "cannot infer type for this binding".to_string(),
                anchor: *span,
                labels: vec![label(
                    *span,
                    "needs a type annotation or an initializer",
                    LabelSeverity::Primary,
                )],
            },
            Self::InvalidIntegerLiteral { found, span } => DiagnosticDescription {
                code: "E0225",
                message: format!("invalid integer literal `{found}`"),
                anchor: *span,
                labels: vec![label(
                    *span,
                    "digits invalid for this literal's base, or too large to fit",
                    LabelSeverity::Primary,
                )],
            },
        }
    }
}
