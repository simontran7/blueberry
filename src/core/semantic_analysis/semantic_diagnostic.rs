use ariadne::{Color, Label, Report, ReportKind, Source};

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

impl SemanticDiagnostic {
    pub(crate) fn render(&self, filename: &str, source: &str) {
        let report = match self {
            Self::TypeMismatch {
                expected,
                found,
                span,
            } => Report::build(ReportKind::Error, filename, usize::from(span.start()))
                .with_code("E0201")
                .with_message("mismatched types".to_string())
                .with_label(
                    Label::new((filename, span.into()))
                        .with_message(format!("expected `{}`, found `{}`", expected, found))
                        .with_color(Color::Red),
                )
                .finish(),
            Self::ArityMismatch {
                expected,
                found,
                call_span,
                callee_span,
                extra_argument_spans,
            } => {
                let mut builder =
                    Report::build(ReportKind::Error, filename, usize::from(call_span.start()))
                        .with_code("E0202")
                        .with_message(format!(
                            "this function takes {} argument{} but {} {} supplied",
                            expected,
                            if *expected == 1 { "" } else { "s" },
                            found,
                            if *found == 1 { "was" } else { "were" },
                        ));

                for (i, span) in extra_argument_spans.iter().enumerate() {
                    builder = builder.with_label(
                        Label::new((filename, span.into()))
                            .with_message(format!("unexpected argument #{}", expected + i + 1))
                            .with_color(Color::Red),
                    );
                }

                builder = builder.with_label(
                    Label::new((filename, callee_span.into()))
                        .with_message("function defined here")
                        .with_color(Color::Blue),
                );

                builder.finish()
            }
            Self::UnresolvedName { name, span } => {
                Report::build(ReportKind::Error, filename, usize::from(span.start()))
                    .with_code("E0203")
                    .with_message(format!("cannot find value `{}` in this scope", name))
                    .with_label(
                        Label::new((filename, span.into()))
                            .with_message("not found in this scope".to_string())
                            .with_color(Color::Red),
                    )
                    .finish()
            }
            Self::DuplicateDefinition {
                name,
                span,
                previous_span,
            } => {
                let mut report_builder =
                    Report::build(ReportKind::Error, filename, usize::from(span.start()))
                        .with_code("E0204")
                        .with_message(format!("the name `{}` is defined multiple times", name))
                        .with_label(
                            Label::new((filename, span.into()))
                                .with_message(format!("`{}` redefined here", name))
                                .with_color(Color::Red),
                        );

                report_builder = report_builder.with_label(
                    Label::new((filename, previous_span.into()))
                        .with_message(format!("previous definition of `{}` here", name))
                        .with_color(Color::Blue),
                );

                report_builder.finish()
            }
            Self::UnknownType { name, span } => {
                Report::build(ReportKind::Error, filename, usize::from(span.start()))
                    .with_code("E0205")
                    .with_message(format!("cannot find type `{}` in this scope", name))
                    .with_label(
                        Label::new((filename, span.into()))
                            .with_message("not found in this scope")
                            .with_color(Color::Red),
                    )
                    .finish()
            }
            Self::NotCallable {
                found,
                callee_span,
                call_span,
            } => Report::build(ReportKind::Error, filename, usize::from(call_span.start()))
                .with_code("E0207")
                .with_message(format!("expected function, found `{}`", found))
                .with_label(
                    Label::new((filename, callee_span.into()))
                        .with_message(format!("has type `{}`", found))
                        .with_color(Color::Blue),
                )
                .with_label(
                    Label::new((filename, call_span.into()))
                        .with_message("call expression requires function")
                        .with_color(Color::Red),
                )
                .finish(),
            Self::IfBranchMismatch {
                then_ty,
                else_ty,
                then_span,
                else_span,
            } => Report::build(ReportKind::Error, filename, usize::from(then_span.start()))
                .with_code("E0209")
                .with_message("if and else branches have incompatible types")
                .with_label(
                    Label::new((filename, then_span.into()))
                        .with_message(format!("then branch has type `{}`", then_ty))
                        .with_color(Color::Blue),
                )
                .with_label(
                    Label::new((filename, else_span.into()))
                        .with_message(format!("else branch has type `{}`", else_ty))
                        .with_color(Color::Red),
                )
                .finish(),
            Self::IfWithoutElse { found, then_span } => {
                Report::build(ReportKind::Error, filename, usize::from(then_span.start()))
                    .with_code("E0210")
                    .with_message("if without else must evaluate to `()`")
                    .with_label(
                        Label::new((filename, then_span.into()))
                            .with_message(format!("found type `{}`, expected `()`", found))
                            .with_color(Color::Red),
                    )
                    .finish()
            }
            Self::BinaryOperandMismatch {
                lhs_ty,
                rhs_ty,
                lhs_span,
                rhs_span,
            } => Report::build(ReportKind::Error, filename, usize::from(lhs_span.start()))
                .with_code("E0211")
                .with_message("binary operation applied to mismatched types")
                .with_label(
                    Label::new((filename, lhs_span.into()))
                        .with_message(format!("this has type `{}`", lhs_ty))
                        .with_color(Color::Blue),
                )
                .with_label(
                    Label::new((filename, rhs_span.into()))
                        .with_message(format!("this has type `{}`", rhs_ty))
                        .with_color(Color::Red),
                )
                .finish(),
            Self::BinaryOperandNotNumeric {
                found,
                operand_span,
            } => Report::build(ReportKind::Error, filename, usize::from(operand_span.start()))
                .with_code("E0212")
                .with_message("binary operator requires integer operands")
                .with_label(
                    Label::new((filename, operand_span.into()))
                        .with_message(format!("found type `{}`", found))
                        .with_color(Color::Red),
                )
                .finish(),
            Self::BinaryOperandNotBool {
                expected,
                found,
                operand_span,
            } => Report::build(ReportKind::Error, filename, usize::from(operand_span.start()))
                .with_code("E0206")
                .with_message("binary operator requires boolean operands")
                .with_label(
                    Label::new((filename, operand_span.into()))
                        .with_message(format!("expected `{}`, found `{}`", expected, found))
                        .with_color(Color::Red),
                )
                .finish(),
            Self::UnaryOperandMismatch {
                operator,
                expected,
                found,
                operand_span,
            } => Report::build(ReportKind::Error, filename, usize::from(operand_span.start()))
                .with_code("E0213")
                .with_message(format!(
                    "cannot apply unary operator `{}` to type `{}`",
                    operator, found
                ))
                .with_label(
                    Label::new((filename, operand_span.into()))
                        .with_message(format!("expected `{}`, found `{}`", expected, found))
                        .with_color(Color::Red),
                )
                .finish(),
            Self::BlockMissingTail {
                expected,
                block_span,
            } => Report::build(ReportKind::Error, filename, usize::from(block_span.start()))
                .with_code("E0214")
                .with_message(format!(
                    "block is missing a tail expression of type `{}`",
                    expected
                ))
                .with_label(
                    Label::new((filename, block_span.into()))
                        .with_message(format!("expected `{}`, found `()`", expected))
                        .with_color(Color::Red),
                )
                .finish(),
            Self::ReturnMissingValue {
                expected,
                return_span,
            } => Report::build(ReportKind::Error, filename, usize::from(return_span.start()))
                .with_code("E0215")
                .with_message(format!(
                    "return without value in function expecting `{}`",
                    expected
                ))
                .with_label(
                    Label::new((filename, return_span.into()))
                        .with_message(format!("expected `{}`", expected))
                        .with_color(Color::Red),
                )
                .finish(),
            Self::InvalidAssignTarget { span } => {
                Report::build(ReportKind::Error, filename, usize::from(span.start()))
                    .with_code("E0216")
                    .with_message("invalid left-hand side of assignment")
                    .with_label(
                        Label::new((filename, span.into()))
                            .with_message("cannot assign to this expression")
                            .with_color(Color::Red),
                    )
                    .finish()
            }
            Self::ReturnOutsideFunction { span } => {
                Report::build(ReportKind::Error, filename, usize::from(span.start()))
                    .with_code("E0217")
                    .with_message("return statement outside of function body")
                    .with_label(Label::new((filename, span.into())).with_color(Color::Red))
                    .finish()
            }
            Self::NonConstantValue { span } => {
                Report::build(ReportKind::Error, filename, usize::from(span.start()))
                    .with_code("E0218")
                    .with_message("attempt to use a non-constant value in a constant")
                    .with_label(
                        Label::new((filename, span.into()))
                            .with_message("non-constant value")
                            .with_color(Color::Red),
                    )
                    .finish()
            }
            Self::CaptureInFunction { span } => {
                Report::build(ReportKind::Error, filename, usize::from(span.start()))
                    .with_code("E0219")
                    .with_message("cannot capture variable from enclosing function")
                    .with_label(
                        Label::new((filename, span.into()))
                            .with_message("not accessible inside nested function")
                            .with_color(Color::Red),
                    )
                    .finish()
            }
            Self::LoopBodyNotUnit {
                source,
                found,
                body_span,
            } => Report::build(ReportKind::Error, filename, usize::from(body_span.start()))
                .with_code("E0220")
                .with_message(format!(
                    "{} body must evaluate to `()`",
                    source.diagnostic_name()
                ))
                .with_label(
                    Label::new((filename, body_span.into()))
                        .with_message(format!("found type `{}`, expected `()`", found))
                        .with_color(Color::Red),
                )
                .finish(),
            Self::BreakOutsideLoop { span } => {
                Report::build(ReportKind::Error, filename, usize::from(span.start()))
                    .with_code("E0221")
                    .with_message("`break` outside of a loop")
                    .with_label(Label::new((filename, span.into())).with_color(Color::Red))
                    .finish()
            }
            Self::ContinueOutsideLoop { span } => {
                Report::build(ReportKind::Error, filename, usize::from(span.start()))
                    .with_code("E0222")
                    .with_message("`continue` outside of a loop")
                    .with_label(Label::new((filename, span.into())).with_color(Color::Red))
                    .finish()
            }
            Self::BreakWithValueFromWhile { span } => {
                Report::build(ReportKind::Error, filename, usize::from(span.start()))
                    .with_code("E0223")
                    .with_message("`break` with value from a `while` loop")
                    .with_label(
                        Label::new((filename, span.into()))
                            .with_message("can only break with a value inside `loop`")
                            .with_color(Color::Red),
                    )
                    .finish()
            }
            Self::LetMissingTypeOrValue { span } => {
                Report::build(ReportKind::Error, filename, usize::from(span.start()))
                    .with_code("E0224")
                    .with_message("cannot infer type for this binding")
                    .with_label(
                        Label::new((filename, span.into()))
                            .with_message("needs a type annotation or an initializer")
                            .with_color(Color::Red),
                    )
                    .finish()
            }
            Self::InvalidIntegerLiteral { found, span } => {
                Report::build(ReportKind::Error, filename, usize::from(span.start()))
                    .with_code("E0225")
                    .with_message(format!("invalid integer literal `{}`", found))
                    .with_label(
                        Label::new((filename, span.into()))
                            .with_message(
                                "digits invalid for this literal's base, or too large to fit",
                            )
                            .with_color(Color::Red),
                    )
                    .finish()
            }
        };
        report.eprint((filename, Source::from(source))).unwrap();
    }
}
