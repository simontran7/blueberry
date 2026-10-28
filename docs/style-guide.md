# Style Guide

## Assertions

Use `assert!` by default. Only use `debug_assert!` if you have benchmarked the code and identified that a specific assertion causes a measurable performance regression (for example, a costly check inside a hot loop).

This is because assertions are almost never a detectable performance hit, while a well-placed `assert!` can even _improve_ performance, because the compiler may use it to eliminate downstream redundant checks (a common trick for helping autovectorization: asserting slice lengths before a loop removes per-iteration bounds checks).

More importantly, failing loudly in release builds is better than failing silently. A compiler that panics with a clear message is better than one that produces wrong output.

## Naming Conventions for Handles

- the core type: `TYPE`
- the handle: `TYPEHandle`
- the view: `TYPEView`
- a run (a contiguous range) of items inside a `RunList<TYPE>` (of handles, or of plain values): `Run<TYPE>`
- a variable, parameter, or struct field that's a handle: `<description>_handle`
- a variable, parameter, or struct field whose type is a `Run`: `<description>_run`
- a variable, parameter, or struct field whose type is an _actual_ slice of handles or a vector of handles: `<description>_handles`
- a variable, parameter, or struct field whose type is a view: `<description>_view`

## Naming by Kind and by Role

Name a type after what a thing _is_ (its kind), and a field after the part it plays (its role). For example, `: I32` after a `let` is a `TypeExpression` (its kind) used as the `let`'s annotation (its role), so the field is `annotation_handle`, while a type expression nested inside a tuple, which isn't an annotation, is just a `type_expression_handle`. Likewise, a function's `body` is a `Block`, and a binary operation's `operator` is a token.

## Collections

A field of a finished value is a `Box<[T]>` (e.g., a `Body`'s `parameter_handles`, a tuple type's elements, a path's segments): it's allocated to its exact length, takes one word less than a `Vec`, and its type says that it's final. A field is a `Vec<T>` only if the type itself grows it (e.g., an arena's items, a builder's output, a `DefinitionList` filled by `add_definition`). Build in a `Vec`, or `collect` directly, then store the result as a `Box<[T]>`.

## Data Layout and APIs

Lay data out flat (arenas of nodes referred to by handle, with their children in a `RunList`), but expose it through methods and views rather than raw fields, so that callers never have to know the layout. An arena and its children always travel together, so they're bundled into one type (e.g., the `HirNodeStore`), and only a node's own children go into it: what an owner does with nodes (its root, its parameters, its annotations) is kept on the owner.

How a value is reached depends on what it is:

- **No public fields.** A domain type exposes its contents through methods, named after what they return (e.g., `body.root()`, `body.parameters()`, `body.store()`), so a field can change without its callers changing. Its fields are visible only to the module that builds it. The exception is a plain record, whose only job is to carry values from one place to another (e.g., a `DiagnosticDescription`, a `TypeMismatch`): its fields are public, so that it can be built with a struct literal.
- **Generic collections are indexed.** A collection that holds one kind of thing, with no meaning of its own (`HandleMap`, `SideHandleMap`, `RunList`, the parser's `Sink`), implements `Index`, like a `Vec`: `map[handle]` when the value must be there, `map.get(handle)` (returning an `Option`) when it might not be.
- **Domain stores have named methods.** A store with meaning of its own (the `HirNodeStore`, `BodySemantics`, a `ScopeTree`, the MIR's `Cfg`) never implements `Index`: each lookup is a method named after what it returns (e.g., `store.expression(handle)`, `semantics.expression_type(handle)`, `cfg.block(handle)`), returning plain data or a view, with a `_mut` twin when it needs one (e.g., `cfg.block_mut(handle)`). A lookup that might find nothing returns an `Option` (e.g., `semantics.path_resolution(handle)`).
- **No `get_` prefix.** Following Rust's API guidelines, a getter is named after what it returns, and `get` is kept for a collection's lookup that might find nothing.

## Layout

### Directory Layout

- Split into crates, keep the list of crates flats
- Within a crate, split stuff into modules as needed, keep structure mostly flat
- Have a folder for `common/` for miscellaneous files
- Extract anything self-contained into its own file

### File Layout

```rust
// mod statement 
// (mod-intra-ordering: suggested reading order)

// use std
// use third-party crates
// use project crates

// pub type definition 
// pub(crate) type definition       
// private type definition   
// (type-intra-ordering: from most important type definition, to least)   

// IF the module's interface is a set of functions:
// public function 
// functions

// impl pub type {
    // pub methods
    // pub(crate) methods
    // private methods
    // (method-intra-ordering: constructors, accessors, then modifiers)
// }   
// impl project trait for pub type        
// impl std trait for pub type 

// impl pub(crate) type {
    // pub methods
    // pub(crate) methods
    // private methods
    // (method-intra-ordering: constructors, accessors, then modifiers)
// }         
// impl project trait for pub(crate) 
// impl std trait for pub(crate) type

// impl private type {
    // pub methods
    // pub(crate) methods
    // private methods
    // (method-intra-ordering: constructors, accessors, then modifiers)
// }           
// impl project trait for private type
// impl std trait for private type

// (impl-intra-ordering: follows from the type definition intra ordering)

// #[cfg(test)]
// mod tests {

// }
```

Inspired by [matklad's advice](https://www.reddit.com/r/rust/comments/wwbxhw/comment/ilkid50/?screen_view_count=4)

## Commit Messages

https://scopedcommits.com/

## Doc Comments

```rust
/// <verbs>s ...
/// 
/// # Examples
/// ```rust
/// ```
```

- Common verbs:
    - **Query/access:** Returns, Gets, Contains, Indexes, Finds
    - **Insertion:** Adds, Appends, Inserts, Puts, Pushes
    - **Deletion:** Removes, Clears, Deletes, Trims, Drops
    - **Modification:** Replaces, Sets, Sorts, Reverses, Updates
    - **Construction:** Constructs, Creates
    - **Iteration/traversal:** Performs, Iterates, Traverses
    - **Copy/transform:** Copies, Converts, Wraps

## Comparisons

When comparing, use `<` and `<=`, and avoid `>` and `>=`: less-than comparisons read in the same order as the number line, with smaller values on the left.

```rust
// Good
assert!(lo <= x && x <= hi);

// Not as good
assert!(x >= lo && x <= hi);
```

## Import Style

- When two modules define items with the same name (e.g., an `Expression` in both a syntax tree and an intermediate representation), qualify those items with their module, so that a name says where it comes from. 

```rust
// Good
use crate::syntax::ast;
use crate::semantics::hir;

fn lower_expression(expression: ast::Expression) -> hir::Expression { .. }
```

```rust
// Not as good
use crate::semantics::hir::Expression;
use crate::syntax::ast::Expression as AstExpression;

fn lower_expression(expression: AstExpression) -> Expression { .. }
```

- Avoid local `use MyEnum::*` imports.

- Prefer `use crate::foo::bar` to `use super::bar`. The exception is a `#[cfg(test)] mod tests`, which starts with `use super::*;` to test the items of the module around it.

- When implementing `Debug` or `Display`, import `std::fmt`:

```rust
// Good
use std::fmt;

impl fmt::Display for Point {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { .. }
}
```

```rust
// Not as good
impl std::fmt::Display for Point {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { .. }
}
```
