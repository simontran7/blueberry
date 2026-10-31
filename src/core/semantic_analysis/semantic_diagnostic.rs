use crate::core::common::diagnostic::{Diagnostic, DiagnosticLabel};
use crate::core::common::text_range::{AbsoluteTextRange, TextLength};
use crate::core::semantic_analysis::ast_lowering::nodes::{
    self, ExpressionHandle, HirNodeRanges, TypeExpressionHandle,
};
use crate::core::semantic_analysis::keys::FunctionKey;
use crate::core::semantic_analysis::name_resolution::definition_list::Definition;
use crate::core::semantic_analysis::name_resolution::resolver::{Resolution, ResolveError};
use crate::core::semantic_analysis::type_checking::inference_table::VariableOrigin;
use crate::core::semantic_analysis::type_checking::type_checker::BodySemantics;
use crate::core::semantic_analysis::type_checking::types::{Ty, TyKind};
use crate::core::syntactic_analysis::cst::ast::AstNode;

#[derive(Debug, Clone, PartialEq, Eq, salsa::SalsaValue)]
pub(crate) enum UnresolvedSemanticDiagnostic<'db> {
    TypeMismatch {
        expression_handle: ExpressionHandle,
        expected: Ty<'db>,
        found: Ty<'db>,
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
        type_expression_handle: TypeExpressionHandle,
    },
    UnknownField {
        expression_handle: ExpressionHandle,
        ty: Ty<'db>,
        is_primitive: bool,
    },
    ExpectedFunction {
        call_handle: ExpressionHandle,
        found: Ty<'db>,
    },
    MismatchedArgumentCount {
        call_handle: ExpressionHandle,
        function: Option<FunctionKey<'db>>,
        parameters: Box<[Ty<'db>]>,
    },
    UnaryOperatorCannotBeApplied {
        expression_handle: ExpressionHandle,
        operator: nodes::UnaryOperator,
        found: Ty<'db>,
        is_unsigned: bool,
    },
    BinaryOperatorCannotBeApplied {
        expression_handle: ExpressionHandle,
        operator: nodes::BinaryOperator,
        lhs: Ty<'db>,
        rhs: Ty<'db>,
    },
    InvalidAssignmentTarget {
        assignment_handle: ExpressionHandle,
        target_handle: ExpressionHandle,
    },
    BreakOutsideOfLoop {
        expression_handle: ExpressionHandle,
    },
    ContinueOutsideOfLoop {
        expression_handle: ExpressionHandle,
    },
    BreakWithValueFromWhile {
        expression_handle: ExpressionHandle,
    },
    ReturnOutsideFunction {
        expression_handle: ExpressionHandle,
    },
    TypeMustBeKnown {
        expression_handle: ExpressionHandle,
        origin: VariableOrigin,
    },
    LiteralOutOfRange {
        expression_handle: ExpressionHandle,
        ty: Ty<'db>,
        type_range: String,
        suggested_ty: Option<Ty<'db>>,
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

impl<'db> UnresolvedSemanticDiagnostic<'db> {
    pub(crate) fn resolve(
        &self,
        db: &'db dyn crate::Db,
        body: &nodes::Body<'db>,
        ranges: &HirNodeRanges,
        semantics: &BodySemantics<'db>,
        definition: Definition<'db>,
    ) -> Diagnostic {
        let anchor = definition.anchor(db);
        let range_of = |expression_handle: ExpressionHandle| {
            ranges.expression_absolute_range(expression_handle, anchor)
        };
        let path_text = |expression_handle: ExpressionHandle| match body
            .store()
            .expression(expression_handle)
        {
            nodes::Expression::Path(path) => Some(path.display(db).to_string()),
            _ => None,
        };
        match self {
            Self::TypeMismatch {
                expression_handle,
                expected,
                found,
                cause,
            } => {
                // only a function has a return type to point at
                let (function_name_range, return_type) = match definition {
                    Definition::Function(key) => {
                        let node = key.to_ast_node(db);
                        (
                            node.name().map(|name| name.range()),
                            node.return_type_annotation()
                                .map(|annotation| annotation.red().range()),
                        )
                    }
                    Definition::Constant(_) => (None, None),
                };
                let expected_type = expected.display(db).to_string();
                let found_type = found.display(db).to_string();
                match (*cause, return_type) {
                    (TypeMismatchCause::ImplicitUnitReturn, Some(return_type_range)) => {
                        let mut labels = vec![DiagnosticLabel::primary(
                            return_type_range,
                            expected_found(&expected_type, "()"),
                        )];
                        if let Some(function_name_range) = function_name_range {
                            labels.push(DiagnosticLabel::secondary(
                                function_name_range,
                                "implicitly returns `()` as its body has no tail or `return` expression",
                            ));
                        }
                        Diagnostic::new("E0308", "mismatched types", return_type_range, labels)
                    }
                    (TypeMismatchCause::IfElse { then_branch_handle }, _) => {
                        let then_range = range_of(value_expression(body, then_branch_handle));
                        let else_range = range_of(value_expression(body, *expression_handle));
                        Diagnostic::new(
                            "E0308",
                            "`if` and `else` have incompatible types",
                            else_range,
                            vec![
                                DiagnosticLabel::secondary(then_range, "expected because of this"),
                                DiagnosticLabel::primary(
                                    else_range,
                                    expected_found(&expected_type, &found_type),
                                ),
                            ],
                        )
                    }
                    (TypeMismatchCause::IfWithoutElse, _) => {
                        let range = range_of(*expression_handle);
                        Diagnostic {
                            notes: Box::new([
                                "`if` expressions without `else` evaluate to `()`".to_string()
                            ]),
                            helps: Box::new([
                                "consider adding an `else` block that evaluates to the expected type"
                                    .to_string(),
                            ]),
                            ..Diagnostic::new(
                                "E0317",
                                "`if` may be missing an `else` clause",
                                range,
                                vec![DiagnosticLabel::primary(range, expected_found(&expected_type, "()"))],
                            )
                        }
                    }
                    (cause, _) => {
                        let range = range_of(*expression_handle);
                        let return_type_range = match cause {
                            TypeMismatchCause::ReturnValue => return_type,
                            _ => None,
                        };
                        let tuple_lengths = match (expected.kind(db), found.kind(db)) {
                            (TyKind::Tuple(expected_elements), TyKind::Tuple(found_elements))
                                if expected_elements.len() != found_elements.len() =>
                            {
                                Some((expected_elements.len(), found_elements.len()))
                            }
                            _ => None,
                        };
                        // two tuples of different lengths are told apart by their lengths, which
                        // are easier to compare than the two types
                        let message = match tuple_lengths {
                            Some((expected_length, found_length)) => format!(
                                "expected a tuple with {expected_length} element{}, found one with \
                                 {found_length} element{}",
                                if expected_length == 1 { "" } else { "s" },
                                if found_length == 1 { "" } else { "s" },
                            ),
                            None => expected_found(&expected_type, &found_type),
                        };
                        let mut labels = vec![DiagnosticLabel::primary(range, message)];
                        if let Some(return_type_range) = return_type_range {
                            labels.push(DiagnosticLabel::secondary(
                                return_type_range,
                                format!(
                                    "expected {} because of return type",
                                    quote_type(&expected_type)
                                ),
                            ));
                        }
                        let notes = match tuple_lengths {
                            Some(_) => Box::new([format!(
                                "expected tuple {}, found tuple {}",
                                quote_type(&expected_type),
                                quote_type(&found_type)
                            )]) as Box<[String]>,
                            None => Box::new([]),
                        };
                        Diagnostic {
                            notes,
                            ..Diagnostic::new("E0308", "mismatched types", range, labels)
                        }
                    }
                }
            }
            Self::ReturnWithoutValue {
                expression_handle, ..
            } => {
                let range = range_of(*expression_handle);
                Diagnostic::new(
                    "E0069",
                    "`return;` in a function whose return type is not `()`",
                    range,
                    vec![DiagnosticLabel::primary(range, "return type is not `()`")],
                )
            }
            Self::UnresolvedPath {
                expression_handle,
                error,
            } => {
                let nodes::Expression::Path(path) = body.store().expression(*expression_handle)
                else {
                    unreachable!("reported on paths")
                };
                let segments = path.segments(db);
                let segment = |index: usize| segments[index].text(db).to_string();
                let range = range_of(*expression_handle);
                match *error {
                    ResolveError::Unresolved { .. } if segments.len() == 1 => {
                        let name = segment(0);
                        Diagnostic::new(
                            "E0425",
                            &format!("cannot find `{name}` in this scope"),
                            range,
                            vec![DiagnosticLabel::primary(range, "not found in this scope")],
                        )
                    }
                    ResolveError::Unresolved { segment_index } => {
                        let name = segment(segment_index);
                        let message = format!("use of unresolved module `{name}`");
                        Diagnostic::new(
                            "E0433",
                            &format!("failed to resolve: {message}"),
                            range,
                            vec![DiagnosticLabel::primary(range, message)],
                        )
                    }
                    ResolveError::MissingFromModule { segment_index } => {
                        let name = segment(segment_index);
                        let module = segment(segment_index - 1);
                        Diagnostic::new(
                            "E0425",
                            &format!("cannot find `{name}` in module `{module}`"),
                            range,
                            vec![DiagnosticLabel::primary(
                                range,
                                format!("not found in `{module}`"),
                            )],
                        )
                    }
                    ResolveError::NotAModule {
                        segment_index,
                        found,
                    } => {
                        let name = segment(segment_index);
                        let found = match found {
                            Resolution::Local(_) => "a local variable",
                            Resolution::Function(_) => "a function",
                            Resolution::Constant(_) => "a constant",
                            Resolution::Module(_) | Resolution::Error => {
                                unreachable!("a module, or a failed import, can be qualified")
                            }
                        };
                        let message = format!("`{name}` is {found}, not a module");
                        Diagnostic::new(
                            "E0433",
                            &format!("failed to resolve: {message}"),
                            range,
                            vec![DiagnosticLabel::primary(range, message)],
                        )
                    }
                }
            }
            Self::ExpectedValueFoundModule { expression_handle } => {
                let kind = value_kind(body, *expression_handle);
                let name = path_text(*expression_handle).unwrap();
                let range = range_of(*expression_handle);
                Diagnostic::new(
                    "E0423",
                    &format!("expected {kind}, found module `{name}`"),
                    range,
                    vec![DiagnosticLabel::primary(range, format!("not a {kind}"))],
                )
            }
            Self::UnknownType {
                type_expression_handle,
            } => {
                let nodes::TypeExpression::Path(path) =
                    body.store().type_expression(*type_expression_handle)
                else {
                    unreachable!("only path type expressions can be unknown")
                };
                unknown_type(
                    &path.display(db).to_string(),
                    ranges.type_expression_absolute_range(*type_expression_handle, anchor),
                )
            }
            Self::UnknownField {
                expression_handle,
                ty,
                is_primitive,
            } => {
                let nodes::Expression::Field {
                    name: Some(name), ..
                } = body.store().expression(*expression_handle)
                else {
                    unreachable!("reported on named fields")
                };
                let field = name.text(db).to_string();
                let end = range_of(*expression_handle).end();
                let range = AbsoluteTextRange::new(end - TextLength::new(field.len()), end);
                let ty = ty.display(db).to_string();
                if *is_primitive {
                    Diagnostic::new(
                        "E0610",
                        &format!("`{ty}` is a primitive type and therefore doesn't have fields"),
                        range,
                        vec![DiagnosticLabel::unlabeled(range)],
                    )
                } else {
                    Diagnostic::new(
                        "E0609",
                        &format!("no field `{field}` on type `{ty}`"),
                        range,
                        vec![DiagnosticLabel::primary(range, "unknown field")],
                    )
                }
            }
            Self::ExpectedFunction { call_handle, found } => {
                let nodes::Expression::Call { callee_handle, .. } =
                    body.store().expression(*call_handle)
                else {
                    unreachable!("reported on calls")
                };
                let found = found.display(db).to_string();
                let callee = match path_text(*callee_handle) {
                    Some(name) => format!("`{name}` has type `{found}`"),
                    None => format!("this expression has type `{found}`"),
                };
                let call_range = range_of(*call_handle);
                Diagnostic::new(
                    "E0618",
                    &format!("expected function, found `{found}`"),
                    call_range,
                    vec![
                        DiagnosticLabel::secondary(range_of(*callee_handle), callee),
                        DiagnosticLabel::primary(call_range, "call expression requires function"),
                    ],
                )
            }
            Self::MismatchedArgumentCount {
                call_handle,
                function,
                parameters,
            } => {
                let nodes::Expression::Call {
                    callee_handle,
                    argument_run,
                } = body.store().expression(*call_handle)
                else {
                    unreachable!("reported on calls")
                };
                let argument_handles = body.store().expression_handles(*argument_run);
                let callee_range = range_of(*callee_handle);
                let mut labels = vec![DiagnosticLabel::unlabeled(callee_range)];
                if parameters.len() < argument_handles.len() {
                    for (index, &argument_handle) in
                        argument_handles.iter().enumerate().skip(parameters.len())
                    {
                        let ty = semantics.expression_type(argument_handle).display(db);
                        labels.push(DiagnosticLabel::secondary(
                            range_of(argument_handle),
                            format!("unexpected argument #{} of type `{ty}`", index + 1),
                        ));
                    }
                } else {
                    for (index, parameter) in
                        parameters.iter().enumerate().skip(argument_handles.len())
                    {
                        let ty = parameter.display(db);
                        labels.push(DiagnosticLabel::secondary(
                            range_of(*call_handle),
                            format!("argument #{} of type `{ty}` is missing", index + 1),
                        ));
                    }
                }
                // Only a definition in the same file can be shown alongside the call.
                let definition_range = function
                    .filter(|function| function.file(db) == definition.file(db))
                    .and_then(|function| function.to_ast_node(db).name())
                    .map(|name| name.range());
                if let Some(definition_range) = definition_range {
                    labels.push(DiagnosticLabel::secondary(
                        definition_range,
                        "function defined here",
                    ));
                }
                Diagnostic::new(
                    "E0061",
                    &format!(
                        "this function takes {} but {} {} supplied",
                        count(parameters.len(), "argument"),
                        count(argument_handles.len(), "argument"),
                        if argument_handles.len() == 1 {
                            "was"
                        } else {
                            "were"
                        },
                    ),
                    callee_range,
                    labels,
                )
            }
            Self::UnaryOperatorCannotBeApplied {
                expression_handle,
                operator,
                found,
                is_unsigned,
            } => {
                let operator = match operator {
                    nodes::UnaryOperator::Negation => "-",
                    nodes::UnaryOperator::Not => "not",
                };
                let found = found.display(db);
                let range = range_of(*expression_handle);
                Diagnostic {
                    notes: if *is_unsigned {
                        Box::new(["unsigned values cannot be negated".to_string()])
                    } else {
                        Box::new([])
                    },
                    ..Diagnostic::new(
                        "E0600",
                        &format!("cannot apply unary operator `{operator}` to type `{found}`"),
                        range,
                        vec![DiagnosticLabel::primary(
                            range,
                            format!("cannot apply unary operator `{operator}`"),
                        )],
                    )
                }
            }
            Self::BinaryOperatorCannotBeApplied {
                expression_handle,
                operator,
                lhs,
                rhs,
            } => {
                let nodes::Expression::BinaryOperation {
                    lhs_handle,
                    rhs_handle,
                    ..
                } = body.store().expression(*expression_handle)
                else {
                    unreachable!("reported on binary operations")
                };
                let lhs = lhs.display(db).to_string();
                let rhs = rhs.display(db).to_string();
                let range = range_of(*expression_handle);
                let message = match operator {
                    nodes::BinaryOperator::Add => format!("cannot add `{rhs}` to `{lhs}`"),
                    nodes::BinaryOperator::Sub => format!("cannot subtract `{rhs}` from `{lhs}`"),
                    nodes::BinaryOperator::Mul => format!("cannot multiply `{lhs}` by `{rhs}`"),
                    nodes::BinaryOperator::Div => format!("cannot divide `{lhs}` by `{rhs}`"),
                    nodes::BinaryOperator::Mod => {
                        format!("cannot calculate the remainder of `{lhs}` divided by `{rhs}`")
                    }
                    _ => unreachable!("only arithmetic operators need integer operands"),
                };
                Diagnostic::new(
                    "E0369",
                    &message,
                    range,
                    vec![
                        DiagnosticLabel::unlabeled(range),
                        DiagnosticLabel::secondary(range_of(*lhs_handle), lhs),
                        DiagnosticLabel::secondary(range_of(*rhs_handle), rhs),
                    ],
                )
            }
            Self::InvalidAssignmentTarget {
                assignment_handle,
                target_handle,
            } => {
                let range = range_of(*assignment_handle);
                Diagnostic::new(
                    "E0070",
                    "invalid left-hand side of assignment",
                    range,
                    vec![
                        DiagnosticLabel::unlabeled(range),
                        DiagnosticLabel::secondary(
                            range_of(*target_handle),
                            "cannot assign to this expression",
                        ),
                    ],
                )
            }
            Self::BreakOutsideOfLoop { expression_handle } => {
                let range = range_of(*expression_handle);
                Diagnostic::new(
                    "E0268",
                    "`break` outside of a loop",
                    range,
                    vec![DiagnosticLabel::primary(
                        range,
                        "cannot `break` outside of a loop",
                    )],
                )
            }
            Self::ContinueOutsideOfLoop { expression_handle } => {
                let range = range_of(*expression_handle);
                Diagnostic::new(
                    "E0268",
                    "`continue` outside of a loop",
                    range,
                    vec![DiagnosticLabel::primary(
                        range,
                        "cannot `continue` outside of a loop",
                    )],
                )
            }
            Self::BreakWithValueFromWhile { expression_handle } => {
                let range = range_of(*expression_handle);
                Diagnostic {
                    helps: Box::new([
                        "use `break` on its own without a value inside this `while` loop"
                            .to_string(),
                    ]),
                    ..Diagnostic::new(
                        "E0571",
                        "`break` with value from a `while` loop",
                        range,
                        vec![DiagnosticLabel::primary(
                            range,
                            "can only break with a value inside `loop`",
                        )],
                    )
                }
            }
            Self::ReturnOutsideFunction { expression_handle } => {
                let range = range_of(*expression_handle);
                Diagnostic::new(
                    "E0572",
                    "return statement outside of function body",
                    range,
                    vec![DiagnosticLabel::unlabeled(range)],
                )
            }
            Self::TypeMustBeKnown {
                expression_handle,
                origin,
            } => {
                let range = range_of(*expression_handle);
                match origin {
                    VariableOrigin::LocalBinding(binding_handle) => {
                        let name = body.store().local_binding(*binding_handle).name().text(db);
                        let origin_range =
                            ranges.local_binding_absolute_range(*binding_handle, anchor);
                        Diagnostic::new(
                            "E0282",
                            "type annotations needed",
                            origin_range,
                            vec![
                                DiagnosticLabel::primary(
                                    origin_range,
                                    format!("consider giving `{name}` an explicit type"),
                                ),
                                DiagnosticLabel::secondary(
                                    range,
                                    "type must be known at this point",
                                ),
                            ],
                        )
                    }
                    VariableOrigin::Expression(origin_handle) => {
                        let origin_range = range_of(*origin_handle);
                        let mut labels = vec![DiagnosticLabel::primary(
                            range,
                            "type must be known at this point",
                        )];
                        if origin_range != range {
                            labels.push(DiagnosticLabel::secondary(
                                origin_range,
                                "cannot infer the type of this expression",
                            ));
                        }
                        Diagnostic::new("E0282", "type annotations needed", range, labels)
                    }
                }
            }
            Self::LiteralOutOfRange {
                expression_handle,
                ty,
                type_range,
                suggested_ty,
            } => {
                let range = range_of(*expression_handle);
                let literal = &definition.file(db).contents(db)[std::ops::Range::from(range)];
                let ty = ty.display(db);
                Diagnostic {
                    notes: Box::new([format!(
                        "the literal `{literal}` does not fit into the type `{ty}` whose range is `{type_range}`"
                    )]),
                    helps: suggested_ty
                        .iter()
                        .map(|suggested_ty| {
                            format!(
                                "consider using the type `{}` instead",
                                suggested_ty.display(db)
                            )
                        })
                        .collect(),
                    ..Diagnostic::new(
                        "E0103",
                        &format!("literal out of range for `{ty}`"),
                        range,
                        vec![DiagnosticLabel::unlabeled(range)],
                    )
                }
            }
        }
    }
}

/// An unknown type can be in a signature or in a body, which are reported from different places.
pub(crate) fn unknown_type(name: &str, range: AbsoluteTextRange) -> Diagnostic {
    Diagnostic::new(
        "E0412",
        &format!("cannot find type `{name}` in this scope"),
        range,
        vec![DiagnosticLabel::primary(range, "not found in this scope")],
    )
}

pub(crate) fn unresolved_import(path: &[String], range: AbsoluteTextRange) -> Diagnostic {
    let label = match path.split_last() {
        Some((name, [])) => format!("no `{name}` in the root"),
        Some((name, parent)) => format!("no `{name}` in `{}`", parent.join("::")),
        None => unreachable!("an import path has at least one segment"),
    };
    Diagnostic::new(
        "E0432",
        &format!("unresolved import `{}`", path.join("::")),
        range,
        vec![DiagnosticLabel::primary(range, label)],
    )
}

fn value_expression(
    body: &nodes::Body<'_>,
    expression_handle: ExpressionHandle,
) -> ExpressionHandle {
    match body.store().expression(expression_handle) {
        nodes::Expression::Block {
            tail_handle: Some(tail_handle),
            ..
        } => value_expression(body, *tail_handle),
        _ => expression_handle,
    }
}

fn value_kind(body: &nodes::Body<'_>, expression_handle: ExpressionHandle) -> &'static str {
    let is_callee = body.store().expressions().map(|(_, expression)| expression).any(|other| {
        matches!(other, nodes::Expression::Call { callee_handle, .. } if *callee_handle == expression_handle)
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
