# Blocks

A block is an expression too: it runs its statements, and its value is its last expression, written without a `;`. Here, `area` is `12`. A block without a last expression produces nothing (`()`).

```
let area = {
    let width = 3;
    let height = 4;
    width * height
};
```

Variables defined inside a block can't be used outside of it: `width` and `height` only exist inside the braces.
