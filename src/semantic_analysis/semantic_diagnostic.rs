use ariadne::{Color, Label, Report, ReportKind, Source};

use crate::common::span::Span;
use crate::front_end::semantic_analysis::hir::LoopSource;

#[derive(Debug, Clone)]
pub(crate) enum SemanticDiagnostic {
    TypeMismatch {
        expected: String,
        found: String,
        span: Span,
    },
    ArityMismatch {
        expected: usize,
        found: usize,
        call_span: Span,
        callee_span: Span,
        extra_arg_spans: Vec<Span>,
    },
    DuplicateDefinition {
        name: String,
        span: Span,
        previous_span: Span,
    },
    UnknownType {
        name: String,
        span: Span,
    },
    UnresolvedName {
        name: String,
        span: Span,
    },
    NotCallable {
        found: String,
        callee_span: Span,
        call_span: Span,
    },
    InvalidAssignTarget {
        span: Span,
    },
    IfBranchMismatch {
        then_ty: String,
        else_ty: String,
        then_span: Span,
        else_span: Span,
    },
    IfWithoutElse {
        found: String,
        then_span: Span,
    },
    BinaryOperandMismatch {
        lhs_ty: String,
        rhs_ty: String,
        lhs_span: Span,
        rhs_span: Span,
    },
    BinaryOperandNotNumeric {
        found: String,
        operand_span: Span,
    },
    BinaryOperandNotBool {
        expected: String,
        found: String,
        operand_span: Span,
    },
    UnaryOperandMismatch {
        operator: String,
        expected: String,
        found: String,
        operand_span: Span,
    },
    BlockMissingTail {
        expected: String,
        block_span: Span,
    },
    ReturnMissingValue {
        expected: String,
        return_span: Span,
    },
    ReturnOutsideFunction {
        span: Span,
    },
    NonConstantValue {
        span: Span,
    },
    CaptureInFunction {
        span: Span,
    },
    LoopBodyNotUnit {
        source: LoopSource,
        found: String,
        body_span: Span,
    },
    BreakOutsideLoop {
        span: Span,
    },
    ContinueOutsideLoop {
        span: Span,
    },
    BreakWithValueFromWhile {
        span: Span,
    },
    LetMissingTypeOrValue {
        span: Span,
    },
    InvalidIntegerLiteral {
        found: String,
        span: Span,
    },
}

impl SemanticDiagnostic {
    pub(crate) fn render(&self, filename: &str, source: &str) {
        let report = match self {
            Self::TypeMismatch {
                expected,
                found,
                span,
            } => Report::build(ReportKind::Error, filename, span.start() as usize)
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
                extra_arg_spans,
            } => {
                let mut builder =
                    Report::build(ReportKind::Error, filename, call_span.start() as usize)
                        .with_code("E0202")
                        .with_message(format!(
                            "this function takes {} argument{} but {} {} supplied",
                            expected,
                            if *expected == 1 { "" } else { "s" },
                            found,
                            if *found == 1 { "was" } else { "were" },
                        ));

                for (i, span) in extra_arg_spans.iter().enumerate() {
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
                Report::build(ReportKind::Error, filename, span.start() as usize)
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
                    Report::build(ReportKind::Error, filename, span.start() as usize)
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
                Report::build(ReportKind::Error, filename, span.start() as usize)
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
            } => Report::build(ReportKind::Error, filename, call_span.start() as usize)
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
            } => Report::build(ReportKind::Error, filename, then_span.start() as usize)
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
                Report::build(ReportKind::Error, filename, then_span.start() as usize)
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
            } => Report::build(ReportKind::Error, filename, lhs_span.start() as usize)
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
            } => Report::build(ReportKind::Error, filename, operand_span.start() as usize)
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
            } => Report::build(ReportKind::Error, filename, operand_span.start() as usize)
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
            } => Report::build(ReportKind::Error, filename, operand_span.start() as usize)
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
            } => Report::build(ReportKind::Error, filename, block_span.start() as usize)
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
            } => Report::build(ReportKind::Error, filename, return_span.start() as usize)
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
                Report::build(ReportKind::Error, filename, span.start() as usize)
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
                Report::build(ReportKind::Error, filename, span.start() as usize)
                    .with_code("E0217")
                    .with_message("return statement outside of function body")
                    .with_label(Label::new((filename, span.into())).with_color(Color::Red))
                    .finish()
            }
            Self::NonConstantValue { span } => {
                Report::build(ReportKind::Error, filename, span.start() as usize)
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
                Report::build(ReportKind::Error, filename, span.start() as usize)
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
            } => Report::build(ReportKind::Error, filename, body_span.start() as usize)
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
                Report::build(ReportKind::Error, filename, span.start() as usize)
                    .with_code("E0221")
                    .with_message("`break` outside of a loop")
                    .with_label(Label::new((filename, span.into())).with_color(Color::Red))
                    .finish()
            }
            Self::ContinueOutsideLoop { span } => {
                Report::build(ReportKind::Error, filename, span.start() as usize)
                    .with_code("E0222")
                    .with_message("`continue` outside of a loop")
                    .with_label(Label::new((filename, span.into())).with_color(Color::Red))
                    .finish()
            }
            Self::BreakWithValueFromWhile { span } => {
                Report::build(ReportKind::Error, filename, span.start() as usize)
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
                Report::build(ReportKind::Error, filename, span.start() as usize)
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
                Report::build(ReportKind::Error, filename, span.start() as usize)
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
