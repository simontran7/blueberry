# Modules

```
// shapes/circle.bb
const PI: I32 = 3;

func area(radius: I32) -> I32 {
    radius * radius * PI
}
```

```
// main.bb
import shapes::circle;

func main() {
    println(circle::area(3));
}
```

The filesystem _is_ the module tree! Every `.bb` file is a module, named after its path from the project root (the directory of the file being compiled): `shapes/circle.bb` is `shapes::circle`. A directory is a namespace just by containing files.

`import shapes::circle;` brings the module in under its last name, `circle`, and you reach its definitions through it: `circle::area`. You can only import a module, never a single definition, so `circle::area` reads the same wherever it's used.

## Visibility and Renaming

> [!CAUTION]
> Unimplemented

```
// shapes/circle.bb
pub const PI: I32 = 3;

pub func area(radius: I32) -> I32 {
    radius * radius * PI
}

func helper(x: I32) -> I32 { // only visible inside `shapes/circle.bb`
    x
}
```

```
// main.bb
import shapes::circle as c;

func main() {
    println(c::area(3));
}
```

Nothing in a file is visible to the files that import it unless it's marked `pub`. And `as` brings a module in under a name of your choice.
