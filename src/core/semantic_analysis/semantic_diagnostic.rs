use crate::core::common::diagnostic::{DiagnosticDescription, DiagnosticLabel, LabelSeverity};
use crate::core::common::text_range::{AbsoluteTextRange, TextLength, TextOffset};
use crate::core::semantic_analysis::ast_lowering::nodes::{
    self, ExpressionHandle, HirNodeSourceMap, TypeExpressionHandle,
};
use crate::core::semantic_analysis::keys::FunctionKey;
use crate::core::semantic_analysis::name_resolution::resolver::{Resolution, ResolveError};
use crate::core::semantic_analysis::type_checking::inference_table::VariableOrigin;
use crate::core::semantic_analysis::type_checking::type_checker::BodySemantics;
use crate::core::semantic_analysis::type_checking::types::{Ty, TyKind};
use crate::core::source_file_key::SourceFileKey;

#[derive(Debug, Clone, PartialEq, Eq, salsa::SalsaValue)]
pub(crate) enum SemanticDiagnostic {
    TypeMismatch {
        expected: String,
        found: String,
        range: AbsoluteTextRange,
        return_type_range: Option<AbsoluteTextRange>,
        /// the number of elements of each, when both are tuples of different lengths
        tuple_lengths: Option<(usize, usize)>,
    },
    ImplicitUnitReturn {
        expected: String,
        return_type_range: AbsoluteTextRange,
        function_name_range: Option<AbsoluteTextRange>,
    },
    IfElseMismatch {
        expected: String,
        found: String,
        then_range: AbsoluteTextRange,
        else_range: AbsoluteTextRange,
    },
    IfWithoutElse {
        expected: String,
        range: AbsoluteTextRange,
    },
    ReturnWithoutValue {
        range: AbsoluteTextRange,
    },
    MismatchedArgumentCount {
        expected: usize,
        found: usize,
        callee_range: AbsoluteTextRange,
        argument_labels: Box<[(AbsoluteTextRange, String)]>,
        definition_range: Option<AbsoluteTextRange>,
    },
    UnresolvedName {
        name: String,
        range: AbsoluteTextRange,
    },
    UnresolvedModule {
        name: String,
        range: AbsoluteTextRange,
    },
    UnresolvedImport {
        path: Box<[String]>,
        range: AbsoluteTextRange,
    },
    MissingFromModule {
        name: String,
        module: String,
        range: AbsoluteTextRange,
    },
    NotAModule {
        name: String,
        found: &'static str,
        range: AbsoluteTextRange,
    },
    ExpectedValueFoundModule {
        kind: &'static str,
        name: String,
        range: AbsoluteTextRange,
    },
    UnknownType {
        name: String,
        range: AbsoluteTextRange,
    },
    UnknownField {
        field: String,
        ty: String,
        range: AbsoluteTextRange,
    },
    PrimitiveHasNoFields {
        ty: String,
        range: AbsoluteTextRange,
    },
    ExpectedFunction {
        found: String,
        callee_name: Option<String>,
        callee_range: AbsoluteTextRange,
        call_range: AbsoluteTextRange,
    },
    UnaryOperatorCannotBeApplied {
        operator: String,
        found: String,
        is_unsigned: bool,
        range: AbsoluteTextRange,
    },
    BinaryOperatorCannotBeApplied {
        operator: nodes::BinaryOperator,
        lhs: String,
        rhs: String,
        range: AbsoluteTextRange,
        lhs_range: AbsoluteTextRange,
        rhs_range: AbsoluteTextRange,
    },
    InvalidAssignmentTarget {
        range: AbsoluteTextRange,
        target_range: AbsoluteTextRange,
    },
    ReturnOutsideFunction {
        range: AbsoluteTextRange,
    },
    BreakOutsideOfLoop {
        range: AbsoluteTextRange,
    },
    ContinueOutsideOfLoop {
        range: AbsoluteTextRange,
    },
    BreakWithValueFromWhile {
        range: AbsoluteTextRange,
    },
    TypeMustBeKnown {
        /// where the type was needed
        range: AbsoluteTextRange,
        /// the name of the `let` binding whose type is unknown, if that's what it is
        binding_name: Option<String>,
        /// what the unknown type belongs to: the `let` binding, or an expression
        origin_range: AbsoluteTextRange,
    },
    LiteralOutOfRange {
        literal: String,
        ty: String,
        type_range: String,
        suggested_ty: Option<String>,
        range: AbsoluteTextRange,
    },
    MainNotFound {
        file: String,
        range: AbsoluteTextRange,
    },
    MainWrongType {
        expected: String,
        found: String,
        range: AbsoluteTextRange,
    },
}

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

pub(crate) struct OwnerSpans {
    pub(crate) file: SourceFileKey,
    pub(crate) anchor: TextOffset,
    pub(crate) name: Option<AbsoluteTextRange>,
    pub(crate) return_type: Option<AbsoluteTextRange>,
}

impl SemanticDiagnostic {
    pub(crate) fn describe(&self) -> DiagnosticDescription {
        match self {
            Self::TypeMismatch {
                expected,
                found,
                range,
                return_type_range,
                tuple_lengths,
            } => {
                // two tuples of different lengths are told apart by their lengths, which are
                // easier to compare than the two types
                let message = match tuple_lengths {
                    Some((expected_length, found_length)) => format!(
                        "expected a tuple with {expected_length} element{}, found one with \
                         {found_length} element{}",
                        if *expected_length == 1 { "" } else { "s" },
                        if *found_length == 1 { "" } else { "s" },
                    ),
                    None => expected_found(expected, found),
                };
                let mut labels = vec![primary(*range, message)];
                if let Some(return_type_range) = return_type_range {
                    labels.push(secondary(
                        *return_type_range,
                        format!("expected {} because of return type", quote_type(expected)),
                    ));
                }
                let notes = match tuple_lengths {
                    Some(_) => Box::new([format!(
                        "expected tuple {}, found tuple {}",
                        quote_type(expected),
                        quote_type(found)
                    )]) as Box<[String]>,
                    None => Box::new([]),
                };
                DiagnosticDescription {
                    notes,
                    ..description("E0308", "mismatched types", *range, labels)
                }
            }
            Self::ImplicitUnitReturn {
                expected,
                return_type_range,
                function_name_range,
            } => {
                let mut labels = vec![primary(*return_type_range, expected_found(expected, "()"))];
                if let Some(function_name_range) = function_name_range {
                    labels.push(secondary(
                        *function_name_range,
                        "implicitly returns `()` as its body has no tail or `return` expression",
                    ));
                }
                description("E0308", "mismatched types", *return_type_range, labels)
            }
            Self::IfElseMismatch {
                expected,
                found,
                then_range,
                else_range,
            } => description(
                "E0308",
                "`if` and `else` have incompatible types",
                *else_range,
                vec![
                    secondary(*then_range, "expected because of this"),
                    primary(*else_range, expected_found(expected, found)),
                ],
            ),
            Self::IfWithoutElse { expected, range } => DiagnosticDescription {
                notes: Box::new(["`if` expressions without `else` evaluate to `()`".to_string()]),
                helps: Box::new([
                    "consider adding an `else` block that evaluates to the expected type"
                        .to_string(),
                ]),
                ..description(
                    "E0317",
                    "`if` may be missing an `else` clause",
                    *range,
                    vec![primary(*range, expected_found(expected, "()"))],
                )
            },
            Self::ReturnWithoutValue { range } => description(
                "E0069",
                "`return;` in a function whose return type is not `()`",
                *range,
                vec![primary(*range, "return type is not `()`")],
            ),
            Self::MismatchedArgumentCount {
                expected,
                found,
                callee_range,
                argument_labels,
                definition_range,
            } => {
                let mut labels = vec![unlabeled(*callee_range)];
                labels.extend(
                    argument_labels
                        .iter()
                        .map(|(range, message)| secondary(*range, message.clone())),
                );
                if let Some(definition_range) = definition_range {
                    labels.push(secondary(*definition_range, "function defined here"));
                }
                description(
                    "E0061",
                    &format!(
                        "this function takes {} but {} {} supplied",
                        count(*expected, "argument"),
                        count(*found, "argument"),
                        if *found == 1 { "was" } else { "were" },
                    ),
                    *callee_range,
                    labels,
                )
            }
            Self::UnresolvedName { name, range } => description(
                "E0425",
                &format!("cannot find `{name}` in this scope"),
                *range,
                vec![primary(*range, "not found in this scope")],
            ),
            Self::UnresolvedModule { name, range } => {
                let message = format!("use of unresolved module `{name}`");
                description(
                    "E0433",
                    &format!("failed to resolve: {message}"),
                    *range,
                    vec![primary(*range, message)],
                )
            }
            Self::MissingFromModule {
                name,
                module,
                range,
            } => description(
                "E0425",
                &format!("cannot find `{name}` in module `{module}`"),
                *range,
                vec![primary(*range, format!("not found in `{module}`"))],
            ),
            Self::UnresolvedImport { path, range } => {
                let label = match path.split_last() {
                    Some((name, [])) => format!("no `{name}` in the root"),
                    Some((name, parent)) => format!("no `{name}` in `{}`", parent.join("::")),
                    None => unreachable!("an import path has at least one segment"),
                };
                description(
                    "E0432",
                    &format!("unresolved import `{}`", path.join("::")),
                    *range,
                    vec![primary(*range, label)],
                )
            }
            Self::NotAModule { name, found, range } => {
                let message = format!("`{name}` is {found}, not a module");
                description(
                    "E0433",
                    &format!("failed to resolve: {message}"),
                    *range,
                    vec![primary(*range, message)],
                )
            }
            Self::ExpectedValueFoundModule { kind, name, range } => description(
                "E0423",
                &format!("expected {kind}, found module `{name}`"),
                *range,
                vec![primary(*range, format!("not a {kind}"))],
            ),
            Self::UnknownType { name, range } => description(
                "E0412",
                &format!("cannot find type `{name}` in this scope"),
                *range,
                vec![primary(*range, "not found in this scope")],
            ),
            Self::UnknownField { field, ty, range } => description(
                "E0609",
                &format!("no field `{field}` on type `{ty}`"),
                *range,
                vec![primary(*range, "unknown field")],
            ),
            Self::PrimitiveHasNoFields { ty, range } => description(
                "E0610",
                &format!("`{ty}` is a primitive type and therefore doesn't have fields"),
                *range,
                vec![unlabeled(*range)],
            ),
            Self::ExpectedFunction {
                found,
                callee_name,
                callee_range,
                call_range,
            } => {
                let callee = match callee_name {
                    Some(name) => format!("`{name}` has type `{found}`"),
                    None => format!("this expression has type `{found}`"),
                };
                description(
                    "E0618",
                    &format!("expected function, found `{found}`"),
                    *call_range,
                    vec![
                        secondary(*callee_range, callee),
                        primary(*call_range, "call expression requires function"),
                    ],
                )
            }
            Self::UnaryOperatorCannotBeApplied {
                operator,
                found,
                is_unsigned,
                range,
            } => DiagnosticDescription {
                notes: if *is_unsigned {
                    Box::new(["unsigned values cannot be negated".to_string()])
                } else {
                    Box::new([])
                },
                ..description(
                    "E0600",
                    &format!("cannot apply unary operator `{operator}` to type `{found}`"),
                    *range,
                    vec![primary(
                        *range,
                        format!("cannot apply unary operator `{operator}`"),
                    )],
                )
            },
            Self::BinaryOperatorCannotBeApplied {
                operator,
                lhs,
                rhs,
                range,
                lhs_range,
                rhs_range,
            } => {
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
                description(
                    "E0369",
                    &message,
                    *range,
                    vec![
                        unlabeled(*range),
                        secondary(*lhs_range, lhs.clone()),
                        secondary(*rhs_range, rhs.clone()),
                    ],
                )
            }
            Self::InvalidAssignmentTarget {
                range,
                target_range,
            } => description(
                "E0070",
                "invalid left-hand side of assignment",
                *range,
                vec![
                    unlabeled(*range),
                    secondary(*target_range, "cannot assign to this expression"),
                ],
            ),
            Self::ReturnOutsideFunction { range } => description(
                "E0572",
                "return statement outside of function body",
                *range,
                vec![unlabeled(*range)],
            ),
            Self::BreakOutsideOfLoop { range } => description(
                "E0268",
                "`break` outside of a loop",
                *range,
                vec![primary(*range, "cannot `break` outside of a loop")],
            ),
            Self::ContinueOutsideOfLoop { range } => description(
                "E0268",
                "`continue` outside of a loop",
                *range,
                vec![primary(*range, "cannot `continue` outside of a loop")],
            ),
            Self::BreakWithValueFromWhile { range } => DiagnosticDescription {
                helps: Box::new([
                    "use `break` on its own without a value inside this `while` loop".to_string(),
                ]),
                ..description(
                    "E0571",
                    "`break` with value from a `while` loop",
                    *range,
                    vec![primary(*range, "can only break with a value inside `loop`")],
                )
            },
            Self::TypeMustBeKnown {
                range,
                binding_name,
                origin_range,
            } => {
                if let Some(name) = binding_name {
                    description(
                        "E0282",
                        "type annotations needed",
                        *origin_range,
                        vec![
                            primary(
                                *origin_range,
                                format!("consider giving `{name}` an explicit type"),
                            ),
                            secondary(*range, "type must be known at this point"),
                        ],
                    )
                } else {
                    let mut labels = vec![primary(*range, "type must be known at this point")];
                    if origin_range != range {
                        labels.push(secondary(
                            *origin_range,
                            "cannot infer the type of this expression",
                        ));
                    }
                    description("E0282", "type annotations needed", *range, labels)
                }
            }
            Self::MainNotFound { file, range } => description(
                "E0601",
                &format!("`main` function not found in `{file}`"),
                *range,
                vec![primary(
                    *range,
                    format!("consider adding a `main` function to `{file}`"),
                )],
            ),
            Self::MainWrongType {
                expected,
                found,
                range,
            } => description(
                "E0580",
                "`main` function has wrong type",
                *range,
                vec![primary(*range, expected_found(expected, found))],
            ),
            Self::LiteralOutOfRange {
                literal,
                ty,
                type_range,
                suggested_ty,
                range,
            } => DiagnosticDescription {
                notes: Box::new([format!(
                    "the literal `{literal}` does not fit into the type `{ty}` whose range is `{type_range}`"
                )]),
                helps: suggested_ty
                    .iter()
                    .map(|suggested_ty| format!("consider using the type `{suggested_ty}` instead"))
                    .collect(),
                ..description(
                    "E0103",
                    &format!("literal out of range for `{ty}`"),
                    *range,
                    vec![unlabeled(*range)],
                )
            },
        }
    }
}

impl<'db> UnresolvedSemanticDiagnostic<'db> {
    pub(crate) fn resolve(
        &self,
        db: &'db dyn crate::Db,
        body: &nodes::Body<'db>,
        source_map: &HirNodeSourceMap,
        semantics: &BodySemantics<'db>,
        owner: &OwnerSpans,
    ) -> SemanticDiagnostic {
        let range_of = |expression_handle: ExpressionHandle| {
            source_map.expression_range(expression_handle, owner.anchor)
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
            } => match (*cause, owner.return_type) {
                (TypeMismatchCause::ImplicitUnitReturn, Some(return_type_range)) => {
                    SemanticDiagnostic::ImplicitUnitReturn {
                        expected: expected.display(db).to_string(),
                        return_type_range,
                        function_name_range: owner.name,
                    }
                }
                (TypeMismatchCause::IfElse { then_branch_handle }, _) => {
                    SemanticDiagnostic::IfElseMismatch {
                        expected: expected.display(db).to_string(),
                        found: found.display(db).to_string(),
                        then_range: range_of(value_expression(body, then_branch_handle)),
                        else_range: range_of(value_expression(body, *expression_handle)),
                    }
                }
                (TypeMismatchCause::IfWithoutElse, _) => SemanticDiagnostic::IfWithoutElse {
                    expected: expected.display(db).to_string(),
                    range: range_of(*expression_handle),
                },
                (cause, _) => SemanticDiagnostic::TypeMismatch {
                    expected: expected.display(db).to_string(),
                    found: found.display(db).to_string(),
                    range: range_of(*expression_handle),
                    return_type_range: match cause {
                        TypeMismatchCause::ReturnValue => owner.return_type,
                        _ => None,
                    },
                    tuple_lengths: match (expected.kind(db), found.kind(db)) {
                        (TyKind::Tuple(expected_elements), TyKind::Tuple(found_elements))
                            if expected_elements.len() != found_elements.len() =>
                        {
                            Some((expected_elements.len(), found_elements.len()))
                        }
                        _ => None,
                    },
                },
            },
            Self::ReturnWithoutValue {
                expression_handle, ..
            } => SemanticDiagnostic::ReturnWithoutValue {
                range: range_of(*expression_handle),
            },
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
                        SemanticDiagnostic::UnresolvedName {
                            name: segment(0),
                            range,
                        }
                    }
                    ResolveError::Unresolved { segment_index } => {
                        SemanticDiagnostic::UnresolvedModule {
                            name: segment(segment_index),
                            range,
                        }
                    }
                    ResolveError::MissingFromModule { segment_index } => {
                        SemanticDiagnostic::MissingFromModule {
                            name: segment(segment_index),
                            module: segment(segment_index - 1),
                            range,
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
                        range,
                    },
                }
            }
            Self::ExpectedValueFoundModule { expression_handle } => {
                SemanticDiagnostic::ExpectedValueFoundModule {
                    kind: value_kind(body, *expression_handle),
                    name: path_text(*expression_handle).unwrap(),
                    range: range_of(*expression_handle),
                }
            }
            Self::UnknownType {
                type_expression_handle,
            } => {
                let nodes::TypeExpression::Path(path) =
                    body.store().type_expression(*type_expression_handle)
                else {
                    unreachable!("only path type expressions can be unknown")
                };
                SemanticDiagnostic::UnknownType {
                    name: path.display(db).to_string(),
                    range: source_map.type_expression_range(*type_expression_handle, owner.anchor),
                }
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
                if *is_primitive {
                    SemanticDiagnostic::PrimitiveHasNoFields {
                        ty: ty.display(db).to_string(),
                        range,
                    }
                } else {
                    SemanticDiagnostic::UnknownField {
                        field,
                        ty: ty.display(db).to_string(),
                        range,
                    }
                }
            }
            Self::ExpectedFunction { call_handle, found } => {
                let nodes::Expression::Call { callee_handle, .. } =
                    body.store().expression(*call_handle)
                else {
                    unreachable!("reported on calls")
                };
                SemanticDiagnostic::ExpectedFunction {
                    found: found.display(db).to_string(),
                    callee_name: path_text(*callee_handle),
                    callee_range: range_of(*callee_handle),
                    call_range: range_of(*call_handle),
                }
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
                let argument_labels = if parameters.len() < argument_handles.len() {
                    (parameters.len()..argument_handles.len())
                        .map(|index| {
                            let argument_handle = argument_handles[index];
                            let ty = semantics.expression_type(argument_handle).display(db);
                            (
                                range_of(argument_handle),
                                format!("unexpected argument #{} of type `{ty}`", index + 1),
                            )
                        })
                        .collect()
                } else {
                    (argument_handles.len()..parameters.len())
                        .map(|index| {
                            let ty = parameters[index].display(db);
                            (
                                range_of(*call_handle),
                                format!("argument #{} of type `{ty}` is missing", index + 1),
                            )
                        })
                        .collect()
                };
                // Only a definition in the same file can be shown alongside the call.
                let definition_range = function
                    .filter(|function| function.file(db) == owner.file)
                    .and_then(|function| function.to_ast_node(db).name())
                    .map(|name| name.range());
                SemanticDiagnostic::MismatchedArgumentCount {
                    expected: parameters.len(),
                    found: argument_handles.len(),
                    callee_range: range_of(*callee_handle),
                    argument_labels,
                    definition_range,
                }
            }
            Self::UnaryOperatorCannotBeApplied {
                expression_handle,
                operator,
                found,
                is_unsigned,
            } => SemanticDiagnostic::UnaryOperatorCannotBeApplied {
                operator: match operator {
                    nodes::UnaryOperator::Negation => "-".to_string(),
                    nodes::UnaryOperator::Not => "not".to_string(),
                },
                found: found.display(db).to_string(),
                is_unsigned: *is_unsigned,
                range: range_of(*expression_handle),
            },
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
                SemanticDiagnostic::BinaryOperatorCannotBeApplied {
                    operator: *operator,
                    lhs: lhs.display(db).to_string(),
                    rhs: rhs.display(db).to_string(),
                    range: range_of(*expression_handle),
                    lhs_range: range_of(*lhs_handle),
                    rhs_range: range_of(*rhs_handle),
                }
            }
            Self::InvalidAssignmentTarget {
                assignment_handle,
                target_handle,
            } => SemanticDiagnostic::InvalidAssignmentTarget {
                range: range_of(*assignment_handle),
                target_range: range_of(*target_handle),
            },
            Self::BreakOutsideOfLoop { expression_handle } => {
                SemanticDiagnostic::BreakOutsideOfLoop {
                    range: range_of(*expression_handle),
                }
            }
            Self::ContinueOutsideOfLoop { expression_handle } => {
                SemanticDiagnostic::ContinueOutsideOfLoop {
                    range: range_of(*expression_handle),
                }
            }
            Self::BreakWithValueFromWhile { expression_handle } => {
                SemanticDiagnostic::BreakWithValueFromWhile {
                    range: range_of(*expression_handle),
                }
            }
            Self::ReturnOutsideFunction { expression_handle } => {
                SemanticDiagnostic::ReturnOutsideFunction {
                    range: range_of(*expression_handle),
                }
            }
            Self::TypeMustBeKnown {
                expression_handle,
                origin,
            } => {
                let (binding_name, origin_range) = match origin {
                    VariableOrigin::LocalBinding(binding_handle) => (
                        Some(
                            body.store()
                                .local_binding(*binding_handle)
                                .name()
                                .text(db)
                                .to_string(),
                        ),
                        source_map.local_binding_range(*binding_handle, owner.anchor),
                    ),
                    VariableOrigin::Expression(origin_handle) => (None, range_of(*origin_handle)),
                };
                SemanticDiagnostic::TypeMustBeKnown {
                    range: range_of(*expression_handle),
                    binding_name,
                    origin_range,
                }
            }
            Self::LiteralOutOfRange {
                expression_handle,
                ty,
                type_range,
                suggested_ty,
            } => SemanticDiagnostic::LiteralOutOfRange {
                literal: owner.file.contents(db)
                    [std::ops::Range::from(range_of(*expression_handle))]
                .to_string(),
                ty: ty.display(db).to_string(),
                type_range: type_range.clone(),
                suggested_ty: suggested_ty.map(|suggested_ty| suggested_ty.display(db).to_string()),
                range: range_of(*expression_handle),
            },
        }
    }
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

fn description(
    code: &'static str,
    message: &str,
    range: AbsoluteTextRange,
    labels: Vec<DiagnosticLabel>,
) -> DiagnosticDescription {
    DiagnosticDescription {
        code,
        message: message.to_string(),
        range,
        labels: labels.into_boxed_slice(),
        notes: Box::new([]),
        helps: Box::new([]),
    }
}

fn primary(range: AbsoluteTextRange, message: impl Into<String>) -> DiagnosticLabel {
    DiagnosticLabel {
        range,
        message: Some(message.into()),
        severity: LabelSeverity::Primary,
    }
}

fn secondary(range: AbsoluteTextRange, message: impl Into<String>) -> DiagnosticLabel {
    DiagnosticLabel {
        range,
        message: Some(message.into()),
        severity: LabelSeverity::Secondary,
    }
}

fn unlabeled(range: AbsoluteTextRange) -> DiagnosticLabel {
    DiagnosticLabel {
        range,
        message: None,
        severity: LabelSeverity::Primary,
    }
}
