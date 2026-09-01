# ARCHITECTURE

The dominant compiler architecture of the past is the **pipeline-based architecture**. It involves processing the source file by stages, akin to a pipeline: 

```
source → tokenize → parse → name resolution → typecheck → lower → ...
```

Each stage runs once, top to bottom, over the whole program. 

This architecture worked fine for **batch compilation**: the primary purpose of a compiler back in the day. Suddenly, with the rise of IDEs, **interactive use** became an increasingly common use case for a compiler. Pipeline-based architectures were unfortunately not well suited for interactive use since if the user changes even a *single* character, the whole pipeline needs to rerun from scratch. 

This is where a [**query-based architecture** ](https://ollef.github.io/blog/posts/query-based-compilers.html) comes in. The core idea is memoized, pure functions that call each other, where the query engine will track which function called which, with what inputs, so it knows exactly what to recompute when something changes. It works well for batch compiling, and for interactive use. On top of this, it naturally allows compiler to be [**incremental**](https://en.wikipedia.org/wiki/Incremental_compiler).

A query system involves two kinds of queries:
-   **Input queries**: Values that aren't computed. They're set directly, and the system tracks when they change. E.g., `file_text(file_id) -> String`.
-   **Derived queries**: Regular functions, but memoized. Their results are computed from other queries (input or derived) and automatically recomputed only when those dependencies change. E.g., `parsed_ast(file_id) -> Ast`, which internally calls `file_text(file_id)`.

