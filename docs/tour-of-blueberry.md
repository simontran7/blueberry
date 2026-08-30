# A Tour of Blueberry

> [!CAUTION]
> Sections marked `Unimplemented` describe language design that has been decided but is not yet wired through the compiler. 

## Hello, blueberry!

Every program is a flat list of top-level definitions: `func` and `const`. 

```
func main() {
    let x: I32 = 1 + 2;
    println(x);
}
```

## Comments

Only line comments, starting with `//` and running to the end of the line. There are no block comments.

```
// leading comment
func f() -> I32 {
    6 / 2 // a single slash is division, not a comment
}
```

## Constants

`const <name>: <type> = <value>;` at the top level. The type annotation is mandatory (unlike `let`).

```
const THRESHOLD: I32 = 10;
const ENABLED: Bool = true;
```

## Variables

`let` bindings are immutable by default; `mut` opts into reassignment. The type annotation is optional and inferred when omitted.

```
let x: I32 = 5;
let mut result = x;
result = result + 1;
```

## Primitive types

| Type   | Meaning                  |
| ------ | ------------------------ |
| `I32`  | 32-bit signed integer    |
| `I64`  | 64-bit signed integer    |
| `U32`  | 32-bit unsigned integer  |
| `U64`  | 64-bit unsigned integer  |
| `Bool` | `true` / `false`         |
| `()`   | unit — the "no value" type |

An integer literal like `1` isn't pinned to a width until it's used somewhere that constrains it; left unconstrained, it defaults to `I32`.

## Functions

`func <name>(<parameters>) -> <return type> { <body> }`. The `-> <return type>` is omitted for a function returning `()`. A function's body is a block expression, so its last expression (without a trailing `;`) is the return value (though an explicit `return` works). You can of course `return` early.

```
func abs(x: I32) -> I32 {
    if x < 0 {
        return -x;
    }
    x
}
```

## Operators

- Arithmetic: `+ - * /`
- Comparison: `< > <= >= == !=`
- Logical: `and`, `or`, `not`

```
if a < b and b < c {
    score = score + 3;
} else if a == b or not (b == c) {
    score = score + 1;
}
```

## `if` as an expression

`if`/`else` produces a value, so it can appear anywhere an expression can, where both branches must agree on type.

```
let d: I32 = if a > THRESHOLD { abs(a) } else { clamp(a, 0, THRESHOLD) };
```

## Loops

There are two loops: while loops, and infinite loops. 

Additionally, there are two loop keywords: the `continue` keyword to skip the rest of the body and re-check the loop's condition, and the `break` keyword to exit the loop completely.

While loops are in the form of `while <condition> { <body> }`. They run `<body>` for as long as `<condition>` holds, or optionally, with the use of `break`. 

Infinite loops are in the form of `loop { <body> }`, where they running forever until a `break` is introduced (pretty much mandatory for it to be any useful). 

`while` loops always evaluates to `()`: it can always also exit normally (the condition going false), so there's nothing to guarantee a value. 

infinite `loop`s are different, as a `break` is its only way out, it can produce a value via `break value;`, and its type is whatever every `break <value>` inside it agrees on.

```
func sum_below(n: I32) -> I32 {
    let mut i: I32 = 0;
    let mut total: I32 = 0;
    while i < n {
        i = i + 1;
        if i == 5 {
            continue;
        }
        total = total + i;
    }
    total
}
```

```
func first_above(n: I32) -> I32 {
    let mut i: I32 = 0;
    loop {
        i = i + 1;
        if i > n {
            break;
        }
    }
    i
}
```

```
func first_above_doubled(n: I32) -> I32 {
    let mut i: I32 = 0;
    loop {
        i = i + 1;
        if i > n {
            break i * 2;
        }
    }
}
```

## Modules

> [!CAUTION]
> Unimplemented

The filesystem *is* the module tree! Every `.bb` file is implicitly a module, and its path relative to the project root is its import path. A directory is a namespace simply by containing files.

Nothing in a file is visible to importers unless marked `pub`. Bringing another file into scope is done via `import "<path>" [as <name>];` 

```
// shapes/circle.bb
pub const PI: I32 = 3;

pub func area(radius: I32) -> I32 {
    radius * radius * PI
}

func helper(x: I32) -> I32 {  
    x
}
```

```
// main.bb
import "shapes/circle" as circle;

func main() {
    println(circle::area(3));
}
```

## Algebraic Data Types

### Records

> [!CAUTION]
> Unimplemented

A record is a named product type (the same thing as a `struct`).

```
record Point {
    x: I32,
    y: I32,
}

func main() {
    let p: Point = Point { x: 1, y: 2 };
    let sum: I32 = p.x + p.y;
}
```

### Named Tuples

> [!CAUTION]
> Unimplemented

A `tuple` declares a named product type whose fields are positional instead of named. Fields are accessed by index: `.0`, `.1`, etc.

```
tuple Point(I32, I32);

func main() {
    let p: Point = Point(1, 2);
    let sum: I32 = p.0 + p.1;
}
```

### Variants

> [!CAUTION]
> Unimplemented

A variant is named sum type. 

```
variant Shape {
    Point,
    Circle(I32),
    Rectangle { width: I32, height: I32 },
}
```

## Pattern matching

> [!CAUTION]
> Unimplemented

A `match` expression destructures a value against a set of patterns. Matches must be exhaustive. `match` is an expression, so all arms must agree on type, just like `if`/`else`.

```
func area(shape: Shape) -> I32 {
    match shape {
        Shape::Point => 0,
        Shape::Circle(r) => r * r,
        Shape::Rectangle { width, height } => width * height,
    }
}
```

Patterns can also destructure records and tuple structs:

```
func manhattan(p: Point) -> I32 {
    let Point { x, y } = p;
    abs(x) + abs(y)
}
```

`_` is the wildcard pattern, matching anything without binding it — used for a catch-all arm or an ignored field.

```
match shape {
    Shape::Circle(_) => 1,
    _ => 0,
}
```

### Binding a whole pattern with `is`

`is`: `<name> is <pattern>` matches `<pattern>` and also binds the whole matched value to `<name>`.

```
match shape {
    c is Shape::Circle(r) => biggest(c, r),
    _ => shape,
}
```

### Guards with `when`

An extra condition on an arm, checked only after the pattern matches. If the guard fails, matching falls through to the next arm.

```
match shape {
    Shape::Circle(r) when r > 10 => 1,
    Shape::Circle(_) => 0,
    _ => -1,
}
```
