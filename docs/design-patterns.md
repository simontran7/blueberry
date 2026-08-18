# Design Patterns

- **Immutable pass outputs**: Each stage produces a new data structure. No stage mutates its input.
- **Error boundaries**: Diagnostics (rendered using the `ariadne` crate) are collected in a diagnostic sink. If a stage produced errors,  the compiler stops, emits diagnostics, instead of feeding bad data forward ([source](https://www.reddit.com/r/Compilers/comments/1706jts/comment/k3k5onq/)). However, if it has warnings, the compiler emits those warning diagnostics, but trucks along. 
- **Front-End Resilience**: Each stage continues after errors (error tokens, error AST nodes, error binding handles and error type handles) to report as many errors as possible in one pass.
- **Interning**: Strings are interned as `Symbol` handles; types are interned as `TypeId` handles so that equality checking is always $O(1)$.
- **Handle-based data structures, but Object-Oriented APIs**: the AST, HIR, MIR, type interner, string interner all use 32-bit handles rather than pointers or references. This gives compact representations, trivial equality checks, and access patterns become cache-friendly. To provide a object-oriented API handle-based data structure, we use the [Proxy](https://refactoring.guru/design-patterns/proxy) design pattern and create a `<Type>View` struct that holds two fields: a handle and the storage.
- **Compiler context** `CompilerContext` in `driver.rs` bundles shared state (`StringInterner`, `TypeInterner`) threaded through all compilation stages (similiar to Cranelift's `Context` and rustc's `TyCtxt`).
- **Iterative processing**: use stacks and loops over recursion
