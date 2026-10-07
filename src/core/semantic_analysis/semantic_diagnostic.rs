use crate::core::common::diagnostic::{DiagnosticDescription, DiagnosticLabel, LabelSeverity};
use crate::core::common::span::Span;
use crate::core::semantic_analysis::hir::nodes::{
    BinaryOperator, DefinitionBody, DefinitionBodySourceMap, Expression, ExpressionHandle,
    TypeAnnotation, UnaryOperator,
};
use crate::core::semantic_analysis::type_checking::inference_diagnostic::{
    InferenceDiagnostic, MismatchCause,
};
use crate::core::semantic_analysis::type_checking::type_checker::TypeCheckSink;
use crate::core::source_file_key::SourceFileKey;

/// A semantic error, ready to be shown: types are rendered, and HIR handles are turned into
/// source spans.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SemanticDiagnostic {
    TypeMismatch {
        expected: String,
        found: String,
        span: Span,
        /// The function's return type, when the expected type comes from it.
        return_type_span: Option<Span>,
    },
    /// A function's body has no tail, so it returns `()` where `expected` is declared.
    ImplicitUnitReturn {
        expected: String,
        return_type_span: Span,
        function_name_span: Option<Span>,
    },
    IfElseMismatch {
        expected: String,
        found: String,
        then_span: Span,
        else_span: Span,
    },
    IfWithoutElse {
        expected: String,
        span: Span,
    },
    ReturnWithoutValue {
        span: Span,
    },
    MismatchedArgumentCount {
        expected: usize,
        found: usize,
        callee_span: Span,
        /// One label per unexpected or missing argument.
        argument_labels: Vec<(Span, String)>,
        /// The called function's name, when it's defined in the same file.
        definition_span: Option<Span>,
    },
    UnresolvedName {
        /// `"value"`, or `"function"` when the name is called.
        kind: &'static str,
        name: String,
        span: Span,
    },
    UnknownType {
        name: String,
        span: Span,
    },
    ExpectedFunction {
        found: String,
        /// The callee's name, if it's a path.
        callee_name: Option<String>,
        callee_span: Span,
        call_span: Span,
    },
    UnaryOperatorCannotBeApplied {
        operator: String,
        found: String,
        is_unsigned: bool,
        span: Span,
    },
    BinaryOperatorCannotBeApplied {
        operator: BinaryOperator,
        lhs: String,
        rhs: String,
        span: Span,
        lhs_span: Span,
        rhs_span: Span,
    },
    InvalidAssignmentTarget {
        span: Span,
        target_span: Span,
    },
    ReturnOutsideFunction {
        span: Span,
    },
    BreakOutsideOfLoop {
        span: Span,
    },
    ContinueOutsideOfLoop {
        span: Span,
    },
    BreakWithValueFromWhile {
        span: Span,
    },
    TypeMustBeKnown {
        span: Span,
    },
}

/// Where the definition owning a body is written, for labels pointing back at its signature.
pub(crate) struct OwnerSpans {
    pub(crate) file: SourceFileKey,
    pub(crate) name: Option<Span>,
    /// A function's return type annotation, if it has one.
    pub(crate) return_type: Option<Span>,
}

impl SemanticDiagnostic {
    pub(crate) fn from_inference<'db>(
        db: &'db dyn crate::Db,
        body: &DefinitionBody<'db>,
        source_map: &DefinitionBodySourceMap,
        types: &TypeCheckSink<'db>,
        owner: &OwnerSpans,
        diagnostic: &InferenceDiagnostic<'db>,
    ) -> Self {
        let span_of =
            |expression_handle: ExpressionHandle| source_map.expressions[expression_handle].span;
        let path_text =
            |expression_handle: ExpressionHandle| match &body.expressions[expression_handle] {
                Expression::Path(path) => Some(
                    path.segments(db)
                        .iter()
                        .map(|segment| segment.text(db))
                        .collect::<Vec<_>>()
                        .join("::"),
                ),
                _ => None,
            };
        match diagnostic {
            InferenceDiagnostic::TypeMismatch {
                expression_handle,
                expected,
                found,
                cause,
            } => match (cause, owner.return_type) {
                (MismatchCause::ImplicitUnitReturn, Some(return_type_span)) => {
                    Self::ImplicitUnitReturn {
                        expected: expected.clone(),
                        return_type_span,
                        function_name_span: owner.name,
                    }
                }
                _ => Self::TypeMismatch {
                    expected: expected.clone(),
                    found: found.clone(),
                    span: span_of(*expression_handle),
                    return_type_span: match cause {
                        MismatchCause::ReturnValue => owner.return_type,
                        _ => None,
                    },
                },
            },
            InferenceDiagnostic::IfElseMismatch {
                then_branch_handle,
                else_branch_handle,
                expected,
                found,
            } => Self::IfElseMismatch {
                expected: expected.clone(),
                found: found.clone(),
                then_span: span_of(value_expression(body, *then_branch_handle)),
                else_span: span_of(value_expression(body, *else_branch_handle)),
            },
            InferenceDiagnostic::IfWithoutElse {
                expression_handle,
                expected,
            } => Self::IfWithoutElse {
                expected: expected.clone(),
                span: span_of(*expression_handle),
            },
            InferenceDiagnostic::ReturnWithoutValue {
                expression_handle, ..
            } => Self::ReturnWithoutValue {
                span: span_of(*expression_handle),
            },
            InferenceDiagnostic::UnresolvedName { expression_handle } => {
                let is_callee = body.expressions.values().any(|other| {
                    matches!(other, Expression::Call { callee_handle, .. } if callee_handle == expression_handle)
                });
                Self::UnresolvedName {
                    kind: if is_callee { "function" } else { "value" },
                    name: path_text(*expression_handle).expect("only paths are resolved"),
                    span: span_of(*expression_handle),
                }
            }
            InferenceDiagnostic::UnknownType { annotation_handle } => {
                let TypeAnnotation::Path(name) = &body.type_annotations[*annotation_handle] else {
                    unreachable!("only named annotations can be unknown")
                };
                Self::UnknownType {
                    name: name.text(db).to_string(),
                    span: source_map.type_annotations[*annotation_handle].span,
                }
            }
            InferenceDiagnostic::ExpectedFunction { call_handle, found } => {
                let Expression::Call { callee_handle, .. } = &body.expressions[*call_handle] else {
                    unreachable!("reported on calls")
                };
                Self::ExpectedFunction {
                    found: found.clone(),
                    callee_name: path_text(*callee_handle),
                    callee_span: span_of(*callee_handle),
                    call_span: span_of(*call_handle),
                }
            }
            InferenceDiagnostic::MismatchedArgumentCount {
                call_handle,
                function,
                parameters,
            } => {
                let Expression::Call {
                    callee_handle,
                    argument_segment,
                } = &body.expressions[*call_handle]
                else {
                    unreachable!("reported on calls")
                };
                let argument_handles = &body.expression_children[*argument_segment];
                let argument_labels = if argument_handles.len() > parameters.len() {
                    (parameters.len()..argument_handles.len())
                        .map(|index| {
                            let argument_handle = argument_handles[index];
                            let ty = types.expression_type(argument_handle).display(db);
                            (
                                span_of(argument_handle),
                                format!("unexpected argument #{} of type `{ty}`", index + 1),
                            )
                        })
                        .collect()
                } else {
                    (argument_handles.len()..parameters.len())
                        .map(|index| {
                            let ty = &parameters[index];
                            (
                                span_of(*call_handle),
                                format!("argument #{} of type `{ty}` is missing", index + 1),
                            )
                        })
                        .collect()
                };
                // Only a definition in the same file can be shown alongside the call.
                let definition_span = function
                    .filter(|function| *function.id(db).file(db) == owner.file)
                    .and_then(|function| function.id(db).to_ast_node(db).name())
                    .map(|name| name.span());
                Self::MismatchedArgumentCount {
                    expected: parameters.len(),
                    found: argument_handles.len(),
                    callee_span: span_of(*callee_handle),
                    argument_labels,
                    definition_span,
                }
            }
            InferenceDiagnostic::UnaryOperatorCannotBeApplied {
                expression_handle,
                operator,
                found,
                is_unsigned,
            } => Self::UnaryOperatorCannotBeApplied {
                operator: match operator {
                    UnaryOperator::Neg => "-".to_string(),
                    UnaryOperator::Not => "not".to_string(),
                },
                found: found.clone(),
                is_unsigned: *is_unsigned,
                span: span_of(*expression_handle),
            },
            InferenceDiagnostic::BinaryOperatorCannotBeApplied {
                expression_handle,
                operator,
                lhs,
                rhs,
            } => {
                let Expression::BinaryOperation {
                    lhs_handle,
                    rhs_handle,
                    ..
                } = &body.expressions[*expression_handle]
                else {
                    unreachable!("reported on binary operations")
                };
                Self::BinaryOperatorCannotBeApplied {
                    operator: *operator,
                    lhs: lhs.clone(),
                    rhs: rhs.clone(),
                    span: span_of(*expression_handle),
                    lhs_span: span_of(*lhs_handle),
                    rhs_span: span_of(*rhs_handle),
                }
            }
            InferenceDiagnostic::InvalidAssignmentTarget {
                assignment_handle,
                target_handle,
            } => Self::InvalidAssignmentTarget {
                span: span_of(*assignment_handle),
                target_span: span_of(*target_handle),
            },
            InferenceDiagnostic::BreakOutsideOfLoop {
                expression_handle,
                is_break,
                bad_value_break,
            } => {
                let span = span_of(*expression_handle);
                match (is_break, bad_value_break) {
                    (true, true) => Self::BreakWithValueFromWhile { span },
                    (true, false) => Self::BreakOutsideOfLoop { span },
                    (false, _) => Self::ContinueOutsideOfLoop { span },
                }
            }
            InferenceDiagnostic::ReturnOutsideFunction { expression_handle } => {
                Self::ReturnOutsideFunction {
                    span: span_of(*expression_handle),
                }
            }
            InferenceDiagnostic::TypeMustBeKnown { expression_handle } => Self::TypeMustBeKnown {
                span: span_of(*expression_handle),
            },
        }
    }

    pub(crate) fn describe(&self) -> DiagnosticDescription {
        match self {
            Self::TypeMismatch {
                expected,
                found,
                span,
                return_type_span,
            } => {
                let mut labels = vec![primary(*span, expected_found(expected, found))];
                if let Some(return_type_span) = return_type_span {
                    labels.push(secondary(
                        *return_type_span,
                        format!("expected {} because of return type", quote_type(expected)),
                    ));
                }
                description("E0308", "mismatched types", *span, labels)
            }
            Self::ImplicitUnitReturn {
                expected,
                return_type_span,
                function_name_span,
            } => {
                let mut labels = vec![primary(*return_type_span, expected_found(expected, "()"))];
                if let Some(function_name_span) = function_name_span {
                    labels.push(secondary(
                        *function_name_span,
                        "implicitly returns `()` as its body has no tail or `return` expression",
                    ));
                }
                description("E0308", "mismatched types", *return_type_span, labels)
            }
            Self::IfElseMismatch {
                expected,
                found,
                then_span,
                else_span,
            } => description(
                "E0308",
                "`if` and `else` have incompatible types",
                *else_span,
                vec![
                    secondary(*then_span, "expected because of this"),
                    primary(*else_span, expected_found(expected, found)),
                ],
            ),
            Self::IfWithoutElse { expected, span } => DiagnosticDescription {
                notes: vec!["`if` expressions without `else` evaluate to `()`".to_string()],
                helps: vec![
                    "consider adding an `else` block that evaluates to the expected type"
                        .to_string(),
                ],
                ..description(
                    "E0317",
                    "`if` may be missing an `else` clause",
                    *span,
                    vec![primary(*span, expected_found(expected, "()"))],
                )
            },
            Self::ReturnWithoutValue { span } => description(
                "E0069",
                "`return;` in a function whose return type is not `()`",
                *span,
                vec![primary(*span, "return type is not `()`")],
            ),
            Self::MismatchedArgumentCount {
                expected,
                found,
                callee_span,
                argument_labels,
                definition_span,
            } => {
                let mut labels = vec![unlabeled(*callee_span)];
                labels.extend(
                    argument_labels
                        .iter()
                        .map(|(span, message)| secondary(*span, message.clone())),
                );
                if let Some(definition_span) = definition_span {
                    labels.push(secondary(*definition_span, "function defined here"));
                }
                description(
                    "E0061",
                    &format!(
                        "this function takes {} but {} {} supplied",
                        count(*expected, "argument"),
                        count(*found, "argument"),
                        if *found == 1 { "was" } else { "were" },
                    ),
                    *callee_span,
                    labels,
                )
            }
            Self::UnresolvedName { kind, name, span } => description(
                "E0425",
                &format!("cannot find {kind} `{name}` in this scope"),
                *span,
                vec![primary(*span, "not found in this scope")],
            ),
            Self::UnknownType { name, span } => description(
                "E0412",
                &format!("cannot find type `{name}` in this scope"),
                *span,
                vec![primary(*span, "not found in this scope")],
            ),
            Self::ExpectedFunction {
                found,
                callee_name,
                callee_span,
                call_span,
            } => {
                let callee = match callee_name {
                    Some(name) => format!("`{name}` has type `{found}`"),
                    None => format!("this expression has type `{found}`"),
                };
                description(
                    "E0618",
                    &format!("expected function, found `{found}`"),
                    *call_span,
                    vec![
                        secondary(*callee_span, callee),
                        primary(*call_span, "call expression requires function"),
                    ],
                )
            }
            Self::UnaryOperatorCannotBeApplied {
                operator,
                found,
                is_unsigned,
                span,
            } => DiagnosticDescription {
                notes: if *is_unsigned {
                    vec!["unsigned values cannot be negated".to_string()]
                } else {
                    Vec::new()
                },
                ..description(
                    "E0600",
                    &format!("cannot apply unary operator `{operator}` to type `{found}`"),
                    *span,
                    vec![primary(
                        *span,
                        format!("cannot apply unary operator `{operator}`"),
                    )],
                )
            },
            Self::BinaryOperatorCannotBeApplied {
                operator,
                lhs,
                rhs,
                span,
                lhs_span,
                rhs_span,
            } => {
                let message = match operator {
                    BinaryOperator::Add => format!("cannot add `{rhs}` to `{lhs}`"),
                    BinaryOperator::Sub => format!("cannot subtract `{rhs}` from `{lhs}`"),
                    BinaryOperator::Mul => format!("cannot multiply `{lhs}` by `{rhs}`"),
                    BinaryOperator::Div => format!("cannot divide `{lhs}` by `{rhs}`"),
                    _ => unreachable!("only arithmetic operators need integer operands"),
                };
                description(
                    "E0369",
                    &message,
                    *span,
                    vec![
                        unlabeled(*span),
                        secondary(*lhs_span, lhs.clone()),
                        secondary(*rhs_span, rhs.clone()),
                    ],
                )
            }
            Self::InvalidAssignmentTarget { span, target_span } => description(
                "E0070",
                "invalid left-hand side of assignment",
                *span,
                vec![
                    unlabeled(*span),
                    secondary(*target_span, "cannot assign to this expression"),
                ],
            ),
            Self::ReturnOutsideFunction { span } => description(
                "E0572",
                "return statement outside of function body",
                *span,
                vec![unlabeled(*span)],
            ),
            Self::BreakOutsideOfLoop { span } => description(
                "E0268",
                "`break` outside of a loop",
                *span,
                vec![primary(*span, "cannot `break` outside of a loop")],
            ),
            Self::ContinueOutsideOfLoop { span } => description(
                "E0268",
                "`continue` outside of a loop",
                *span,
                vec![primary(*span, "cannot `continue` outside of a loop")],
            ),
            Self::BreakWithValueFromWhile { span } => DiagnosticDescription {
                helps: vec![
                    "use `break` on its own without a value inside this `while` loop".to_string(),
                ],
                ..description(
                    "E0571",
                    "`break` with value from a `while` loop",
                    *span,
                    vec![primary(*span, "can only break with a value inside `loop`")],
                )
            },
            Self::TypeMustBeKnown { span } => description(
                "E0282",
                "type annotations needed",
                *span,
                vec![primary(*span, "cannot infer type")],
            ),
        }
    }
}

/// Returns the expression whose value a branch produces: a block's tail, recursively. `if`/`else`
/// mismatches point at it rather than at the whole block.
fn value_expression(
    body: &DefinitionBody<'_>,
    expression_handle: ExpressionHandle,
) -> ExpressionHandle {
    match &body.expressions[expression_handle] {
        Expression::Block {
            tail_handle: Some(tail_handle),
            ..
        } => value_expression(body, *tail_handle),
        _ => expression_handle,
    }
}

/// Formats the `expected X, found Y` label, where an unknown integer type is written `integer`.
fn expected_found(expected: &str, found: &str) -> String {
    format!(
        "expected {}, found {}",
        quote_type(expected),
        quote_type(found)
    )
}

fn quote_type(ty: &str) -> String {
    match ty {
        "{integer}" => "integer".to_string(),
        _ => format!("`{ty}`"),
    }
}

/// Formats a count with its noun, e.g. `1 argument`, `2 arguments`.
fn count(n: usize, noun: &str) -> String {
    format!("{n} {noun}{}", if n == 1 { "" } else { "s" })
}

fn description(
    code: &'static str,
    message: &str,
    span: Span,
    labels: Vec<DiagnosticLabel>,
) -> DiagnosticDescription {
    DiagnosticDescription {
        code,
        message: message.to_string(),
        span,
        labels,
        notes: Vec::new(),
        helps: Vec::new(),
    }
}

fn primary(span: Span, message: impl Into<String>) -> DiagnosticLabel {
    DiagnosticLabel {
        span,
        message: Some(message.into()),
        severity: LabelSeverity::Primary,
    }
}

fn secondary(span: Span, message: impl Into<String>) -> DiagnosticLabel {
    DiagnosticLabel {
        span,
        message: Some(message.into()),
        severity: LabelSeverity::Secondary,
    }
}

fn unlabeled(span: Span) -> DiagnosticLabel {
    DiagnosticLabel {
        span,
        message: None,
        severity: LabelSeverity::Primary,
    }
}
