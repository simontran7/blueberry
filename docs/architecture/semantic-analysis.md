# Semantic Analysis

Semantic analysis answers two questions about the program: _what does each name refer to_, and _what type does each expression have_.

```
CST ──► AST lowering ──► name collecting ──────────► type checking a body
        (bodies,          (definition lists,     (walks each body; resolves
         signatures)       imports, scope trees)  each path when it reaches it)
                                   ▲                           │
                                   └──── name resolving ◄──────┘
```

1. **AST lowering** turns the syntax tree into HIR: one signature and one body per definition. It never looks at what a name means or what type something has.
2. **Name collecting** gathers what names _can_ refer to: the definitions of each file and block, the imported modules, and the scopes of each body.
3. **Type checking a body** walks each body once, and decides what each name _does_ refer to (by asking the `Resolver`, using what was collected) and what type each expression has, together.

Each stage is made of queries, computed the first time something needs them and cached afterwards.

## AST Lowering

### Stable Keys

Every function, constant and block is given a **key** (`FunctionKey`, `ConstantKey`, `BlockKey`), which is a `salsa` tracked struct. A key's identity describes _what_ the node is rather than _where_ it sits in the file:
- a function or a constant: its file, what it's declared in (the file, or a block), and its name
- a block: its file, the definition whose body it's in, and the block it's nested in (if any)

`salsa` matches a key to its previous version by these fields (and, for keys with identical fields, by the order they're created in), so editing one function, constant, or block leaves every other key, and so every other cached result, unchanged.

Each key also records where its node currently is (its kind and range, a `RedNodeTag`), so that its syntax node can be found again (`.to_ast_node(db)`). This field is _tracked_: when text only moves, it changes without changing the key's identity, and only the queries reading it (e.g., lowering) re-run. Their results come out identical, so nothing after them is recomputed.

_In the code:_ `keys.rs`; keys are created while collecting definitions (`top_level_definitions_of`, `block_definitions_of`) and while lowering bodies (blocks)

### Signatures

Every definition's header is lowered into a `FunctionSignature` (its name, parameter annotations and return annotation) or a `ConstantSignature` (its name and annotation). The annotations are still just what the user wrote, e.g. `I32`; they only become types when they're needed, with `()` as the return type if none is written, and the `Error` type for an unknown type name.

An annotation is a **type expression**: what the user wrote for a type, which denotes a type (a `Ty`) the way an expression denotes a value.

> [!NOTE]
> An annotation becomes a type in two steps:
> 1. **lowering** (AST lowering): the syntax becomes a type expression in the HIR, e.g. the path `I32`. It's still just a name.
> 2. **resolving** (type checking): the type expression becomes a `Ty`, e.g. the built-in 32-bit integer type, by `resolve_type_expression`.
>
> Resolving is part of type checking, not of AST lowering, since it needs to know what names mean: today, a name can only be a primitive, but once blueberry has user-defined types, `Point` will need to be looked up the same way as a value path.

Type expressions nest (e.g., `(I32, (Bool, U64))`), so every type expression gets its own handle in a `HirNodeStore` (see [Bodies](#bodies)), and a tuple type expression refers to its elements by handle. A signature has its own store, and so does each body, and both are built by the same `HirNodeStoreBuilder`. It also records in a `HirNodeRanges` where each type expression came from, so that an unknown name inside a tuple (e.g., `Foo` in `(I32, Foo)`) is reported at the name itself.

Resolving `(I32, Foo)` gives a `ResolvedTypeExpression`: the type `(I32, Error)`, and the handles of the paths that name no type (here, `Foo`). The caller turns each of those into an "unknown type" diagnostic. It's the same function for signatures and for a `let` in a body.

A signature is kept separate from its body, so that `f` can call `g` using only `g`'s signature, without `g`'s body being checked first. This matters for two reasons:
- **stability**: editing inside `g`'s body doesn't change `g`'s signature, so `f`'s cached result stays valid and `f` isn't checked again
- **recursion**: if `f` calls `g` and `g` calls `f`, neither needs the other's body, so checking them doesn't loop forever

This only works because blueberry requires every signature to be fully annotated, and so the header alone says what the type is.

_In the code:_ `signature::hir_of(d)`, a `Signature` (either a `FunctionSignature` or a `ConstantSignature`, lowered by the `SignatureLowerer` in `ast_lowering/signature_lowerer.rs`), its ranges `signature::ranges_of(d)`, and as a type, `signature::type_of(d)`; annotations are lowered into the HIR by `lower_type_expression` (`ast_lowering/type_expression_lowering.rs`), then turned into types by `resolve_type_expression` (`type_checking/type_expression_resolution.rs`)

Every query here and below takes a `Definition` (a function or a constant), rather than having one query per kind of definition: `Definition` is a salsa **supertype** (an enum of the two salsa keys, `FunctionKey` and `ConstantKey`), so a single query can be keyed by either, and only matches on the kind where the two genuinely differ (e.g., `body::semantics_of` checks a function body against its return type, and a constant body against its declared type).

### Bodies

Every body is lowered into a **HIR** (a `Body`): a `HirNodeStore` holding flat tables of expressions, statements, local bindings and type expressions, each referred to by a handle, plus the body's root expression and parameters. Signatures use the same `HirNodeStore` for their type expressions, so every HIR node, whatever it belongs to, is stored and walked the same way. Every `let` and parameter becomes its own **binding** (see [Name Collecting](#name-collecting)).

The store only holds nodes, and the children of those nodes (e.g., a tuple's elements). What an owner does with its nodes is kept on the owner: a body's `root_handle` and `parameter_handles`, a function signature's `parameter_annotation_handles` and `return_type_annotation`, and a constant signature's `annotation`.

Alongside each body, a **`HirNodeRanges`** is built, recording the text range each HIR node came from.

The HIR itself stores no positions in the text. This way, an edit that only moves text (e.g. adding whitespace or a comment) leaves the HIR unchanged, so the HIR, and every result computed from it (scopes, types, MIR), stays cached.

These ranges are **relative** (a `RelativeTextRange`): each one is measured from the start of its definition (its **anchor**, `Definition::anchor`) rather than from the start of the file. Most edits happen outside any given definition, and an edit above a definition only moves its anchor, so its `HirNodeRanges` stays unchanged too. A range is turned back into a range of the file only when it's needed, e.g. when a diagnostic is reported, by adding the anchor's current position (`expression_absolute_range(handle, anchor)`, `type_expression_absolute_range(handle, anchor)`), which gives an `AbsoluteTextRange`. The two are separate types, so a relative range can't reach a diagnostic without being anchored first.

For example, adding a line above `func b` shifts `b` in the file, but not `b`'s nodes relative to `b`:

```
func a() {}                     func a() {}
                                // a new line
func b() {                      func b() {          ← anchor moves: 13 → 26
    let y: I32 = 1;                 let y: I32 = 1; ← `I32` stays at 22..25 from the anchor
}                               }
```

Lowering produces the HIR and its ranges together (`body::hir_and_ranges_of`), but they're cached separately: `body::hir_of` returns only the HIR, so when an edit changes only the ranges, salsa sees that `body::hir_of`'s result didn't change, and doesn't recompute anything that depends on it.

The `HirNodeRanges` is the bridge between the two:
- **HIR to text**, for diagnostics: a diagnostic stores the handle of the node it's about (e.g. an `ExpressionHandle`), and its range is only looked up through the `HirNodeRanges` when it's reported
- **text to HIR**, for editor features such as hover or go-to-definition: the syntax node under the cursor is mapped back to its handle, whose type can then be looked up (not built yet)

_In the code:_ `body::hir_of(d)`, `body::ranges_of(d)`

## Name Collecting

In blueberry, the same name can be declared many times.

For example, a name is redeclared in the same scope:

```
let x = 1;
let x = true;
println(x); 
```

Both variables are called `x`. If the compiler only remembered names, it would have a single entry for `x` and couldn't tell whether `println(x)` means the number or the boolean.

The fix is to give every declaration its own ID: a **binding** for each `let` and parameter, and a **key** (`FunctionKey`, `ConstantKey`) for each definition. Resolving a name always produces one of these IDs:

```
let x = 1; // binding #1
let x = true; // binding #2
println(x);        
```

Now, `println(x)` can be resolved to binding #2.

Bindings let us tell declarations apart, but they don't say _how_ to choose between them. That's the job of a fixed lookup order (see [Resolving a Path](#resolving-a-path)): search from the closest scope outward, and stop at the first match.

> [!NOTE]
> Bindings also matter after name resolution. Later passes store facts per variable, such as its type, and keep them around. Keying those tables by binding instead of by name means `{ let x = 1; }` and `{ let x = true; }` never overwrite each other.

**Name resolution** is the process of determining which declaration (i.e., `let` binding, `const` definition, or `func` definition) each name in a program refers to. In blueberry, it's split in two:
1. **collecting** (this section): the declarations and scopes are stored in `DefinitionList`s, `ImportList`s, and `ScopeTree`s, before any body is checked
2. **resolving**: each use of a name is looked up in them by the `Resolver`, at the moment the type checker reaches it (see [Resolving a Path](#resolving-a-path))

Each data structure below is built by its own query, separately, the first time something needs it:
- one `DefinitionList` for the file's top-level definitions (`top_level_definitions_of`): built first, since it's how the compiler finds out which definitions exist.
- one `DefinitionList` per block, for the definitions declared inside that block (`block_definitions_of`): built the first time the `Resolver` searches that block's scope.
- one `ImportList` for the file's import declarations (`imports_of`): built the first time the `Resolver` doesn't find a name locally, in a block, or at the file's top level. So if every name in a file resolves before reaching imports, its `ImportList` is never built.
- one `ScopeTree` per function or constant definition, for its parameters and `let` bindings (`scopes_of`): built right before that body is type checked.

> [!NOTE]
> We may think of the `ScopeTree`(s), `DefinitionList`(s), and the `ImportList` as a disconnected symbol table.

### Definition Lists

A `DefinitionList` is merely a flat list of the functions and constants declared at one level: either a file's top level, or a single block. Unlike a `ScopeTree`, it needs no nesting, because a definition is visible across its whole file or block, even before the line that declares it. Each definition is stored as its key, and every later step is asked for by key, e.g. "lower the body of `FunctionKey(double)`".

A nested definition (a `func` or `const` declared inside a block) isn't part of its parent's body, because it's semantically independent of the definition around it: it only borrows the parent's block as a namespace. It can't see the parent's local bindings (it isn't a closure), and it's visible across the whole block, even before the line that declares it. The parent's HIR only keeps the block's key (and a `Statement::Definition` placeholder where the declaration was), and the definition is listed in that block's `DefinitionList`. From there, it's handled exactly like a top-level definition, with its own signature, body, and type checking:

```
main's body (HIR)
└─ Expression::Block { block_key: Some(B), statements: [Definition, Let x, …], tail }
                                   │
                                   ▼
        block_definitions_of(B)  →  DefinitionList [ FunctionKey(is_ready) ]
                                                               │
                         ┌─────────────────────────────────────┼──────────────────────────────┐
                         ▼                                     ▼                              ▼
            signature::hir_of(is_ready)             body::hir_of(is_ready)      body::semantics_of(is_ready)
          (name, parameters, return type)                 its own HIR                its own type check
```

### Import Lists

An `ImportList` is a flat list of paths from all the file's `import` declarations. Each path (e.g., `shapes::circle`) names a module, i.e., a file (`shapes/circle.bb`), which the `ModuleMap` finds. The module is brought into scope under the path's last segment (`circle`), and its definitions are then reached through a qualified path (e.g., `circle::area`). Only a module can be imported, never a single definition, so that `circle::area` always reads the same way, wherever it's used.

An import naming no module (e.g., `import shapes::nope;`) is reported once, at the import (`import_diagnostics_of`). Its name is still brought into scope, but as a failed import, so that every path through it (e.g., `nope::area`) silently gets the `Error` type rather than reporting the same problem again (see [Resilience](#resilience)).

### Scope Trees

A **scope** is a region of code where a set of names can be used. A new scope begins at the start of a function body (holding its parameters), at a block's `{`, or right after a `let`, and ends at the closing `}` of the enclosing block. A new scope nests inside the current one, so code in an inner scope can also see the names of every scope around it.

A `ScopeTree` is the tree of all scopes in one function definition's body or a constant definition's body. Each scope holds names, each paired with its binding, and a pointer to its parent: the scope around it. The tree also records the innermost scope every expression sits in (`containing_scope`), so that the `Resolver` knows where to start looking up a use site (e.g., the `x` in `println(x)`).

For example, this function:

```
func f(a: I32) {
    let x = 1;
    println(x);
    x = x + 1;
    foo();
    let y = 2;
    println(y);
}
```

has these scopes:

```
func f(a: I32)
┌─ scope 0: a ─────────────────────────────────┐
│ {                                            │
│ ┌─ scope 1: (block) ───────────────────────┐ │
│ │   let x = 1;                             │ │
│ │ ┌─ scope 2: x ─────────────────────────┐ │ │
│ │ │   println(x);                        │ │ │
│ │ │   x = x + 1;                         │ │ │
│ │ │   foo();                             │ │ │
│ │ │   let y = 2;                         │ │ │
│ │ │ ┌─ scope 3: y ─────────────────────┐ │ │ │
│ │ │ │   println(y);                    │ │ │ │
│ │ │ └──────────────────────────────────┘ │ │ │
│ │ └──────────────────────────────────────┘ │ │
│ └──────────────────────────────────────────┘ │
│ }                                            │
└──────────────────────────────────────────────┘
```

## Type Checking a Body

**Type checking** is the process of verifying that a program uses its types consistently, reporting a type error wherever it doesn't. This means two things:
- **bodies**: every expression is assigned a type, and each one must fit where it's used
- **`trait`s and `impl`s**: every `impl` must match the `trait` it implements (blueberry doesn't have traits yet)

There are two kinds of bodies to typecheck:
- a **function body**: the block of a `func`
- a **constant body**: the initializer of a `const`

Each body, including the body of a nested definition, is checked on its own, using only the _signatures_ of other definitions. Within a body, names are resolved and types are inferred together, in a single walk.

### The Model

#### Type Inference

Languages can be classified by which declarations (definitions and locals) may omit their type annotations:
- **Local type inference**: function signatures are annotated, but declarations inside a body may omit their annotations.
- **Full type inference**: any declaration may omit its annotation, even a function signature.

There are two common type inference techniques:
- **Immediate inference** takes the type straight from the expression (e.g. Go's `x := 5`). It only works when an expression's type is always fully known on the spot.
- **Constraint-based inference** also works when the type arrives later (e.g. Rust's `let v = Vec::new(); v.push(1);`, where `v`'s element type is only known on the second line). It introduces unknowns, generates constraints between types, solves them, then replaces every unknown with its solution.

Blueberry has local type inference, and infers with constraints. Constraints are solved **eagerly**: each one is solved as soon as it's generated, during the walk, rather than collected and solved after it, because later code may need a type _now_ (e.g., a method call like `x.count()` needs `x`'s type to find `count`). Only replacing every unknown with its solution happens once, at the end (see [Finishing a Body](#finishing-a-body)).

#### Types

A type is one of:
- a primitive: `I8`, `I16`, `I32`, `I64`, `U8`, `U16`, `U32`, `U64`, or `Bool`
- a tuple type, e.g. `(I32, Bool)`; `()` (unit) is simply the empty tuple, and `(I32,)` is a tuple of one element
- a function type, e.g. `fn(I32) -> Bool`
- `Bottom`: the type of an expression that never produces a value (`return`, `break`, `continue`, a `loop` without `break`)
- `Error`: the type of anything already reported as wrong (see [Resilience](#resilience)). It's printed as `{error}` in diagnostics and dumps, like `{unknown}` and `{integer}`, so that it can't be mistaken for a type the user wrote.
- an inference variable, i.e., an unknown

#### Inference Variables

For each type that isn't known yet, the type checker creates a placeholder called an **inference variable** $?i$. There are two kinds:
- a **general** variable, which may become any type. It's created for an unannotated `let` (e.g., `let x = ...`), and for an `if`, block, or `loop` whose type isn't expected to be anything known (see [The Coercion Funnel](#the-coercion-funnel)).
- an **integer** variable, which may only become an integer type. It's created for an integer literal that isn't expected to be an integer type (e.g., the `5` in `let x = 5`).

A type can be fully concrete (e.g. `I32`), an inference variable, or a compound type containing inference variables (e.g. `fn(?0) -> I32`).

Following the approach of Milner's **Algorithm J**, inference variables are stored in an **inference table**: when an inference variable is solved, the type it's solved to (its **solution**) is written once into the table, rather than updating every type that mentions it, as done in **Algorithm W**. The inference table is designed as an extension of a disjoint set. A group of inference variables known to be equal is stored as a disjoint set, where the group's common type is the disjoint set's representative value. General and integer variables live in separate disjoint sets, so that an integer variable may _only_ ever become an integer type.

Since solutions are looked up in the inference table rather than written into every type, a type may mention an inference variable that has _already_ been solved (i.e., is no longer unknown). This means that before looking at a type, the type checker must **shallowly resolve** it (via `.shallow_resolve(ty)`). Shallow resolving means following chains of inference variables if any, but it won't recurse in the inner types (e.g., `fn(?0) -> I32` is returned as is).

In contrast, **zonking** (via `.zonk(ty)`) replaces every solved inference variable _inside_ the type (e.g., `fn(?0) -> I32` becomes `fn(Bool) -> I32` if `?0 := Bool`). Its variant `.zonk_and_poison(ty)` also gives up on every inference variable still unsolved, by solving it to the `Error` type (see [Finishing a Body](#finishing-a-body)).

As a rule of thumb, the type checker shallow resolves when it only needs to look at a type's outer shape while checking (e.g., "is this an integer type?"), and zonks when it needs the entire type, i.e., to display it in a diagnostic, to check whether an error type is hidden anywhere inside it, and at the end, when every unknown is replaced with its solution.

Concretely, the type checker shallow resolves a type right before matching on its kind, i.e., whenever it asks a question about the type's outer shape:

| Where | Question |
|---|---|
| after checking any expression | is it `Bottom`? (to track divergence) |
| a call | is the callee a function type? |
| `-e` | is the operand signed? (and, once the body is finished, did it end up unsigned?) |
| a binary operator | is the left-hand side `Bottom`? |
| arithmetic operators | is the operand an integer? |
| reading an expectation (e.g., an integer literal, an `if`) | is the expected type an integer type, or an unsolved inference variable? |

It doesn't need to when it only passes a type along (returns it, stores it, or puts it in an expectation), nor when it calls `unify`, `coerce`, or pours into a funnel, since those shallow resolve their inputs themselves.

#### Unification

The only kind of constraint so far is **equality**, solved by `unify(expected, found)`, which makes the two types equal, or reports a mismatch if they can't be. `expected` is the type the context requires (e.g. an annotation, a parameter, `Bool` for a condition) and `found` is the type the expression actually has, so a mismatch can be reported as "expected `Bool`, found `I32`".

> [!NOTE]
> Once blueberry has traits, a second kind of constraint will appear: **obligations** (e.g., "`T` implements `Eq`"), solved by a trait solver.

First, `unify` must shallow resolve both types. Otherwise, it might mistake a solved variable for an unsolved one. For example, if `unify` treated the `?0` on the last line as still unknown, it would set `?0 := Bool`, thereby overwriting `I64`, and missing the type mismatch error:

```
let x = 5;          // x: ?0, solved to 5's integer variable
let y: I64 = x;     // unify(I64, ?0) → ?0 := I64
let z: Bool = x;    // unify(Bool, ?0) → must see I64, and report a mismatch
```

After shallow resolving both sides, `unify` dispatches on the two shapes:

| `expected` | `found` | Action |
|---|---|---|
| `t` | `t` (the same type) | ok (types are interned, so this is a cheap `==`) |
| `?x` (general) | `?y` (general) | merge the two groups (both are unsolved, so it can't fail) |
| `?x` (general) | any type `t` | occurs check, then `?x := t` |
| any type `t` | `?x` (general) | occurs check, then `?x := t` |
| `?i` (integer) | `?j` (integer) | merge the two groups |
| `?i` (integer) | an integer type / `Error` | `?i := t` |
| an integer type / `Error` | `?i` (integer) | `?i := t` |
| `Error` | anything (either side) | ok (see [Resilience](#resilience)) |
| `fn(p1, ..) -> r1` | `fn(q1, ..) -> r2` | same number of parameters? then unify each $p_i$ with $q_i$, and `r1` with `r2` |
| `(t1, ..)` | `(u1, ..)` | same number of elements? then unify each $t_i$ with $u_i$ |
| anything else | | mismatch |

The function and tuple rows are the recursive part: rather than comparing two types with `==`, unification walks into both and unifies the pieces, so `fn(?0) -> I32` and `fn(Bool) -> I32` unify with `?0 := Bool`.

Before setting `?x := t`, `unify` performs an **occurs check**, i.e., it checks that `?x` doesn't occur anywhere inside `t`. Otherwise, the solution would have to contain itself, which would be a type error. For example:

```text
?x = fn(?x) -> I32
   = fn(fn(?x) -> I32) -> I32
   = fn(fn(fn(?x) -> I32) -> I32) -> I32
   = ...forever
```

### The Walk

The type checker walks a body by **recursive descent** over its HIR: `check_expression` dispatches on the kind of expression to a dedicated method (e.g., `check_if`, `check_call`), which in turn calls `check_expression` on each child, and returns the expression's type.

Since the walk goes top-down, a parent passes its child an **expectation**, i.e., the type it wants the child to have (if any), and the child returns the type it actually has. Types therefore flow in both directions: expectations go down the tree, and actual types come back up.

An expectation is only a hint: the child may use it, pass it further down, or ignore it. So after checking a child with an expectation, the parent **coerces** the child's type to the expected type (see [Coercion](#coercion)), and reports a type mismatch if that fails (via `coerce_or_report`). For example, an `if`'s condition is checked with the expectation `Bool`, then coerced to `Bool`:

| Condition's type | Coercion | Result |
|---|---|---|
| `Bool` | nothing to do | ok |
| `Bottom` (e.g., `if return { }`) | `Bottom` can be used as any type | ok |
| an unsolved inference variable `?0` | `unify(Bool, ?0)` → `?0 := Bool` | ok |
| `I32`, or an integer variable (e.g., `if 5 { }`) | `unify(Bool, I32)` fails | mismatch |

A child uses an expectation when it already knows the answer: an integer literal expected to be an `I64` simply takes `I64`, rather than creating an integer variable only to unify it with `I64` right after. A child passes an expectation down when its type comes from one of its own children: a block passes its expectation to its tail expression, and an `if` to both branches. This way, a mismatch is reported on the innermost expression that's wrong (e.g., one branch), rather than on the whole `if`.

### Resolving a Path

When the walk reaches a path (e.g., `x`, or `circle::area`), it asks the `Resolver` what the path names, using what was collected (see [Name Collecting](#name-collecting)). This happens during the walk rather than in a separate pass, because the path's type is needed right then, and because some names can't be resolved without types (e.g., a method in `x.count()` needs `x`'s type; blueberry doesn't have methods yet).

The `Resolver` resolves a path's first segment as a name, in the following order:
1. each scope, starting from the path's innermost scope (its `containing_scope`) and moving outward: its `let` bindings, then, if the scope is a block, the definitions declared in that block
2. the definitions declared in the blocks around the body's own definition, if it's nested (e.g., a function declared inside another function's block can call its siblings)
3. top-level definitions in the current file
4. imported modules

This way, the closest declaration of a name always wins: the most recent `let` beats an earlier one, an inner scope beats an outer one, and a file's top-level definitions beat imported modules:

```
func x() { ... } // FunctionKey(x)
func main() {
    let x = 5;   // binding #1
    x;           // binding #1, not FunctionKey(x)
}
```

Each following segment is then looked up among the top-level definitions of the module the previous segment resolved to. For example, in `circle::area`, `circle` resolves to the module `shapes/circle.bb`, and `area` is looked up in that file's top-level `DefinitionList`.

Resolution fails if:
- the first segment isn't found (`nope`, or `nope::area`)
- a following segment isn't found in its module (`circle::nope`)
- a segment followed by another one isn't a module (`area::x`)

And a path that resolves to a module can't be used as a value (e.g., `let c = circle;`). In each case, a diagnostic is reported and the path's type is `Error` (see [Resilience](#resilience)).

_In the code:_ `Resolver::resolve_path`

### Coercion

A **coercion** is a rule letting a type be used where a different one is expected.

Blueberry has a single one so far, **bottom-to-any**: `Bottom` can be used where any type is expected, since an expression of type `Bottom` never produces a value. `coerce(from, to)` tries the coercions, then falls back to `unify(to, from)`. Hence, every "must be" in [Rules per Expression](#rules-per-expression) is checked as a coercion.

Bottom-to-any can't be part of `unify` itself: `unify` means equality, which goes both ways, whereas a coercion only goes one way (`Bottom` can be used as an `I32`, but not the other way around).

#### The Coercion Funnel

Sometimes, several values flow into one result:
- an `if` with an `else`: both branches
- a block: its tail expression
- a `loop`: every `break e`
- a function: every `return e`, and its body

Each of these uses a **coercion funnel**: it starts from the expected type, or from a fresh general variable if there's none, and each value is **poured** into it, i.e., coerced to the type merged so far. Once every value is poured, the funnel is **drained**, giving the merged type, or `Bottom` if nothing was poured (e.g., a `loop` that never breaks).

For example, in `if c { return } else { 5 }` without an expectation, the funnel starts at `?0`; `Bottom` is poured, then `{integer}`, so the `if`'s type is `{integer}`.

#### Coercing `Bottom` into an Unsolved Inference Variable

When `Bottom` is coerced into an unsolved inference variable, the variable is deliberately left unsolved, so that a later value can still decide its type:

```
let a = if c { return } else { 5 };       // Bottom into ?0: ?0 stays unsolved
                                          // 5 into ?0: ?0 := {integer}, so a: I32
let b = if c { return } else { return };  // Bottom into ?0, twice: ?0 stays unsolved
```

For `b`, nothing ever solves `?0`. Yet, it isn't an error: `b` simply never gets a value. Hence, when `Bottom` is coerced into an unsolved inference variable, that variable is marked as **coerced from `Bottom`** (the mark is kept when it's unified with other variables). When the body is finished, a marked inference variable that's still unsolved is solved to `Bottom`, rather than becoming an error, so `b: Bottom`. An unmarked one is still an error, since nothing ever said anything about it.

### Divergence

An expression **diverges** if it never finishes, so that the code after it never runs: `return`, `break` and `continue` jump somewhere else, and `loop {}` runs forever. That's what the `Bottom` type stands for (see [Types](#types)): an expression of type `Bottom` never produces a value, because control never gets past it.

The type checker needs to know when code can't be reached, for one main reason: the end of a block. A function that must return an `I32` is only wrong for reaching its end without a value if its end can actually be reached:

```
func f() -> I32 {
    loop {
        return 1;
        break;    // can't run: it comes after a `return`
    };
}                 // fine: the loop never finishes, so the end of `f` is never reached

func g(c: Bool) -> I32 {
    loop {
        if c { return 1; }
        break;    // can run, when `c` is false
    };
}                 // error: "implicitly returns `()`", since the loop can finish and reach the end of `g`
```

#### Tracking It

While walking a body, the type checker keeps `self.is_reachable`, which answers: can the point the walk has reached run?
- `true`: the code being checked might run
- `false`: something checked before it has definitely left, so it can't run

It starts as `true`. After checking any expression, if its type is `Bottom`, it becomes `false` (at the end of `check_expression`), and it stays `false` for everything checked after it, since nothing after a `return` can run either:

```
foo();      // reachable
return;     // Bottom, so the code after it isn't reachable
bar();      // not reachable: can't run
```

#### Where It's Combined

Only three expressions decide their own reachability, rather than simply making what follows unreachable once something in them diverged, because they have parts that might not run:
- **`if`**: the condition always runs, but only one branch does. So with an `else`, the code after the `if` is reachable if the condition finishes, and _either_ branch does (`condition_reachable && (then_reachable || else_reachable)`). Without an `else`, a false condition skips the then branch, so only the condition counts.
- **`a and b`, `a or b`**: `b` might not run, so only `a`'s divergence counts.
- **loops**: a loop can only finish through a `break`, so it diverges unless a `break` in it can run. Each `break` that's checked while `self.is_reachable` is `true` marks its loop's frame as exitable (`can_exit`), and when the loop's body is done, `check_loop` sets `self.is_reachable` to that mark. A `break` checked while it's `false`, like the one in `f` above, can't run, so it doesn't count.

#### What It's Used For

A block without a tail produces `()`, but only if its end can be reached. So when the block's end is reached while `self.is_reachable` is `false`, nothing is poured into its funnel, and its type is `Bottom` instead. That's why `f` above is fine: its body's type is `Bottom`, which coerces to `I32`.

Divergence and an expression's type are tracked separately, and don't always match. In `f`, the `break` can't run, but it's still type checked, so it still gives the loop `()`: the loop's type is `()`, yet it diverges. What decides whether the code after the loop can run is its divergence, not its type.

_In the code:_ `self.is_reachable`, `LoopFrame::can_exit`

### Rules per Expression

The rules below use two short phrases, one for each half of how a parent checks a child (see [The Walk](#the-walk)):
- **"checked with `T`"**: the child is checked with the expectation `T`. This is only a hint that's passed down: the child may use it to decide its own type (an integer literal checked with `U64` becomes a `U64`), pass it on to its own children, or ignore it.
- **"must be `T`"**: once the child is checked, its type is coerced to `T` (see [Coercion](#coercion)). This is the actual requirement: if the coercion fails, a type mismatch is reported at the child.

The two usually go together. For example, an `if`'s condition is checked with `Bool`, and must be `Bool`: the expectation tells the condition which type it should have, and the coercion afterwards rejects a condition that isn't a `Bool`, like `if 5 { .. }`.

#### Integer Literals

An integer literal can be any integer type (`I8` through `I64`, or `U8` through `U64`), so its type depends on its expectation:
- if the expected type is an integer type `t`, it takes `t`
- otherwise (i.e., no expected type, or a non-integer expected type), it gets a fresh integer variable `?i`, which the parent's coercion then solves, or rejects

```
let a: U64 = 5;   // expected U64 → 5: U64
let b = 5;        // expected ?0, which isn't an integer type → 5: ?1; then ?0 := ?1
let c: Bool = 5;  // expected Bool → 5: ?2; then unify(Bool, ?2) → mismatch
```

Checking whether the literal fits in its type happens at the very end, once every type is fully resolved (see [Finishing a Body](#finishing-a-body)). Additionally, a literal right under a negation is checked together with it, as a negative value (e.g., `-2147483648` fits in an `I32`, even though `2147483648` alone doesn't). A literal too large to even be stored (beyond `u128`) is lowered as having no value, and is reported by the same check, as out of range for whatever type it got. The literal is shown as written in the source.

#### Paths

A path takes the type of whatever it names (see [Resolving a Path](#resolving-a-path)):
- a **local** (a `let` binding or a parameter): the type recorded when the local was introduced, e.g. `T` for `let x: T = ..`, the declared type for a parameter `x: T`, or a fresh variable for `let x = e`. Every use of `x` shares that _same_ type, so whatever one use learns about `x`'s type, every other use sees too:

  ```
  let x = 5;        // x: ?0, solved to 5's integer variable
  let y: I64 = x;   // unify(I64, ?0) → ?0 := I64
  let z = x + 1;    // x: ?0, which shallow resolves to I64 → 1: I64
  ```
- a **function**: its signature type, e.g. `fn(I32) -> Bool` for `func f(x: I32) -> Bool`
- a **constant**: its declared type, e.g. `I64` for `const LIMIT: I64 = 5`

A path ignores its expectation, since its type is already decided; the parent coerces it to the expected type afterwards.

#### Tuples and Fields

- `(e1, .., en)`: if the expected type is a tuple with the same number of elements `(t1, .., tn)`, each `ei` is checked with `ti`, and must be `ti`; otherwise, each `ei` is checked without an expectation. The tuple's type is made of each element's expected type, or of its own type without one. Since each element was already coerced, the parent's coercion of the whole tuple can't fail on an element, so a mismatch is reported once, on the innermost wrong element
- `e.i`: `e` is checked first, and must be a tuple with an element at index `i` (written as a plain integer, so `e.01` isn't `e.1`); the expression has that element's type. Otherwise, the field is reported as unknown (or as a primitive having no fields, e.g. `5.0`). An unsolved general variable says nothing about which fields exist, so it's reported as "type annotations needed" (see [Type Annotations Needed](#type-annotations-needed))

```
let p = (1, true);       // p: (?i, Bool)
let q: (U64, Bool) = p;  // unify((U64, Bool), (?i, Bool)) → ?i := U64
let r = p.0 + 1;         // p.0: U64
let s = p.2;             // no field `2` on type `(U64, Bool)`
```

#### `let`

`let x: T = e` and `let x = e` are checked the same way: `x`'s declared type is `T`, or a fresh general variable without an annotation. `e` is checked with the declared type, and must be the declared type. `x` gets the declared type.

#### `if`

- the condition is checked with `Bool`, and must be `Bool`
- the branches are checked with the `if`'s expectation, but only if it's a known type: an unsolved inference variable says nothing yet, and would only let the first branch decide the type for the second one
- with an `else`: both branches are poured into a funnel (see [The Coercion Funnel](#the-coercion-funnel)); if the `else` doesn't fit, an "`if` and `else` have incompatible types" mismatch is reported
- without an `else`: the `if` produces `()` when the condition is false, so `()` is poured too; if a non-`()` type was expected, an "`if` may be missing an `else` clause" mismatch is reported

#### Blocks

- each `let` and expression statement is checked in order; an expression statement without a `;` (e.g., an `if` followed by more statements) must be `()`
- the tail expression is checked with the block's expectation, and poured into the block's funnel
- without a tail, `()` is poured, unless the block always diverges (it ends in a `return`, say), in which case nothing is poured, and the block's type is `Bottom` (see [Divergence](#divergence))

#### Loops

- a `loop`'s type comes from its `break`s: each `break e` checks `e` with the loop's expected type, and pours it into the loop's funnel; a bare `break` pours `()`. A `loop` without any `break` has type `Bottom`
- a `while`'s condition is checked with `Bool`, and must be `Bool`; a `while` has type `()`, and a `break` inside it can't carry a value
- in both, the body must be `()`, since its value is discarded each time around
- a loop diverges unless a `break` in it can run (see [Divergence](#divergence))

#### Calls

`f(a1, .., an)`: the callee `f` is checked first, with no expectation, since nothing says what type it should have. What happens next depends on its type:
- **a function type `fn(p1, .., pn) -> r`**: each argument `ai` is checked with `pi`, and must be `pi`. The call has type `r`. If the number of arguments is wrong, a mismatch is reported, and the arguments that have no parameter are still checked, with no expectation.
- **`Bottom`**: the callee never produces a value (as in `(return 1)(2)`), so the call never happens. It's accepted silently, and the call has type `Bottom`.
- **`Error`**: the callee's own error was already reported, so the call is accepted silently, and has type `Error`.
- **an unsolved general variable**: nothing is known about the callee, so it's reported as "type annotations needed" (see [Type Annotations Needed](#type-annotations-needed)), and the call has type `Error`.
- **any other type** (an integer, `Bool`, a tuple): it's reported as "expected function", and the call has type `Error`.

In every case but the first, there are no parameters to check the arguments against, but they're still checked with no expectation, so that a mistake inside an argument is found too.

#### Binary Operations

Each binary operator has a rule (`BinaryOperator::rule`) that says which types its operands must have, and which type it produces.

- `a + b`, `a - b`, `a * b`, `a div b`, `a mod b`: `a` is checked first, and must be an integer type. Only then is `b` checked: with `a`'s type, and it must be `a`'s type. The expression has `a`'s type.

  The order matters when `a` isn't an integer. In `(1, 2) + 1`, the operator can't be applied, so that's reported, and `b` is checked with no expectation instead. Otherwise, the `1` would also be reported, for not being a tuple, which is only a consequence of the first mistake.
- `a < b`, `a > b`, `a <= b`, `a >= b`, `a == b`, `a != b`: `a` is checked first, and can have any type. Then `b` is checked with `a`'s type, and must be `a`'s type, so that two values are only ever compared with a value of the same type. The expression has type `Bool`.
- `a and b`, `a or b`: both `a` and `b` are checked with `Bool`, and must be `Bool`, even when the other one isn't, so that a mistake on one side doesn't hide a mistake on the other. The expression has type `Bool`.

  `b` only runs when `a` doesn't already decide the result (`a and b` is `if a { b } else { false }`), so only `a`'s divergence counts: in `x and (return 1)`, the code after it is still reachable (see [Divergence](#divergence)).

For the arithmetic and comparison operators, when `a` diverges (as in `(return 1) + 5`), it says nothing about `b`'s type, so `b` is checked with no expectation. That doesn't apply to `and` and `or`, whose `b` is always checked with `Bool`.

#### Unary Operations

- `-e`: `e` is checked with the negation's own expectation, since `-e` has the type of `e`. Then `e` must be a signed integer, which is checked in two steps:
  1. right away, `e` must be an integer, and mustn't already be known to be unsigned. So `-true` and `-x` with `x: U32` are reported here, and `-e` becomes the `Error` type, so that it causes no further errors.
  2. once the body is finished, `e` is checked again, since its integer type might not have been known yet. In `let y = 5; let z = -y; let w: U32 = y;`, `y` is only known to be a `U32` after `-y` was checked.
- `not e`: `e` is checked with `Bool`, and must be `Bool`. The expression has type `Bool`, even when `e` isn't one, since `not` always produces a `Bool`.

#### Assignments

`a = b`: `a` must be a place, i.e., a local (not a function nor a constant), or a field of a place (e.g., `p.0.1`). `b` is checked with `a`'s type, and must be `a`'s type. The assignment has type `()`.

#### `return`, `break`, `continue`

`return e` checks `e` with the function's return type, and pours it into the function's funnel; a bare `return` pours `()`. `return` (and `break`, `continue`) has type `Bottom`.

#### Summary

| Code | Checks | Type of the expression |
|---|---|---|
| `5` | — | the expected integer type, or `?i` |
| `true` | — | `Bool` |
| `(e1, .., en)` | if `(t1, .., tn)` is expected, each `ei` must be `ti` | `(ty(e1), .., ty(en))`; `()` for `()` |
| `e.i` | `e` must be a tuple with an element at index `i` | the element's type |
| `x` (a path) | — | the type of what `x` names |
| `let x: T = e` | `e` must be `T` | `x: T` |
| `let x = e` | `e` must be `?0` | `x: ?0` |
| `if c { a } else { b }` | `c` must be `Bool`; `a`, `b` poured into one funnel | the merged type |
| `if c { a }` | `c` must be `Bool`; `a`, `()` poured into one funnel | `()` |
| `{ s1; s2; e }` | `e` poured into the block's funnel | `ty(e)`, `()` without a tail, or `Bottom` if it diverges |
| `f(a1, .., an)` where `f: fn(p1, .., pn) -> r` | each `ai` must be `pi` | `r` |
| `-e` | `e` must be a signed integer | `ty(e)` |
| `not e` | `e` must be `Bool` | `Bool` |
| `a + b`, `a - b`, `a * b`, `a div b`, `a mod b` | `b` must be `ty(a)`, an integer | `ty(a)` |
| `a < b`, `a == b`, … | `b` must be `ty(a)` | `Bool` |
| `a and b`, `a or b` | `a` and `b` must be `Bool` | `Bool` |
| `a = b` | `b` must be `ty(a)` | `()` |
| `return e` | `e` poured into the function's funnel | `Bottom` |
| `loop { .. }` | each `break e` poured into the loop's funnel | the merged type, or `Bottom` if it never breaks |
| `while c { .. }` | `c` must be `Bool` | `()` |
| `continue` | — | `Bottom` |

### Function and Constant Bodies

A function body is checked as follows:
1. Each parameter is given its declared type, so inside the body, `x` in `func f(x: I32)` already has type `I32`.
2. The declared return type `r` is remembered (`()` if there's no `-> <type>`), as the start of the function's funnel.
3. The body's block is checked with `r`, which the block passes down to its tail expression, and the block's type is poured into the function's funnel, like every `return e` inside it.

For example:

```
func f(x: I32) -> I64 {   // x: I32, r = I64; the block is checked with I64
    let y = 5;            // y: ?0; 5 is checked with ?0 → 5: ?1, then ?0 := ?1
    if x > 0 {            // 0 is checked with I32 (the lhs's type) → 0: I32
        return y;         // y: ?0 poured into the function's funnel → ?1 := I64
    }
    y                     // tail, checked with I64: y is an I64, ok
}
```

A constant body is only its initializer, since it has no parameters and no `return`: it's checked with the annotated type (which is mandatory in blueberry), and must be the annotated type:

```
const LIMIT: I64 = 5;   // 5 is checked with I64 → 5: I64
const FLAG: Bool = 5;   // Bool isn't an integer type → 5: ?0; unify(Bool, ?0) → mismatch
```

_In the code:_ `body::semantics_of(d)`

### Finishing a Body

Once the walk is over, every unknown is replaced with its solution:
1. every integer variable still unsolved defaults to `I32`
2. every general variable still unsolved and coerced from `Bottom` defaults to `Bottom` (see [Coercing `Bottom` into an Unsolved Inference Variable](#coercing-bottom-into-an-unsolved-inference-variable))
3. every negation of an integer variable is checked again, now that the variable's type is known: if it became unsigned, the negation is reported
4. every expression's and binding's type is **zonked and poisoned**: each solution is written back into the types recorded during the walk, since solving updated only the inference table. An inference variable still unsolved becomes the `Error` type, and a "type annotations needed" diagnostic is reported (once per variable)
5. every integer literal is checked to fit in its now-known type

#### Type Annotations Needed

The diagnostic points at where the unknown came from. Each general variable records its **origin** when it's created: the `let` binding it was made for (`let t = ..` without an annotation), or the expression it was made for (e.g., an `if` whose branches all diverge). When two variables are unified, a binding origin wins over an expression origin, since a binding is something the user can annotate:

```
[E0282] Error: type annotations needed
 2 │     let t = loop {};
   │         ┬ consider giving `t` an explicit type
 3 │     let y = t.0;
   │             ┬ type must be known at this point
```

The origin is a HIR handle (a `LocalBindingHandle` or an `ExpressionHandle`), so `body::semantics_of` stays independent of positions: the range is only looked up, through the `HirNodeRanges`, when the diagnostic is resolved (see [Diagnostics](#diagnostics)).

_In the code:_ `VariableOrigin`, `InferenceTable::zonk_and_poison`

The result, a `BodySemantics`, holds the body's semantics: each expression's type, each binding's type, what each path refers to, and the body's diagnostics.

### Resilience

The type checker is **resilient**: it keeps going after an error, and makes sense of as much of the code as possible, so that one run reports every _real_ error, and a mistake in one place doesn't affect unrelated code. Unlike error repair, it never guesses what the user meant (e.g., it doesn't assume an unknown name is an `I32`); it only marks what's broken as broken.

The risk is that one mistake causes a pile of follow-up errors that are only consequences of the first. To avoid that, blueberry uses **poisoning** (a trick from D's compiler: see Walter Bright's [Improving Compiler Error Messages](https://digitalmars.com/articles/b47.html)): the type of anything that's already wrong becomes the `Error` type (much like an `Error` node in the syntax tree), which is then silently accepted everywhere.

An `Error` type is produced, after its diagnostic has been reported, by:
- a path that doesn't resolve, that names a module, or that goes through a failed import (see [Resolving a Path](#resolving-a-path))
- an unknown type in an annotation, e.g. `let x: Foo = ..`
- a syntax error, i.e., an `Error` node from the parser
- an operator applied to the wrong type, e.g. `true + 1`
- an inference variable left unsolved when the body is finished

And an `Error` type is silently accepted by:
- `unify` and coercion, which succeed when either side is `Error`
- expectations: an expected `Error` type becomes no expectation, so it isn't passed down
- the checks that look at a type's shape: calling it, taking a field, or using it as an operand. Each has its own case for an `Error` type that accepts it silently.

A type that only _contains_ an `Error` isn't accepted silently, though. In `let t = (1, foo); t(5);`, `t` is `(I32, Error)`, and calling it is still reported. The `Error` inside the tuple doesn't change its shape, so calling it would be a mistake whatever `foo` turns out to be: it's a separate mistake, not a consequence of the first one.

For example, where `foo` doesn't exist:

```
let y: I32 = foo + 1;   // foo: Error, reported as an unresolved name
                        // Error + 1: Error is accepted as an integer, so no operator error
                        // 1 is checked with no expectation
                        // unify(I32, Error) → ok, so no type mismatch
```

Only the unresolved name is reported. After type checking, a body with any diagnostic or any `Error` type left in it isn't lowered to MIR.

#### Checking the Children Anyway

Poisoning keeps one mistake from causing more errors. The opposite matters too: one mistake mustn't _hide_ other mistakes. So when an expression is wrong in a way that stops it from being checked normally, its children are still checked, with no expectation, since nothing says what type they should have. A mistake inside a child is then still found, and every child still gets a type, which later stages (the HIR dump, and someday the editor's hover) expect.

```
let t = (1, 2);
t(foo);              // calling a tuple: reported
                     // the argument `foo` is still checked: "cannot find `foo`" is reported too
```

Fixing the first mistake wouldn't fix the second, so both are real, and both are reported in one run.

The same rule shows up wherever checking can't go on normally:
- a call whose callee isn't a function: its arguments are still checked (`check_call`)
- a call with too many arguments: the extra ones are still checked
- a field with no name, like `t.` (a syntax error): the receiver is still checked first (`check_field`)
- a binary operation with no operator (a syntax error): both sides are still checked (`check_binary_operation`)
- a `break` outside of a loop, or a `return` in a constant: its value is still checked (`check_break`, `check_return`)
- a `break` with a value from a `while`: the value is still checked

The children are always checked _before_ the expression gives up. For example, `check_field` checks the receiver before it looks at the field name, so that `foo.` still reports the unknown `foo`.

The syntax error cases don't show up in `blueberry check` today, since the driver stops after syntax errors, before type checking. The type checker still handles them, because an editor checks code while it's being typed, and so while it's full of syntax errors.

### Diagnostics

Semantic diagnostics come from three places:

| Source | Covers | Where |
|---|---|---|
| Type checker | everything inside a body, including `let` annotations | `body::semantics_of`, then `resolve` |
| `definition_diagnostics_of` | unknown types in signatures | `semantic_analysis.rs:288-296` |
| `import_diagnostics_of` | imports that point nowhere | `semantic_analysis.rs:312` |

While walking a body, the type checker records each problem as an `UnresolvedSemanticDiagnostic`, which points at HIR handles rather than positions in the text, so that the `BodySemantics` stays unchanged when text only moves. A type mismatch records the expected and found types, and a **cause**: why that type was required (e.g., an `if` and `else` disagreeing, or a missing `else`).

When a diagnostic is reported, it's **resolved** into a `Diagnostic` (via `.resolve(..)`), looking up its ranges through the `HirNodeRanges`. 

Additionally, mismatches are annotated with a cause which decides its message, e.g. "`if` and `else` have incompatible types".


