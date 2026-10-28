# Docs

## Architecture

The core compiler is based on the [**query-based architecture**](https://ollef.github.io/blog/posts/query-based-compilers.html). Every stage is a set of queries, computed the first time something needs them and cached afterwards:

```mermaid
flowchart TD
    subgraph inputs [Inputs]
        SourceFileKey
        ModuleMap
    end

    subgraph syntax [Syntax]
        tokens_of
        cst_of
    end

    subgraph collect [Collecting]
        top_level_definitions_of
        block_definitions_of
        imports_of
        module_file_of
    end

    subgraph definition [Per definition: a function or a constant]
        signature_hir_and_source_map_of["signature::hir_and_source_map_of"]
        signature_hir_of["signature::hir_of"]
        signature_type_of["signature::type_of"]
        body_hir_and_source_map_of["body::hir_and_source_map_of"]
        body_hir_of["body::hir_of"]
        scopes_of
        body_types_of["body::types_of"]
        mir_of
    end

    SourceFileKey --> tokens_of --> cst_of

    cst_of --> top_level_definitions_of
    cst_of --> block_definitions_of
    cst_of --> imports_of

    ModuleMap --> module_file_of
    imports_of -- "supplies the path argument" --> module_file_of

    cst_of --> signature_hir_and_source_map_of --> signature_hir_of --> signature_type_of
    cst_of --> body_hir_and_source_map_of --> body_hir_of --> scopes_of

    body_hir_of & scopes_of & signature_type_of -.-> body_types_of
    top_level_definitions_of & block_definitions_of & module_file_of -.-> body_types_of

    body_hir_of & signature_type_of & body_types_of --> mir_of
```

1. [Lexical Analysis](architecture/lexical-analysis.md): source text to tokens (not written yet)
2. [Syntactic Analysis](architecture/syntactic-analysis.md): tokens to a concrete syntax tree
3. [Semantic Analysis](architecture/semantic-analysis.md): what each name refers to, and what type each expression has
4. [MIR Construction](architecture/mir-construction.md): the typed HIR to a control-flow graph in SSA form

## Testing

```bash
cargo test
```

Most tests are snapshot tests: see [Testing](testing.md) to add one.

## Style

Follow the [Style Guide](style-guide.md), including its commit messages.
