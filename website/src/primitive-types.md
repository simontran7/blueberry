# Primitive types

| Type   | Meaning                  |
| ------ | ------------------------ |
| `I32`  | 32-bit signed integer    |
| `I64`  | 64-bit signed integer    |
| `U32`  | 32-bit unsigned integer  |
| `U64`  | 64-bit unsigned integer  |
| `Bool` | `true` / `false`         |
| `()`   | unit, i.e., the empty tuple |

> [!NOTE]
> An integer literal like `1` takes whatever integer type it's used as. If nothing decides, it's an `I32`.

```
let big: U64 = 1_000_000;
let small: I32 = -2_147_483_648;
```

Underscores group digits, and don't change the value. A literal must fit in its type: `let x: I32 = 3_000_000_000;` is an error.
