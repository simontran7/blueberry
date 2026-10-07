# Semantic Analysis

Semantic analysis starts where parsing ends: with a file's **CST**. It answers two questions about the program: _what does each name refer to_, and _what type does each expression have_.

## AST Lowering

TO WRITE

## Name Resolution

In blueberry, the same name can be declared many times.

For example, a name is redeclared in the same scope:

```
let x = 1;
let x = true;
println(x); 
```

Both variables are called `x`. If the compiler only remembered names, it would have a single entry for `x` and couldn't tell whether `println(x)` means the number or the boolean.

The fix is to give every declaration its own ID: a **binding** for each `let` and parameter, and a **key** (`FunctionKey`, `ConstantKey`) for each definition. The output of name resolution is always one of these IDs:

```
let x = 1; // binding #1
let x = true; // binding #2
println(x);        
```

Now, `println(x)` can be resolved to binding #2.

Bindings let us tell declarations apart, but it doesn't enforce _how_ choose between them. That's the job of a fixed lookup order. The idea is to search from the closest scope outward and stop at the first match. This picks the latest `x` above, and the local `x` over the function below:

```
func x() { ... } // FunctionKey(x)
func main() {
    let x = 5;   // binding #1
    x;           // binding #1, not FunctionKey(x)
}
```

> [!NOTE]
> Bindings also matter after name resolution. Later passes store facts per variable, such as its type, and keep them around. Keying those tables by binding instead of by name means `{ let x = 1; }` and `{ let x = true; }` never overwrite each other.

**Name resolution** is the process of determining which declaration (i.e., `let` binding, `const` definitions, or `func` definitions) each identifier in a program refers to.

In blueberry, name resolution works in two steps: 
1. Collect declarations and scope information into `ScopeTree`s, `DefinitionList`s, and `ImportList`s.
2. Resolve each use of a name by searching them with the `Resolver`.

### Collecting

Collecting finds the declarations in a file and stores them in data structures.

Collecting isn't one step that builds everything at once. Each data structure below is built by its own query, separately, the first time something needs it:
- one `ScopeTree` per function or constant definition, for its parameters and `let` bindings (`function_scopes_of`, `constant_scopes_of`): built right before that body is type checked.
- one `DefinitionList` for the file's top-level definitions (`file_scoped_definitions_of`): built first, since it's how the compiler finds out which definitions exist.
- one `DefinitionList` per block, for the definitions declared inside that block (`block_scoped_definitions_of`): built the first time the `Resolver` searches that block's scope.
- one `ImportList` for the file's import declarations (`imports_of`): built the first time the `Resolver` doesn't find a name locally, in a block, or at the file's top level.

So if every name in a file resolves before reaching imports, its `ImportList` is never built.

A `ScopeTree` is the tree of all scopes in one function definition's body or a constant definition's body. Each scope holds names, each paired with its binding, and a pointer to its parent: the scope around it. The tree also records the innermost scope every expression sits in. This is exposed via the `containing_scope` method. By exposing this method, the `Resolver` may know where to start looking up a use site (e.g., the `x` in `println(x)`).

A **scope** is a region of code where a set of names can be used. A new scope begins at the start of a function body (holding its parameters), at a block's `{`, or right after a `let`, and ends at the closing `}` of the enclosing block. A new scope nests inside the current one, so code in an inner scope can also see the names of every scope around it.

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

A `DefinitionList` is merely a flat list of the functions and constants declared at one level: either a file's top level, or a single block. Unlike a `ScopeTree`, it needs no nesting, because a definition is visible across its whole file or block, even before the line that declares it. 

Lastly, an `ImportList` is a flat list of paths from all the file's `import` declarations. Each path (e.g., `math::sqrt`) names a module and a definition in it: the `ModuleMap` finds the module's file, and the name is looked up in that file's top-level `DefinitionList`. Only top-level definitions can be imported, since nested ones are private to their block.

> [!NOTE]
> We may think of the `ScopeTree`(s), `DefinitionList`(s), and the `ImportList` as a disconnected symbol table.

### Resolving

Blueberry's `Resolver` attempts to resolve a given name in the following order:
1. each scope, starting from the given name's innermost scope and moving outward: its `let` bindings, then, if the scope is a block, the nested definitions declared in that block if any
2. top-level definitions in the current file
3. import declarations (representing top-level definitions in other files)

This way, the closest declaration of a name always wins: the most recent `let` beats an earlier one, an inner scope beats an outer one, and a file's top-level definitions beat imported ones.

## Type Checking

**Type checking** is the process of verifying that a program uses its types consistently, reporting a type error wherever it doesn't. This means two things:
- **bodies**: every expression is assigned a type, and each one must fit where it's used
- **`trait`s and `impl`s**: every `impl` must match the `trait` it implements 

### Type Inference

Languages can be classified by which declarations (definitions and locals) may omit their type annotations:
- **Local type inference**: function signatures are annotated, but declarations inside a body may omit their annotations.
- **Full type inference**: any declaration may omit its annotation, even a function signature.

There are two common type inference techniques:
- **Immediate inference** takes the type straight from the expression (e.g. Go's `x := 5`). It only works when an expression's type is always fully known on the spot.
- **Constraint-based inference** works both when an expression's type is always fully known on the spot, but also when the type arrives later, (e.g. Rust's `let v = Vec::new(); v.push(1);`, where `v`'s element type is only known on the second line). Constraint-based inference involves 4 steps:
1. Introduce unknowns
2. Generate constraints
3. Solve constraints
4. Apply the substitution

Constraints can be solved eagerly, as soon as they're generated, so steps 1–3 happen together in a single walk over the body; or delayed, collected during the walk and solved after it. Either way, step 4 runs once at the end.

Blueberry supports local type inference, and infers with constraints.

#### Step 1: Introduce Unknowns

For each type that isn't known yet, the type checker creates a placeholder called an **inference variable** $?i$. It creates one for:
- an unannotated local (e.g., `let x = ...`)
- an integer literal with no concrete expected type (e.g., the `5` in `let x = 5`)

A type is _already_ known when the user writes an explicit type annotation (e.g., `let x: I32 = e`), or when the value is a `Bool`, so no inference variable is needed.

A type can be fully concrete (e.g. `I32`), an inference variable, or a compound type containing inference variables (e.g. `func (?0) -> I32`).

Following the approach of Milner's **Algorithm J**, inference variables are stored in an **inference table**: when an inference variable is solved, the type it's solved to (its **solution**) is written once into the table, rather than updating every type that mentions it, as done in **Algorithm W**. The inference table is designed as an extension of a disjoint set. A group of inference variables known to be equal is stored as a disjoint set, where the group's common type is the disjoint set's representative value. Non-integer inference variables and integer inference variables live in separate disjoint sets, so that an integer variable may _only_ ever become an integer type.

Since solutions are looked up in the inference table rather than written into every type, a type may mention an inference variable that has _already_ been solved (i.e., is no longer unknown). This means that before looking at a type, the type checker must **shallowly resolve** (via the `.shallow_resolve(ty)` method on the inference table) it. Shallow resolving means following chains of inference variables if any, but it won't recurse in the inner types (e.g., `fn(?0) -> I32` is returned as is).

#### Step 2: Generate Constraints

The type checker applies the language's rules to the code to produce **constraints**: rules the types must follow. 

For blueberry, there are two kinds of constraints that the type checker needs to solve:
- **Equality** constraints
- **Obligation** constraints

Which constraints are generated, and when they are solved, depends on what is being type checked: see [Type Checking Bodies](#type-checking-bodies) and [Type Checking Traits](#type-checking-traits).

> [!NOTE]
> As of now, since Blueberry doesn't yet have traits, the type checker only builds equality constraints. 

#### Step 3: Solve Constraints

The type checker solves the constraints by building a **substitution**: a mapping from each inference variable to a type.

Each equality constraint is solved by `unify(a, b)`, which makes the two types equal, or reports a mismatch if they can't be. Each obligation is solved by the **trait solver**, which also involves solving equality constraints.

##### Unification Algorithm

In `unify(expected, found)`, `expected` is the type the context requires (e.g. an annotation, a parameter, `Bool` for a condition) and `found` is the type the expression actually has, so a mismatch can be reported as "expected `Bool`, found `I32`".

First, the unification algoritm must shallow resolve `a` and `b`. Without shallow resolving a type, `unify` might mistake a solved variable for an unsolved one. For example: in the following code, if `unify` treated the `?0` on the last line as still unknown, it would set `?0 := I32`, thereby overwriting `Bool`, and missing the type mismatch error (`x` is a `Bool`, but `y` expects an `I32`).

```
let x = foo();    // x: ?0
if x { }          // unify(Bool, ?0)  → ?0 := Bool
let y: I32 = x;   // unify(I32, ?0)   → must see Bool, and report a mismatch
```

After shallow resolving both sides, `unify` dispatches on the two shapes:

| `a` | `b` | Action |
|---|---|---|
| `t` | `t` (the same type) | ok (types are interned, so this is a cheap `==`) |
| `Error` | anything (either side) | ok, so one bad type doesn't cause a pile of follow-up errors (the "poisoning" trick from D's compiler: see Walter Bright's [Improving Compiler Error Messages](https://digitalmars.com/articles/b47.html)) |
| `?x` (general) | `?y` (general) | `union(?x, ?y)`, which merges the two groups (both are unsolved, so it can't fail) |
| `?x` (general) | any type `t` | occurs check, then `set_value(?x, Some(t))` |
| any type `t` | `?x` (general) | same as the row above except in the other direction; perform action as the row above |
| `?i` (integer) | `?j` (integer) | `union(?i, ?j)` |
| `?i` (integer) | `I32` / `I64` / `U32` / `U64` | `set_value(?i, Some(t))` |
| `I32` / `I64` / `U32` / `U64` | `?i` (integer) | `set_value(?i, Some(t))` |
| `?i` (integer) | `Bool`, `fn`, … | mismatch |
| `fn(p1, ..) -> r1` | `fn(q1, ..) -> r2` | same number of parameters? then unify each $p_i$ with $q_i$, and `r1` with `r2` |
| anything else | | `Err(TypeMismatch { expected, found })` |

The function row is the recursive part: rather than comparing two types with `==`, unification walks into both and unifies the pieces, so `fn(?0) -> I32` and `fn(Bool) -> I32` unify with `?0 := Bool`.

Before setting `?x := t`, `unify` performs an **occurs check**, i.e., it checks that `?x` doesn't occur anywhere inside `t`. Otherwise, the solution would have to contain itself, which would be a type error. For example:

```text
?x = fn(?x) -> I32
   = fn(fn(?x) -> I32) -> I32
   = fn(fn(fn(?x) -> I32) -> I32) -> I32
   = ...forever
```

#### Step 4: Apply the Substitution

The last step involevs applying the substitution, i.e., every inference variable is recursively replaced with its type. 

An inference variable without a substitution becomes a type error, while an integer inference variable defaults to `I32`.

### Type Checking Bodies

There are two kinds of **body**, i.e., code the type checker walks:
- a **function body**: the block of a `func`
- a **constant body**: the initializer of a `const`

Each body is checked on its own, using only the _signatures_ of other definitions.

While walking a body, the type checker generates, then solves each equality constraint eagerly. This is because later code may need a type _now_. For instance, a method call like `x.count()` needs `x`'s type to find `count`.

#### Function bodies

1. Each parameter is given its declared type, so inside the body, `x` in `func f(x: I32)` already has type `I32`.
2. The declared return type `r` is remembered (`()` if there's no `-> <type>`), so that each `return e` inside the body can generate `unify(r, ty(e))`.
3. The body's block is walked, generating constraints for every statement and expression inside it.
4. The block's type, i.e., the type of its tail expression, or `()` if there's none, is unified with `r`.

For example:

```
func f(x: I32) -> I64 {   // x: I32, r = I64
    let y = 5;            // y: ?0, where ?0 is an integer variable
    if x > 0 {            // unify(ty(x), ty(0)): I32 = ?1 → ?1 := I32
        return y;         // unify(I64, ?0)                → ?0 := I64
    }
    y                     // tail: unify(I64, ty(y))       → I64 = I64, ok
}
```

#### Constant bodies

Since constant bodies contain are no parameters and no `return`, only the initializer is walked, and its type is unified with the annotated type (which is mandatory in blueberry):

```
const LIMIT: I64 = 5;   // 5: ?0, an integer variable
                        // unify(I64, ?0) → ?0 := I64
```

As the type checker walks either a function body, or a constant body, the following specific equality constraints are generated for each definition, statement, and expression. Below, `ty(e)` is the type the type checker found for `e`:

| Code | Equality constraints | Type of the expression |
|---|---|---|
| `func f(..) -> r { body }` | `unify(r, ty(body))` | — |
| `const X: T = e` | `unify(T, ty(e))` | — |
| `5` | none, a fresh integer variable `?i` is created | `?i` |
| `true`, `()` | none | `Bool`, `()` |
| `x` (a local or a definition) | none, its type is looked up | the type of `x` |
| `let x: T = e` | `unify(T, ty(e))` | `x: T` |
| `let x = e` | none, `x` simply takes `e`'s type | `x: ty(e)` |
| `if c { a } else { b }` | `unify(Bool, ty(c))`, `unify(ty(a), ty(b))` | `ty(a)` |
| `if c { a }` | `unify(Bool, ty(c))`, `unify((), ty(a))` | `()` |
| `{ s1; s2; e }` | none | `ty(e)`, or `()` if there's no tail |
| `f(a1, .., an)` where `f: fn(p1, .., pn) -> r` | `unify(pi, ty(ai))` for each argument | `r` |
| `-e` | `ty(e)` must be a signed integer type, or an integer variable that ends up signed | `ty(e)` |
| `!e` | `unify(Bool, ty(e))` | `Bool` |
| `a + b`, `a - b`, `a * b`, `a / b` | `unify(ty(a), ty(b))`, which must be an integer | `ty(a)` |
| `a < b`, `a == b`, … | `unify(ty(a), ty(b))` | `Bool` |
| `a && b`, `a \|\| b` | `unify(Bool, ty(a))`, `unify(Bool, ty(b))` | `Bool` |
| `a = b` | `unify(ty(a), ty(b))` | `()` |
| `return e` | `unify(r, ty(e))`, where `r` is the function's return type | `Bottom` |
| `loop { .. }` | each `break e` inside: `unify(?b, ty(e))`, where `?b` is a fresh variable for the loop | `?b` (`Bottom` if it never breaks) |
| `while c { .. }` | `unify(Bool, ty(c))` | `()` |
| `continue` | none | `Bottom` |

`Bottom` (the type of `return`, `break`, and `continue`) never produces a value, so it should fit wherever any type is expected. That isn't equality, so it isn't handled by `unify` but by **coercion**, which runs before `unify`.

### Type Checking Traits

Blueberry doesn't have traits yet

## All Together

This section puts the pieces above together by following the flow of semantic analysis on a fresh run, where nothing has been computed yet:

1. Stable IDs are assigned
2. Definitions are collected
3. Signatures are lowered (AST → HIR)
4. Bodies are lowered (AST → HIR)
5. Scopes are built
6. Signature types are lowered (HIR → Ty)
7. Names are resolved and bodies are type checked

Steps 1–5 only deal with _names_: what is declared, and where. Steps 6 and 7 deal with _types_.

Each step is explained with this program, made of two files:

```
// math.bb
func square(n: I32) -> I32 {
    n * n
}
```

```
// main.bb
import math::square;

func double(n: I32) -> I32 {
    n + n
}

func main() {
    func is_ready() -> Bool { true }
    let x = double(5);
    let y = square(x);
}
```

### 1. Stable IDs are assigned

The CST is walked once, and every function, constant and block is given an ID.

The ID is stable because it describes _what_ a node is (its parent, kind and name) rather than _where_ it sits in the file. Because of that, it is used to build the `FunctionKey`, `ConstantKey` or `BlockKey` so that editing one function, constant, or block leaves every other key, and so every other cached result, unchanged.

In the example: in `main.bb`, `double`, `main`, `is_ready`, and each of their blocks get an ID. In `math.bb`, `square` and its block get an ID.

_In the code:_ `red_node_directory_of(file)`

### 2. Definitions are collected

As part of name resolution, top-level definitions and nested definitions are each collected and stored in a `DefinitionList`: one for the file, and one per block that declares definitions. Each definition is stored as its key: its ID from step 1, wrapped in a `FunctionKey` or `ConstantKey`. Every later step is asked for by key, e.g. "lower the body of `FunctionKey(double)`". The file's imports are collected into an `ImportList`.

In the example: `main.bb`'s list is `[FunctionKey(double), FunctionKey(main)]`, `main`'s block's list is `[FunctionKey(is_ready)]`, and `main.bb`'s `ImportList` is `[math::square]`. `math.bb`'s list is `[FunctionKey(square)]`.

_In the code:_ `file_scoped_definitions_of(file)`, `block_scoped_definitions_of(block)`, `imports_of(file)`

### 3. Definition Signatures are lowered (AST → HIR)

Every definition's header is lowered into an HIR node: a `FunctionSignature` (its name, parameter annotations and return annotation) or a `ConstantSignature` (its name and type annotation). The annotations are still just the _names_ the user wrote, e.g. `I32`; they become types in step 6.

A signature is kept separate from its body, so that `f` can call `g` using only `g`'s signature, without `g`'s body being checked first. This matters for two reasons:
- **stability**: editing inside `g`'s body doesn't change `g`'s signature, so `f`'s cached result stays valid and `f` isn't checked again
- **recursion**: if `f` calls `g` and `g` calls `f`, neither needs the other's body, so checking them doesn't loop forever

This only works because blueberry requires every signature to be fully annotated, and so the header alone says what the type is.

In the example: `double`'s signature has the parameter annotation `I32` and the return annotation `I32`. `main`'s has no parameters and no return annotation.

_In the code:_ `function_signature_of(f)`, `constant_signature_of(c)`

### 4. Definition Bodies are lowered (AST → HIR)

Every body is lowered into a **HIR** (a `DefinitionBody`) consisting of flat tables of expressions, statements and local bindings, each referred to by a handle. Every `let` and parameter becomes its own **binding**, so two `x`s are never confused (see [Name Resolution](#name-resolution)).

For diagnostics, a source map is also built, storing information about which HIR node came from which AST node.

In the example: `n` becomes a binding in `double` (and a separate one in `square`), and `x` and `y` become bindings in `main`.

_In the code:_ `function_body_of(f)`, `constant_body_of(c)`
 
### 5. Scopes are built for every Definition's Body

Each body's HIR is walked to build a `ScopeTree`: a tree of scopes, each holding the bindings declared in it, plus a table saying the enclosing scope of every expression.

In the example: `double` has a scope holding `n`. `main` has a scope for its block, a scope holding `x` that starts after its `let`, and inside that, a scope holding `y`.

_In the code:_ `function_scopes_of(f)`, `constant_scopes_of(c)`

### 6. Signature types are "lowered" (HIR → Ty)

Each annotation in a signature from step 3 is turned into a type the type checker can use (e.g. the name `I32` into the type `I32`), and the whole signature becomes one type, with `()` as the return type if none is written.

Nothing is checked here: every type in a signature is written down, so there is nothing to infer, and an unknown type name simply becomes the `Error` type. Bodies are different: most of their expressions have no written type, so their type level (step 7) must infer and check.

In the example: `double: fn(I32) -> I32`, `main: fn() -> ()`, `is_ready: fn() -> Bool`, and `square: fn(I32) -> I32`.

_In the code:_ `function_signature_type_of(f)`, `constant_signature_type_of(c)`

### 7. Names are resolved and definition bodies are type checked

Name resolution and type checking happen together, in one walk over each body: a name is resolved at the moment the type checker reaches it, because its type is needed right then. Steps 2 and 5 only _collect_ what names can refer to, but resolving each use happens here.

Every body, including the bodies of nested definitions, is type checked on its own (see [Type Checking](#type-checking)). First, the checker is _seeded_ with the definition's signature type from step 6: each parameter gets its declared type, and the return type is remembered. Then the body is walked once, and for every expression:
1. any name is resolved through the `Resolver`, which walks from the innermost scope outward, checking each scope's local bindings and then that block's definitions, before trying the file's definitions, then imports
2. the expression is given a type, using the expected type if one is passed down
3. each equality is solved right away with `unify`, recording a mismatch if it fails

The result is each expression's type, each binding's type, and the list of mismatches.

In the example, for `main`:
- `double` isn't a local, nor in `main`'s block's list (`[is_ready]`), so it's found in the file's list, and its signature `fn(I32) -> I32` is used
- `5` is checked against the parameter type `I32`, so it's given `I32` directly
- `x` gets the call's return type, `I32`
- `square` isn't a local, nor in `main`'s block's list, nor in `main.bb`'s list, so the imports are tried: `math::square` ends in `square`, the `ModuleMap` finds `math.bb`, and `square` is found in `math.bb`'s list. Its signature `fn(I32) -> I32` is used
- `x` (the local, found in the innermost scopes) has type `I32`, matching the parameter type, so `y` gets `I32`

`double`'s `n + n` and `square`'s `n * n` give `I32`, matching their return types, and `is_ready`'s `true` gives `Bool`, matching its return type. No mismatches.

_In the code:_ `function_types_of(f)`, `constant_types_of(c)`
