# Loops

A `while` loop runs its body for as long as its condition holds. Inside any loop, `continue` skips the rest of the body, and `break` leaves the loop.

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

A `loop` runs forever, so a `break` is the only way out.

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

Since a `break` is the only way out of a `loop`, it can carry a value: `break i * 2` makes the whole `loop` evaluate to `i * 2`, and every `break` in it must agree on the type. A `while` loop can also end by its condition turning false, with no value to give, so it's always `()`.

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
