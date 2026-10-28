# scratchpad

- https://github.com/ehsanmok/create-your-own-lang-with-rust
- https://github.com/isuckatcs/how-to-compile-your-language

### Before starting

- [ ] Read [docs/architecture/database.md](docs/architecture/database.md): the database, its inputs, and the module map

### 0. The spine

- [ ] ★ [semantic_analysis.rs](src/core/semantic_analysis.rs), top-down
  - [ ] `body::types_of`
  - [ ] `body::hir_of`, `body::source_map_of`
  - [ ] `scopes_of`
  - [ ] `signature::type_of`, `signature::hir_of`, `signature::source_map_of`
  - [ ] `top_level_definitions_of`, `block_definitions_of`, `imports_of`, `module_file_of`
  - [ ] `definition_diagnostics_of`, `import_diagnostics_of`
  - [ ] `definition_at`, `all_definitions_of`
  - [ ] the private helpers at the bottom

### 1. AST lowering

- [ ] ★ [keys.rs](src/core/semantic_analysis/keys.rs): `FunctionKey`, `ConstantKey`, `BlockKey`
- [ ] ★ [ast_lowering/nodes.rs](src/core/semantic_analysis/ast_lowering/nodes.rs): `Body`, `Signature`, `Expression`, `Statement`, `HirNodeStore`, `HirNodeSourceMap`
- [ ] ◐ [ast_lowering/hir_node_store_builder.rs](src/core/semantic_analysis/ast_lowering/hir_node_store_builder.rs)
- [ ] ◐ [ast_lowering.rs](src/core/semantic_analysis/ast_lowering.rs): signatures, type expressions, names, paths
- [ ] ◐ [ast_lowering/body_lowerer.rs](src/core/semantic_analysis/ast_lowering/body_lowerer.rs): `lower_expression`, `lower_block`
- [ ] ○ [ast_lowering/dumper.rs](src/core/semantic_analysis/ast_lowering/dumper.rs)

### 2. Name collecting and resolving

- [ ] ★ [name_resolution/definition_list.rs](src/core/semantic_analysis/name_resolution/definition_list.rs): `DefinitionList`, `Definition`
- [ ] ◐ [name_resolution/import_list.rs](src/core/semantic_analysis/name_resolution/import_list.rs)
- [ ] ★ [name_resolution/scope_tree.rs](src/core/semantic_analysis/name_resolution/scope_tree.rs): `containing_scope`, `visit_expression`
- [ ] ★ [name_resolution/resolver.rs](src/core/semantic_analysis/name_resolution/resolver.rs): the lookup order, `Resolution`, `ResolveError`
- [ ] ○ [module_map.rs](src/core/module_map.rs) and [compiler.rs](src/core/compiler.rs), when imports come up

### 3. Type checking

Doc: [Type Checking a Body](docs/architecture/semantic-analysis.md#type-checking-a-body)

- [x] ★ [type_checking/types.rs](src/core/semantic_analysis/type_checking/types.rs): `Ty`, `TyKind`, `InferenceVariable`, `is_poisoned`
- [x] ★ [type_checking/inference_table.rs](src/core/semantic_analysis/type_checking/inference_table.rs): resolving, `unify`, `coerce`, `occurs`, the fallbacks
- [] ◐ [type_checking/type_expression_lowering.rs](src/core/semantic_analysis/type_checking/type_expression_lowering.rs)
- [ ] ★ [type_checking/type_checker.rs](src/core/semantic_analysis/type_checking/type_checker.rs)
  - [x] the types at the top: `BodySemantics`, `TypeChecker`, `Expectation`, `CoercionFunnel`, `LoopFrame`, `Diverges`, `BinaryOperatorRule`
  - [ ] `check_function`, `check_constant`, `check_expression`
  - [ ] `check_integer_literal`, `check_path`
  - [ ] `check_let`, `check_block`, `check_if`
  - [ ] `check_loop`, `check_break`, `check_continue`, `check_return`, `check_returned`
  - [ ] `check_tuple`, `check_field`, `check_call`
  - [ ] `check_unary_operation`, `check_binary_operation`, `constrain_to_integer`
  - [ ] `check_assignment`, `is_place`
  - [ ] `coerce_or_report`, `pour_or_report`
  - [ ] `finish`, `check_pending_negations`, `check_literal_ranges`

### 4. Diagnostics

- [ ] ◐ [semantic_diagnostic.rs](src/core/semantic_analysis/semantic_diagnostic.rs): the two enums, and `resolve`
