# Semantic Analysis

Semantic analysis covers four concerns:
- **Collecting**: determine what names exist and where (which definitions a file declares, which locals are visible in a scope, which modules a file imports).
- **Lowering**: convert syntax tree into the HIR.
- **Resolving**: given one specific _occurrence_ of a name, decide which declaration it points to.
- **Type checking**: inferring and checking the types of expressions.

## Stable identities

A definition or block needs an identity that survives edits elsewhere in the file so that unrelated changes don't invalidate its cached signature, body, or type. 

`RedNodeId`/`RawRedNodeId` (in `red_node_directory.rs`) implement this as a tree-path scheme: `(file, parent, kind, name, collision_index)`. Two same-named siblings (e.g. two anonymous blocks) get distinct ids via the collision index, and inserting an unrelated definition elsewhere in the file leaves every other id untouched. `RedNodeDirectory` is the per-file table mapping an id to its current tag (kind + span), rebuilt on every parse. `to_ast_node` walks from the file's root back down to the concrete node an id currently refers to.

## Lowering

## Name Resolution

Every identifier in the source needs an answer to: "what does this name refer to?". Depending on where the name is located, it is stored in different data structures: 

| Location | Stored in |
|---|---|
| Local variable or a function's parameter | `ExpressionScopes`, one per definition body |
| Definition in a block | `DefinitionTree`, one per block that _has_ nested definitions (via `block_scoped_definitions_of`) |
| Definition at top-level | `DefinitionTree`, one per file (via `file_scoped_definitions_of`) |
| Imported name from another file | `Vec<Path>` (via `imports_of`), which is resolved to a file via `ProjectModules` (via `module_file_of`) |

The core query is `resolve(db, file, scope, name)`, which returns `Option<Resolution<'db>>`: `None` if `name` doesn't resolve anywhere, or a `Local(LocalBindingHandle)`, `Function(FunctionKey<'db>)`, or `Constant(ConstantKey<'db>)`.

### Collecting

### Resolving

## Type checking


