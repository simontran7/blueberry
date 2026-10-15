use crate::core::common::diagnostic::{DiagnosticDescription, DiagnosticLabel, LabelSeverity};
use crate::core::common::span::Span;
use crate::core::semantic_analysis::hir::nodes::{
    BinaryOperator, DefinitionBody, DefinitionBodySourceMap, Expression, ExpressionHandle,
    TypeAnnotation, TypeAnnotationHandle, UnaryOperator,
};
use crate::core::semantic_analysis::ids::keys::FunctionKey;
use crate::core::semantic_analysis::name_resolution::resolver::{Resolution, ResolveError};
use crate::core::semantic_analysis::type_checking::type_checker::TypeCheckSink;
use crate::core::source_file_key::SourceFileKey;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SemanticDiagnostic {
    TypeMismatch {
        expected: String,
        found: String,
        span: Span,
        return_type_span: Option<Span>,
    },
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
        argument_labels: Vec<(Span, String)>,
        definition_span: Option<Span>,
    },
    UnresolvedName {
        name: String,
        span: Span,
    },
    UnresolvedModule {
        name: String,
        span: Span,
    },
    UnresolvedImport {
        path: Vec<String>,
        span: Span,
    },
    MissingFromModule {
        name: String,
        module: String,
        span: Span,
    },
    NotAModule {
        name: String,
        found: &'static str,
        span: Span,
    },
    ExpectedValueFoundModule {
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
    LiteralOutOfRange {
        literal: String,
        ty: String,
        range: String,
        suggested_ty: Option<String>,
        span: Span,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, salsa::SalsaValue)]
pub(crate) enum UnresolvedSemanticDiagnostic<'db> {
    TypeMismatch {
        expression_handle: ExpressionHandle,
        expected: String,
        found: String,
        cause: TypeMismatchCause,
    },
    ReturnWithoutValue {
        expression_handle: ExpressionHandle,
    },
    UnresolvedPath {
        expression_handle: ExpressionHandle,
        error: ResolveError<'db>,
    },
    ExpectedValueFoundModule {
        expression_handle: ExpressionHandle,
    },
    UnknownType {
        annotation_handle: TypeAnnotationHandle,
    },
    ExpectedFunction {
        call_handle: ExpressionHandle,
        found: String,
    },
    MismatchedArgumentCount {
        call_handle: ExpressionHandle,
        function: Option<FunctionKey<'db>>,
        parameters: Vec<String>,
    },
    UnaryOperatorCannotBeApplied {
        expression_handle: ExpressionHandle,
        operator: UnaryOperator,
        found: String,
        is_unsigned: bool,
    },
    BinaryOperatorCannotBeApplied {
        expression_handle: ExpressionHandle,
        operator: BinaryOperator,
        lhs: String,
        rhs: String,
    },
    InvalidAssignmentTarget {
        assignment_handle: ExpressionHandle,
        target_handle: ExpressionHandle,
    },
    BreakOutsideOfLoop {
        expression_handle: ExpressionHandle,
        is_break: bool,
        bad_value_break: bool,
    },
    ReturnOutsideFunction {
        expression_handle: ExpressionHandle,
    },
    TypeMustBeKnown {
        expression_handle: ExpressionHandle,
    },
    LiteralOutOfRange {
        expression_handle: ExpressionHandle,
        ty: String,
        range: String,
        suggested_ty: Option<String>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, salsa::SalsaValue)]
pub(crate) enum TypeMismatchCause {
    General,
    ReturnValue,
    ImplicitUnitReturn,
    IfElse {
        then_branch_handle: ExpressionHandle,
    },
    IfWithoutElse,
}

pub(crate) struct OwnerSpans {
    pub(crate) file: SourceFileKey,
    pub(crate) name: Option<Span>,
    pub(crate) return_type: Option<Span>,
}

impl SemanticDiagnostic {
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
            Self::UnresolvedName { name, span } => description(
                "E0425",
                &format!("cannot find `{name}` in this scope"),
                *span,
                vec![primary(*span, "not found in this scope")],
            ),
            Self::UnresolvedModule { name, span } => {
                let message = format!("use of unresolved module `{name}`");
                description(
                    "E0433",
                    &format!("failed to resolve: {message}"),
                    *span,
                    vec![primary(*span, message)],
                )
            }
            Self::MissingFromModule { name, module, span } => description(
                "E0425",
                &format!("cannot find `{name}` in module `{module}`"),
                *span,
                vec![primary(*span, format!("not found in `{module}`"))],
            ),
            Self::UnresolvedImport { path, span } => {
                let label = match path.split_last() {
                    Some((name, [])) => format!("no `{name}` in the root"),
                    Some((name, parent)) => format!("no `{name}` in `{}`", parent.join("::")),
                    None => unreachable!("an import path has at least one segment"),
                };
                description(
                    "E0432",
                    &format!("unresolved import `{}`", path.join("::")),
                    *span,
                    vec![primary(*span, label)],
                )
            }
            Self::NotAModule { name, found, span } => {
                let message = format!("`{name}` is {found}, not a module");
                description(
                    "E0433",
                    &format!("failed to resolve: {message}"),
                    *span,
                    vec![primary(*span, message)],
                )
            }
            Self::ExpectedValueFoundModule { kind, name, span } => description(
                "E0423",
                &format!("expected {kind}, found module `{name}`"),
                *span,
                vec![primary(*span, format!("not a {kind}"))],
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
            Self::LiteralOutOfRange {
                literal,
                ty,
                range,
                suggested_ty,
                span,
            } => DiagnosticDescription {
                notes: vec![format!(
                    "the literal `{literal}` does not fit into the type `{ty}` whose range is `{range}`"
                )],
                helps: suggested_ty
                    .iter()
                    .map(|suggested_ty| format!("consider using the type `{suggested_ty}` instead"))
                    .collect(),
                ..description(
                    "E0103",
                    &format!("literal out of range for `{ty}`"),
                    *span,
                    vec![unlabeled(*span)],
                )
            },
        }
    }
}

impl<'db> UnresolvedSemanticDiagnostic<'db> {
    pub(crate) fn resolve(
        &self,
        db: &'db dyn crate::Db,
        body: &DefinitionBody<'db>,
        source_map: &DefinitionBodySourceMap,
        types: &TypeCheckSink<'db>,
        owner: &OwnerSpans,
    ) -> SemanticDiagnostic {
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
        match self {
            Self::TypeMismatch {
                expression_handle,
                expected,
                found,
                cause,
            } => match (*cause, owner.return_type) {
                (TypeMismatchCause::ImplicitUnitReturn, Some(return_type_span)) => {
                    SemanticDiagnostic::ImplicitUnitReturn {
                        expected: expected.clone(),
                        return_type_span,
                        function_name_span: owner.name,
                    }
                }
                (TypeMismatchCause::IfElse { then_branch_handle }, _) => {
                    SemanticDiagnostic::IfElseMismatch {
                        expected: expected.clone(),
                        found: found.clone(),
                        then_span: span_of(value_expression(body, then_branch_handle)),
                        else_span: span_of(value_expression(body, *expression_handle)),
                    }
                }
                (TypeMismatchCause::IfWithoutElse, _) => SemanticDiagnostic::IfWithoutElse {
                    expected: expected.clone(),
                    span: span_of(*expression_handle),
                },
                (cause, _) => SemanticDiagnostic::TypeMismatch {
                    expected: expected.clone(),
                    found: found.clone(),
                    span: span_of(*expression_handle),
                    return_type_span: match cause {
                        TypeMismatchCause::ReturnValue => owner.return_type,
                        _ => None,
                    },
                },
            },
            Self::ReturnWithoutValue {
                expression_handle, ..
            } => SemanticDiagnostic::ReturnWithoutValue {
                span: span_of(*expression_handle),
            },
            Self::UnresolvedPath {
                expression_handle,
                error,
            } => {
                let Expression::Path(path) = &body.expressions[*expression_handle] else {
                    unreachable!("reported on paths")
                };
                let segments = path.segments(db);
                let segment = |index: usize| segments[index].text(db).to_string();
                let span = span_of(*expression_handle);
                match *error {
                    ResolveError::Unresolved { .. } if segments.len() == 1 => {
                        SemanticDiagnostic::UnresolvedName {
                            name: segment(0),
                            span,
                        }
                    }
                    ResolveError::Unresolved { segment_index } => {
                        SemanticDiagnostic::UnresolvedModule {
                            name: segment(segment_index),
                            span,
                        }
                    }
                    ResolveError::MissingFromModule { segment_index } => {
                        SemanticDiagnostic::MissingFromModule {
                            name: segment(segment_index),
                            module: segment(segment_index - 1),
                            span,
                        }
                    }
                    ResolveError::NotAModule {
                        segment_index,
                        found,
                    } => SemanticDiagnostic::NotAModule {
                        name: segment(segment_index),
                        found: match found {
                            Resolution::Local(_) => "a local variable",
                            Resolution::Function(_) => "a function",
                            Resolution::Constant(_) => "a constant",
                            Resolution::Module(_) | Resolution::Error => {
                                unreachable!("a module, or a failed import, can be qualified")
                            }
                        },
                        span,
                    },
                }
            }
            Self::ExpectedValueFoundModule { expression_handle } => {
                SemanticDiagnostic::ExpectedValueFoundModule {
                    kind: value_kind(body, *expression_handle),
                    name: path_text(*expression_handle).unwrap(),
                    span: span_of(*expression_handle),
                }
            }
            Self::UnknownType { annotation_handle } => {
                let TypeAnnotation::Path(name) = &body.type_annotations[*annotation_handle] else {
                    unreachable!("only named annotations can be unknown")
                };
                SemanticDiagnostic::UnknownType {
                    name: name.text(db).to_string(),
                    span: source_map.type_annotations[*annotation_handle].span,
                }
            }
            Self::ExpectedFunction { call_handle, found } => {
                let Expression::Call { callee_handle, .. } = &body.expressions[*call_handle] else {
                    unreachable!("reported on calls")
                };
                SemanticDiagnostic::ExpectedFunction {
                    found: found.clone(),
                    callee_name: path_text(*callee_handle),
                    callee_span: span_of(*callee_handle),
                    call_span: span_of(*call_handle),
                }
            }
            Self::MismatchedArgumentCount {
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
                    .filter(|function| function.file(db) == owner.file)
                    .and_then(|function| function.to_ast_node(db).name())
                    .map(|name| name.span());
                SemanticDiagnostic::MismatchedArgumentCount {
                    expected: parameters.len(),
                    found: argument_handles.len(),
                    callee_span: span_of(*callee_handle),
                    argument_labels,
                    definition_span,
                }
            }
            Self::UnaryOperatorCannotBeApplied {
                expression_handle,
                operator,
                found,
                is_unsigned,
            } => SemanticDiagnostic::UnaryOperatorCannotBeApplied {
                operator: match operator {
                    UnaryOperator::Neg => "-".to_string(),
                    UnaryOperator::Not => "not".to_string(),
                },
                found: found.clone(),
                is_unsigned: *is_unsigned,
                span: span_of(*expression_handle),
            },
            Self::BinaryOperatorCannotBeApplied {
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
                SemanticDiagnostic::BinaryOperatorCannotBeApplied {
                    operator: *operator,
                    lhs: lhs.clone(),
                    rhs: rhs.clone(),
                    span: span_of(*expression_handle),
                    lhs_span: span_of(*lhs_handle),
                    rhs_span: span_of(*rhs_handle),
                }
            }
            Self::InvalidAssignmentTarget {
                assignment_handle,
                target_handle,
            } => SemanticDiagnostic::InvalidAssignmentTarget {
                span: span_of(*assignment_handle),
                target_span: span_of(*target_handle),
            },
            Self::BreakOutsideOfLoop {
                expression_handle,
                is_break,
                bad_value_break,
            } => {
                let span = span_of(*expression_handle);
                match (is_break, bad_value_break) {
                    (true, true) => SemanticDiagnostic::BreakWithValueFromWhile { span },
                    (true, false) => SemanticDiagnostic::BreakOutsideOfLoop { span },
                    (false, _) => SemanticDiagnostic::ContinueOutsideOfLoop { span },
                }
            }
            Self::ReturnOutsideFunction { expression_handle } => {
                SemanticDiagnostic::ReturnOutsideFunction {
                    span: span_of(*expression_handle),
                }
            }
            Self::TypeMustBeKnown { expression_handle } => SemanticDiagnostic::TypeMustBeKnown {
                span: span_of(*expression_handle),
            },
            Self::LiteralOutOfRange {
                expression_handle,
                ty,
                range,
                suggested_ty,
            } => SemanticDiagnostic::LiteralOutOfRange {
                literal: owner.file.contents(db)
                    [std::ops::Range::from(span_of(*expression_handle))]
                .to_string(),
                ty: ty.clone(),
                range: range.clone(),
                suggested_ty: suggested_ty.clone(),
                span: span_of(*expression_handle),
            },
        }
    }
}

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

fn value_kind(body: &DefinitionBody<'_>, expression_handle: ExpressionHandle) -> &'static str {
    let is_callee = body.expressions.values().any(|other| {
        matches!(other, Expression::Call { callee_handle, .. } if *callee_handle == expression_handle)
    });
    if is_callee { "function" } else { "value" }
}

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
