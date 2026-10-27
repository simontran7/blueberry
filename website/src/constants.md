# Constants

```
const THRESHOLD: I32 = 10;
const ENABLED: Bool = true;
const LIMIT: I32 = THRESHOLD * 2 + 1;
```

A constant gives a name to a value. Unlike a `let`, it always needs a type annotation. Its value can be any expression of that type, including one that uses other constants: here, `LIMIT` is `21`.

A constant can't be assigned to: `THRESHOLD = 5;` is an error. For a value that changes, use a `mut` [variable](variables.md) instead.

```
func main() {
    let x = BOUND;
    const BOUND: I32 = 5;
}
```

Like a function, a constant can be used before it's defined, and can be defined inside a block, where it can be used anywhere in that block.

Constants can also belong to a type, like `Point::ORIGIN`: see [Associated Constants](methods-and-traits.md#associated-constants).
