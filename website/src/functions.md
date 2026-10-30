# Functions

A function lists its parameters with their types, then its return type after `->`. A function that returns nothing (`()`) leaves out the `->` part, like `main`.

A function's body is a block, and a block's value is its last expression, written without a `;`. That's what the function returns: here, `x`. Use `return` to return early.

```
func abs(x: I32) -> I32 {
    if x < 0 {
        return -x;
    }
    x
}
```

A function can be used before it's defined. Functions and constants can also be defined inside a block, where they can be used anywhere in that block. Unlike a closure, though, a function defined inside another one can't use that function's variables.

```
func main() {
    let y = double(21);

    func double(x: I32) -> I32 {
        x * 2
    }
}
```

`return` never finishes, so you can use it wherever a value is expected: here, the `else` branch returns `b` from the whole function.

```
func first_positive(a: I32, b: I32) -> I32 {
    let x = if a > 0 { a } else { return b; };
    x
}
```
