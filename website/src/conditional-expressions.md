# Conditional Expressions

`if`/`else` produces a value, so you can use it anywhere an expression goes. Both branches must have the same type.

```
let d: I32 = if a > THRESHOLD { abs(a) } else { clamp(a, 0, THRESHOLD) };
```

`else if` chains conditions. An `if` without an `else` produces nothing (`()`), so its branch can't produce a value either.

```
let sign = if x < 0 { -1 } else if x == 0 { 0 } else { 1 };

if x < 0 {
    x = 0;
}
```
