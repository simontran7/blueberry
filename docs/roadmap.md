# Roadmap

## Language Features

- Product Types (`record`, `tuple`, anonymous tuple)
- Sum Types
- Pattern Matching
- References
- Arrays and Slices
- Strings
- Methods
- Generics
  - Give call arguments better expectations from the call's expected type (rustc's "expected inputs for expected output"), once signatures can contain inference variables
- Typeclasses with default implementations
  - Only allow `==`/`!=` on types implementing `Eq`, and `<`/`>`/`<=`/`>=` on types implementing `Ord` (today any two values of the same type compare, e.g. `f == g` on functions or `() < ()`)
- `Option` and `Result`
- Panic (abort-only)
  - Check division by zero (today `1 / 0` is emitted as a plain division)
  - Check arithmetic overflow, or decide that it wraps
- Anonymous Function
- Closures
- Iterators
- Visibility
- Importing definitions directly (e.g., `import shapes::circle::{area};`), on top of today's module imports
- Constant evaluation (today a constant is read with a `Constant` MIR instruction, but nothing computes its value)
- Type aliases
- Standard Library
- Tail Call Optimisation
- `defer`
- Pipe Operator
- Named Function Arguments
- Green Threads

## Tooling

- x86-64 backend
- WebAssembly backend
- Language server
- Built-in test runner
- Linter
- Formatter
- Integrated Build System
- Package manager
- Debug info (DWARF)
