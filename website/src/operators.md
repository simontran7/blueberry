# Operators

## Arithmetic

| Operator | Meaning                                                           |
| -------- | ----------------------------------------------------------------- |
| `+`      | Addition                                                          |
| `-`      | Subtraction                                                       |
| `*`      | Multiplication                                                    |
| `div`    | Division, rounding down                                           |
| `mod`    | Remainder of `div`                                                |
| `-x`     | Negation, only on signed integers (`I8`, `I16`, `I32`, and `I64`) |

`div` rounds down, toward negative infinity, and `mod` is the remainder that goes with it, so that `a == b * (a div b) + a mod b`. A non-zero `a mod b` has the sign of `b`. For example, `-7 div 2` is `-4` and `-7 mod 2` is `1`, while `7 div -2` is `-4` and `7 mod -2` is `-1`.

## Comparison

| Operator | Meaning                  |
| -------- | ------------------------ |
| `<`      | Less than                |
| `>`      | Greater than             |
| `<=`     | Less than or equal to    |
| `>=`     | Greater than or equal to |
| `==`     | Equal to                 |
| `!=`     | Not equal to             |

Comparisons work on any two values of the same type.

## Logical

| Operator | Meaning                                     |
| -------- | ------------------------------------------- |
| `and`    | `true` if both sides are `true`             |
| `or`     | `true` if at least one side is `true`       |
| `not`    | Flips `true` to `false`, and `false` to `true` |

`and` and `or` only evaluate their right side when the left one doesn't already decide the answer.

```
if a < b and b < c {
    score = score + 3;
} else if a == b or not (b == c) {
    score = score + 1;
}
```

## Precedence

Arithmetic binds tighter than comparisons, which bind tighter than `and`, which binds tighter than `or`: `a + 1 < b and c or d` is `(((a + 1) < b) and c) or d`.
