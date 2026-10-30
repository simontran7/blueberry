# Variables

`let` introduces a variable. The type annotation is optional: leave it out, and the type is inferred.

Variables are immutable by default. Mark one `mut` to be able to assign to it again.

```
let x: I32 = 5;
let mut result = x;
result = result + 1;
```

> [!CAUTION]
> Unimplemented: the compiler doesn't check `mut` yet, so today it also lets you assign to a variable that isn't `mut`.

A `let` with a name that's already in use introduces a new variable, which hides the old one from then on. Here, the second `x` is `6`.

```
let x = 5;
let x = x + 1;
```
