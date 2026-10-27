# Database

`Compiler` is the `core`'s **facade**.

This means that the batch compiler and the language server never calls a query directly, but asks `Compiler`, which calls the queries. 

It covers:
- **loading**: `load_modules` (builds the `ModuleMap`), `file` (finds a file's `SourceFileKey` in it), `files_reachable_from` (a file and everything it imports), `path`, `contents`
- **diagnostics**, per file and per stage: `lexical_diagnostics`, `syntax_diagnostics`, `semantic_diagnostics`
- **editor features**: `type_at`, `goto_definition`
- **IR dumps**: `dump_tokens`, `dump_cst`, `dump_hir`, `dump_mir`

_In the code:_ `compiler.rs`, `batch/driver.rs`

## The Database

The database (`BlueberryDatabase`) stores the inputs, and the cached result of every query computed from them. A query is computed the first time something asks for it, and is recomputed only when an input it read (directly, or through another query) has changed.

_In the code:_ `db.rs`

## Inputs

Everything the compiler computes comes from two kinds of inputs:

- a **`SourceFileKey`** per `.bb` file: its path and its contents. Every query about a file (e.g., `tokens_of`, `cst_of`) is keyed by it, and changing its contents (`set_contents`) is how `salsa` learns that the file was edited.
- the **`ModuleMap`**: one table, for the whole project, from each module path to its file's `SourceFileKey` (e.g., `shapes::circle` to the key of `shapes/circle.bb`). It's a **singleton** (there's only ever one, found with `ModuleMap::try_get`).

_In the code:_ `source_file_key.rs`, `module_map.rs`

## The Module Map

The `ModuleMap` exists for three reasons:

1. **imports**: an `import` names a module (`shapes::circle`), not a file. When a path goes through an import, the `Resolver` asks `module_file_of` which file that module is, and `module_file_of` looks it up in the map. An import naming no module in the map is reported by `import_diagnostics_of` (see [Import Lists](semantic-analysis.md#import-lists)).
2. **no reading inside queries**: since a query must never read a file (see [The Database](#the-database)), every file is read up front, before any query runs, and stored as an input.
3. **one key per file**: if `main.bb` and `util.bb` both import `math`, they must reach the _same_ `SourceFileKey`, so that `math.bb` is parsed and checked once, and its definitions have the same keys wherever they're used from. The map creates each file's key exactly once.

Since the map is itself an input, adding or removing a file (e.g., in an editor) would be a matter of setting a new table, after which `salsa` recomputes only the queries that looked a module up.

> [!NOTE]
> Every `.bb` file under the root is read when the map is built, but nothing more: a file is only tokenized, parsed, and checked when a query asks about it, e.g., when an import reaches it.

_In the code:_ `ModuleMap::load`, `ModulePath`, `module_file_of`

## Loading a Project

Loading a project is the first stage of a run ("Stage 0: Scanning" in the driver), before anything is lexed:

1. The **root** is the directory of the file being compiled (`--path`), or the current directory.
2. `ModuleMap::load` walks the root, and every directory under it. For each `.bb` file, it reads its contents into a new `SourceFileKey`, and records it under its module path: its path relative to the root, without the extension, one segment per directory (e.g., `shapes/circle.bb` becomes `shapes::circle`).
3. The table is stored in the database as the `ModuleMap`.

If a directory or a file can't be read, the run stops right there, with the error.

