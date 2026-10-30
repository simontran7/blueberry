# Development Environment

## Creating a Project

A project is just a directory of `.bb` files, with no project file to write:

```
hello/
├── main.bb
└── shapes/
    └── circle.bb
```

`main.bb` holds `main`, where the program starts. Its directory is the project's root: imports are found from there, so `import shapes::circle;` brings in `hello/shapes/circle.bb` (see [Modules](modules.md)).

```
// hello/shapes/circle.bb
func area(radius: I32) -> I32 {
    radius * radius * 3
}
```

```
// hello/main.bb
import shapes::circle;

func main() {
    let a: I32 = circle::area(2);
}
```

## Checking a Program

`check` runs the compiler on `main.bb`, and on every file it imports, and reports any error it finds.

```bash
blueberry check --path=hello/main.bb
```

For example, with `let a: Bool = circle::area(2);` in `main.bb`, it reports:

```
[E0308] Error: mismatched types
   ╭─[main:4:19]
   │
 4 │     let a: Bool = circle::area(2);
   │                   ───────┬───────
   │                          ╰───────── expected `Bool`, found `I32`
───╯
```

When there are none, it prints `Compiled main`.

Options are written as `--name=value`, and `--path` is always required. Run `blueberry --help` to see every command and option.

## Looking Inside the Compiler

`--emit` also prints what the compiler made of your program at one of its stages: `tokens`, `cst` (the syntax tree), `hir` (the program, with the type of every expression), or `mir`. It can take several, separated by commas, like `--emit=hir,mir`.

```bash
blueberry check --path=hello/main.bb --emit=hir
```

## Building and Running

> [!CAUTION]
> Unimplemented

`build` compiles a program into an executable, and `run` builds it, then runs it.

```bash
blueberry build --path=hello/main.bb
blueberry run --path=hello/main.bb
```

## Editor Support

> [!CAUTION]
> Unimplemented

A language server will bring blueberry's errors, the type of an expression on hover, and go-to-definition into your editor.
