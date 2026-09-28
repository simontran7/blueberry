# Semantic Analyis

## Name Resolution

...

## Type Checking

**Type checking** is the process of assigning a type to every expression and verifying that each one is used consistently with it, reporting a type error wherever they don't.

Whenever a type isn't written, the type checker has to **infer** it. Languages can be classified by which declarations (definitions and locals) may omit their type annotations:
- **Local inference**: function signatures are annotated, but declarations inside a body may omit their annotations.
- **Full inference**: any declaration may omit its annotation, even a function signature.

There are two common inference techniques:
- **Immediate inference** takes the type straight from the expression (e.g. Go's `x := 5`). It only works when an expression's type is always fully known on the spot.
- **Constraint-based inference** works both when an expression's type is always fully known on the spot, but also when the type arrives later, (e.g. Rust's `let v = Vec::new(); v.push(1);`, where `v`'s element type is only known on the second line). Constraint-based inference involves 4 steps:
1. Introduce unknowns
2. Collect constraints
3. Solve constraints
4. Apply the substitution

Blueberry supports local inference and infers with the constraint-based technique.

### Step 1: Introduce Unknowns

For each type that isn't known yet, the type checker creates a placeholder called an **inference variable** (`?0`, `?1`, etc.). It creates one for:
- an unannotated local, e.g. `let x = ...`
- an integer literal with no concrete expected type, e.g. the `5` in `let x = 5`

Types that are _already_ known when the user writes an explicit type annotation (e.g. `let x: i32 = e`), or when the value is a `bool`.

A type can be fully concrete (e.g. `i32`), an inference variable, or a compound type containing inference variables (e.g. `func (?0) -> i32` which is definitely a function, but the parameter type is unknown).

Inference variables live in the **inference table**. It tracks them with a disjoint set, where each set's representative holds the type known so far (`None` while unknown). General and integer inference variables live in separate disjoint sets, so an integer variable can only ever become an integer type.

### Step 2: Collect Constraints

The type checker applies the language's rules to the code to produce **constraints**: rules the types must follow. 

There are differnt constraints which each are solved by different algorithms:
- **equality** constraints (e.g., `if c { .. }` has the constraint that `condition` must be a `bool`)
- **trait** constraints
- **subtyping** constraints

Blueberry only has equality constraints.

Constraints can't always be collected first and solved later. A method call like `x.len()` needs `x`'s type _now_ to find `len`, so rust-analyzer solves equalities **eagerly** (as soon as they're produced) and delays trait constraints until more is known. Blueberry has no traits, so it simply unifies every constraint the moment it's produced.

### Step 3: Solve Constraints

The type checker solves the constraints by building a **substitution**: a mapping from each inference variable to a type.

Each equality constraint is solved by `unify(a, b)`. It first **shallow resolves** both sides: if a side is `?3` and `?3` is already known to be `i32`, it's treated as `i32`. Then it dispatches on the two shapes:

| `a` | `b` | Action |
|---|---|---|
| `?x` (general) | `?y` (general) | `union(?x, ?y)` |
| `?x` (general) | any type `t` | occurs check, then `set_value(?x, Some(t))` |
| `?i` (integer) | `?j` (integer) | `union(?i, ?j)` |
| `?i` (integer) | `i32` / `i64` / `u32` / `u64` | `set_value(?i, Some(t))` |
| `?i` (integer) | `bool`, `fn`, … | mismatch |
| `fn(p1, ..) -> r1` | `fn(q1, ..) -> r2` | same number of parameters? then unify each `pᵢ` with `qᵢ`, and `r1` with `r2` |
| `i32` | `i32` (same primitive) | ok |
| `Error` | anything | ok, to avoid a pile of follow-up errors |
| anything else | | mismatch |

The function row is the recursive part: rather than comparing two types with `==`, unification walks into both and unifies the pieces, so `fn(?0) -> i32` and `fn(bool) -> i32` unify with `?0 := bool`.

**Occurs check.** Before setting `?x := t`, unification checks that `?x` doesn't occur anywhere inside `t`. Otherwise the answer would have to contain itself:

```text
?x = fn(?x) -> i32
   = fn(fn(?x) -> i32) -> i32
   = fn(fn(fn(?x) -> i32) -> i32) -> i32
   = ...forever
```

No finite type satisfies that, so it's reported as an error. The check isn't specific to generics: it's needed whenever an inference variable can be unified with a type that contains other types, which in Blueberry means function types. Generics make it more common (e.g. `?x = Vec<?x>`).

### Step 4: Apply the Substitution

Once the body has been walked, the type checker applies the substitution: every inference variable is replaced with its type, recursively. 

Variables with no answer fall back:
- An integer variable still `None` defaults to `i32`.
- A general variable still `None` is an error: "type annotations needed".


**Bidirectional typing.** As the type checker moves through the code, a type can flow **up** (from the expression out to its context) or **down** (from the context into the expression). Blueberry follows the bidirectional typing technique (Dunfield & Krishnaswami, [_Bidirectional Typing_](https://arxiv.org/abs/1908.05839)), which has two modes:
- **Checking mode**: the expected type is known and _pushed down_ (e.g. from annotations, return types, and call arguments). It produces better-localized error messages ([source](https://jaked.org/blog/2021-09-07-Reconstructing-TypeScript-part-0#:~:text=One%20way%20this%20makes%20the%20type%20checker%20more%20usable%20is%20by%20localizing%20errors.)).
- **Inference mode**: the type is _inferred_ from the expression's structure alone, when no expected type is known.

```text
let x: i32 = if c { 1 } else { 2 };

             let x: i32
                 │   i32 flows DOWN ↓ into the if
                if
      ┌──────────┼──────────┐
    c  ↓ bool    1  ↓ i32   2  ↓ i32

and for `let y = x + 1`:
    x ↑ i32   (looked up, flows UP)
```
