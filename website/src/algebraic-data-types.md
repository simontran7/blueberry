# Algebraic Data Types

## Tuples

### Unnamed Tuples

A tuple groups a fixed number of values, which can have different types. You reach its elements by index (`pair.0`, `pair.1`), and in a `mut` tuple, you can assign them one at a time.

A tuple of one element needs a trailing comma: `(5,)`, since `(5)` is just `5` in parentheses.

```
func swap(pair: (I32, Bool)) -> (Bool, I32) {
    (pair.1, pair.0)
}

func main() {
    let mut point: (I64, I64) = (1, 2);
    point.0 = 3;
    let one = (5,);
    let nested = ((1, 2), true);
    let deep = nested.0.1;
}
```

These tuples are unnamed: their type is just the types of their elements, like `(I32, Bool)`. To give a tuple type a name, see [Named Tuples](#named-tuples).

### Named Tuples

> [!CAUTION]
> Unimplemented

A named tuple is like a named record, but its fields are positional: you reach them by index, `.0`, `.1`, and so on.

```
tuple Point(I32, I32);

func main() {
    let p: Point = Point(1, 2);
    let sum: I32 = p.0 + p.1;
}
```

Unlike an [unnamed tuple](#unnamed-tuples), like `(1, 2)`, a named tuple has a name, so `Point(1, 2)` and `(1, 2)` have different types, even though both hold two `I32`s.

## Records

### Unnamed Records

Unlike some languages, blueberry has no unnamed records: every record belongs to a declared [named record](#named-records). In TypeScript, for example, `{ x: 1, y: 2 }` is a value of its own, without a type declared for it. In blueberry, you declare `record Point` first, then write `Point { x: 1, y: 2 }`.

### Named Records

> [!CAUTION]
> Unimplemented

A named record is a type with a name, made of named fields (the same thing as a `struct`).

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

> [!NOTE]
> In the condition of an `if` or a `while`, and right after `match`, put a record in parentheses: `if p == (Point { x: 1, y: 2 }) { … }`. Without them, the `{` after `Point` would look like the start of the `if`'s body.

## Variants

> [!CAUTION]
> Unimplemented

A variant is a named type whose value is one of several cases. Each case can carry nothing, positional fields, or named fields.

```
variant Shape {
    Point,
    Circle(I32),
    Rectangle { width: I32, height: I32 },
}
```

You create a variant's value through one of its cases: a case without fields is just its name, a case with positional fields is called like a function, and a case with named fields is written like a record.

```
let a = Shape::Point;
let b = Shape::Circle(5);
let c = Shape::Rectangle { width: 3, height: 4 };
```
