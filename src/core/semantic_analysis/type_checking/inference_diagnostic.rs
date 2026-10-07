use crate::core::semantic_analysis::hir::nodes::{
    BinaryOperator, ExpressionHandle, TypeAnnotationHandle, UnaryOperator,
};
use crate::core::semantic_analysis::ids::keys::FunctionKey;

/// A problem found while type checking one body. It points at HIR handles rather than source
/// spans: the spans are only looked up when the diagnostic is reported (see
/// `semantic_diagnostic.rs`).
///
/// Types are stored already rendered, as they were known when the problem was found: e.g. an
/// integer literal whose type wasn't known yet is shown as `{integer}`, even if it's solved
/// later.
#[derive(Debug, Clone, PartialEq, Eq, salsa::SalsaValue)]
pub(crate) enum InferenceDiagnostic<'db> {
    /// `expression_handle` has type `found`, where `expected` was required. At most one per expression.
    TypeMismatch {
        expression_handle: ExpressionHandle,
        expected: String,
        found: String,
        cause: MismatchCause,
    },
    /// The `else` branch's type doesn't match the `then` branch's.
    IfElseMismatch {
        then_branch_handle: ExpressionHandle,
        else_branch_handle: ExpressionHandle,
        expected: String,
        found: String,
    },
    /// An `if` without `else` where a value of type `expected` is needed.
    IfWithoutElse {
        expression_handle: ExpressionHandle,
        expected: String,
    },
    /// A `return` without a value in a function that doesn't return `()`.
    ReturnWithoutValue { expression_handle: ExpressionHandle },
    /// A path that names no local binding nor definition.
    UnresolvedName { expression_handle: ExpressionHandle },
    /// A type annotation inside the body naming no type.
    UnknownType {
        annotation_handle: TypeAnnotationHandle,
    },
    /// A call whose callee isn't a function.
    ExpectedFunction {
        call_handle: ExpressionHandle,
        found: String,
    },
    /// A call passing a different number of arguments than the function's `parameters`.
    MismatchedArgumentCount {
        call_handle: ExpressionHandle,
        /// The called function, when the callee names one.
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
    /// The left-hand side of an assignment isn't a local binding.
    InvalidAssignmentTarget {
        assignment_handle: ExpressionHandle,
        target_handle: ExpressionHandle,
    },
    /// A `break` or `continue` outside of any loop, or a `break` with a value out of a `while`
    /// loop (`bad_value_break`).
    BreakOutsideOfLoop {
        expression_handle: ExpressionHandle,
        is_break: bool,
        bad_value_break: bool,
    },
    /// A `return` in a constant's body.
    ReturnOutsideFunction { expression_handle: ExpressionHandle },
    /// `expression_handle`'s type still had an unsolved inference variable once the body was checked.
    TypeMustBeKnown { expression_handle: ExpressionHandle },
}

/// Why a type was expected, so that a mismatch can point at where the expectation comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, salsa::SalsaValue)]
pub(crate) enum MismatchCause {
    Other,
    /// The expression is the function's result: its body's tail, or a `return`'s value.
    ReturnValue,
    /// The function's body has no tail, so it returns `()` where a value is expected.
    ImplicitUnitReturn,
}
