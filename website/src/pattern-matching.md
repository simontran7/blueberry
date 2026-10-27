# Pattern matching

> [!CAUTION]
> Unimplemented

```
func area(shape: Shape) -> I32 {
    match shape {
        Shape::Point => 0,
        Shape::Circle(r) => r * r,
        Shape::Rectangle { width, height } => width * height,
    }
}
```

A `match` compares a value against a list of patterns, and runs the arm of the first one that matches, taking the value apart into the names in the pattern (here, `r`, `width`, and `height`). The arms must cover every possible case. Like `if`/`else`, a `match` produces a value, so all of its arms must have the same type.

```
func manhattan(p: Point) -> I32 {
    let Point { x, y } = p;
    abs(x) + abs(y)
}
```

A `let` can take a value apart too, when its pattern always matches, like a record's.

```
match shape {
    Shape::Circle(_) => 1,
    _ => 0,
}
```

`_` matches anything, without giving it a name: use it for a catch-all arm, or for a field you don't need.

```
func div_mod(a: I32, b: I32) -> (I32, I32) {
    (a div b, a mod b)
}

func main() {
    let (q, r) = div_mod(7, 2);
}
```

A tuple pattern takes an [unnamed tuple](algebraic-data-types.md#unnamed-tuples) apart: `(q, r)` matches a tuple of two elements, and names each of them. Here, `q` is `3` and `r` is `1`.

## Binding a whole pattern with `@`

```
match shape {
    c @ Shape::Circle(r) => biggest(c, r),
    _ => shape,
}
```

`c @` names the whole matched value `c`, while the pattern after it still takes it apart (`r`).

## Testing a pattern with `is`

> [!CAUTION]
> Unimplemented

```
if shape is Shape::Circle(r) {
    println(r);
}

while next_shape() is Shape::Circle(r) {
    println(r);
}

let is_circle = shape is Shape::Circle(_);
let is_big_circle = shape is Shape::Circle(r) and r > 10;
```

`is` checks whether a value matches a pattern, and gives `true` or `false`. Unlike `match`, it's just another expression, so it works as the condition of an `if` or a `while`, and anywhere else a `Bool` does.

A name bound by `is` can only be used where the match is known to have succeeded: in the rest of an `and`, in the `then` branch of an `if`, and in the body of a `while`. It can't be used in an `else` branch, in the rest of an `or`, or after the expression.

```
let has_big_circle = (a is Shape::Circle(r) or b is Shape::Circle(r)) and r > 10;
```

Both sides of an `or` must bind the same names, with the same types, so that each name is bound whichever side matched.

`is` binds tighter than `and` and `or`, and looser than comparison and arithmetic: `a is Shape::Circle(r) and r > 10` is `(a is Shape::Circle(r)) and (r > 10)`.

In a condition, a pattern with braces must be in parentheses: `if p is (Point { x, y }) { … }`, for the same reason as [records](algebraic-data-types.md#named-records).

## Guards with `when`

```
match shape {
    Shape::Circle(r) when r > 10 => 1,
    Shape::Circle(_) => 0,
    _ => -1,
}
```

`when` adds a condition to an arm, checked only once its pattern matches. If the condition is false, matching moves on to the next arm.
