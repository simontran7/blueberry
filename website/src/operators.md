# Operators

```
if a < b and b < c {
    score = score + 3;
} else if a == b or not (b == c) {
    score = score + 1;
}
```

- Arithmetic: `+ - * div mod`
- Comparison: `< > <= >= == !=`
- Logical: `and`, `or`, `not`

`div` rounds down, toward negative infinity, and `mod` is the remainder that goes with it, so that `a == b * (a div b) + a mod b`. A non-zero `a mod b` has the sign of `b`. For example, `-7 div 2` is `-4` and `-7 mod 2` is `1`, while `7 div -2` is `-4` and `7 mod -2` is `-1`.

Comparisons work on any two values of the same type, and `and` and `or` only evaluate their right side when the left one doesn't already decide the answer. Negation (`-x`) only works on signed integers (`I8`, `I16`, `I32`, and `I64`).

Arithmetic binds tighter than comparisons, which bind tighter than `and`, which binds tighter than `or`: `a + 1 < b and c or d` is `(((a + 1) < b) and c) or d`.
