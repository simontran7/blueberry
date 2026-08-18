# Semantic Analyis

## Symbol Table

An identifier's **scope** is the part of a program where it's accessible. An identifier may refer to different values in different parts of the program. Crawfish has **static scope**, which means visibility is determined by *physical* location in the source code, at *compile-time*.

The **symbol table** maps identifiers to their semantic information. It is a stack of scopes, where each scope is a `HashMap<Symbol, BindingId>`. When entering a scope, the semantic analyzer pushes a new scope frame onto the stack, and when exiting, pops it. Lookup searches from the topmost frame downward, so inner bindings shadow outer ones.

```text
// Example: nested function that tries to access outer variable
func outer(x: i32) -> i32 {
    let y = 10;

    func inner() -> i32 {  // <- Push ScopeKind::FunctionBoundary here
        x + y  // Error: can't see x or y (they're locals from outer)
    }

    inner()
}
```

Each scope frame has an associated `ScopeKind` that tells lookup when to filter out locals. Most scopes are `ScopeKind::Normal`. When the semantic analyzer enters a nested function body, it pushes `ScopeKind::FunctionBoundary`. When it enters a constant definition's value expression, it pushes `ScopeKind::ConstantBoundary`. Lookups that cross either boundary stop seeing outer local bindings (`BindingKind::Local`) but continue seeing item bindings (`BindingKind::Item`).

`find_binding` returns `Result<BindingId, LookupError>`. `LookupError::NotFound` means the name doesn't exist. `LookupError::BlockedByBoundary(ScopeKind)` means the name exists as a local binding but a boundary blocks access, which lets the semantic analyzer emit a specific diagnostic (`CaptureInFunction` or `NonConstantValue`) depending on which boundary was crossed.

## Unification Table

The **unification table** stores substitutions during type inference. It is a disjoint-set forest with path compression and union by rank, giving amortized $O(\alpha(n))$ per operation.

Each equivalence class has an optional `TypeId` concrete slot. When a `var = ConcreteType` constraint is solved, the representative's slot is pinned to that type. A slot of `None` means the class is still unsolved; `Some(ty)` means the entire class has been resolved to `ty`.

The alternative (a flat `HashMap<TypeVarId, TypeId>`) has a chain-chasing problem: `?a = ?b` then `?b = I32` requires following `?a -> ?b -> I32`. With many constraints, chains grow arbitrarily deep and lookups become $O(n)$. Union-find avoids this.

## Three-Phase Algorithm

The semantic analyzer walks the AST and produces HIR nodes. Before walking function bodies, it passes over all top-level items, registering each name into the symbol table (the global scope frame) to enable forward references. The same kind of pass runs over items nested inside blocks.

After this pass, the semantic analyzer performs a recursive descent. For non-leaf AST nodes, within each `.typecheck_*()` method, the semantic analyzer either registers new names if required, or retrieves names from the symbol table. It will also type check the expression and create an HIR node with the resulting `TypeId`.

Type checking uses the **bidirectional typing** technique (Dunfield & Krishnaswami, [*Bidirectional Typing*](https://arxiv.org/abs/1908.05839)):
- **`check(expression, ty)`**: the expected type `ty` is known and *pushed down* into the expression. Used when context provides a type: function return positions, explicit annotations, `const` values, and call arguments.
- **`infer(expression)`**: no expected type is known; the type is *synthesized* from the expression's structure alone.

The checking mode is preferred when context is available because it produces better-localized error messages.

For each HIR node, the semantic analyzer either assigns a concrete type immediately or assigns a unification variable via `fresh_ty_var()` or `fresh_int_var` when the type cannot be determined immediately. Then, an **equality constraint** is recorded:

```rust,ignore
pub enum Constraint {
    Equality { expected: TypeId, actual: TypeId, provenance: Provenance },
}
```

`Provenance` records where the constraint came from, carrying enough span information to emit a precise diagnostic if unification later fails.

The semantic analyzer creates four kinds of scopes: source file (`Normal`), function body (`FunctionBoundary`), constant initializer (`ConstantBoundary`), and inner block (`Normal`). `FunctionBoundary` prevents nested functions from capturing locals of the enclosing function. `ConstantBoundary` prevents constant initializers from referencing local variables, since constants must be evaluable at compile time.

Phase 2 solves the constraints by finding a substitution (a mapping from each unification variable to a concrete type) that satisfies all equality constraints simultaneously. Each constraint is solved by calling `.unify()`, which implements Robinson's unification algorithm.

Before unifying, the semantic analyzer calls `.shallow_resolve()` on each side:

1. If the type is concrete (e.g. `I32`, `Bool`), return it immediately.
2. If the type is a unification variable, call `find` on the unification table to locate the representative.
3. If the representative has a concrete slot, recurse on it. Otherwise, intern the root variable as a `TypeId` and return it.

After shallow-resolving both sides, dispatch on their shapes:

| `expected` | `actual` | action |
|---|---|---|
| inference var | inference var | merge their equivalence classes |
| inference var | concrete type | pin the variable to the concrete type |
| concrete type | inference var | pin the variable to the concrete type |
| concrete type | concrete type | verify they are equal; emit `TypeMismatch` if not |

When generics and compound types arrive (e.g. `Func(A, B)`), unification will also need to recurse into subterms and add an **occurs check** to reject infinite types like `?a = List<?a>`.

After phase 2, every unification variable has been resolved. The HIR still holds placeholder `TypeId`s from phase 1. Phase 3 walks the HIR and replaces every placeholder with its resolved concrete type via `shallow_resolve`, covering expression nodes and local bindings. Item bindings never hold inference variables since `.collect_item_definition()` always resolves their types from explicit annotations.

Unresolved fallbacks:
- `IntVar` defaults to `I32`
- `TyVar` becomes `error_id`

After this phase, every HIR node has a concrete type and the HIR is complete.
