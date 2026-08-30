# Style Guide

> [!NOTE]
> Inspired by [TigerStyle](https://tigerstyle.dev/)

## Assertions

Use `assert!` by default. Only use `debug_assert!` if you have benchmarked the code and identified that a specific assertion causes a measurable performance regression (for example, a costly check inside a hot loop).

This is because assertions are almost never a detectable performance hit, while a well-placed `assert!` can even *improve* performance, because the compiler may use it to eliminate downstream redundant checks (a common trick for helping autovectorization: asserting slice lengths before a loop removes per-iteration bounds checks).

More importantly, failing loudly in release builds is better than failing silently!

## Implementing

Implement according to the vertical slice architecture.

## Testing

### Snapshot Testing (Primary)

1. Create an input file `.crw` in `<stage>/snapshot_inputs/`.

2. Run the test (it will fail because no snapshot exists yet).

3. Review the snapshot using `cargo insta review` to make sure the output looks correct. If it does, accept it.

4. Commit the `.snap` file to git.

### Fuzz Testing (Secondary)

Fuzz test for isolated algorithmic code where integrated tests wouldn't catch edge cases.

## Dependencies

Zero dependencies.

## Naming Conventions 

- Qualifiers are appended as suffixes to names
- Don't abbreviate
- Use `snake_case` for everything other than `PascalCase` for type definitions, and `SCREAMING_SNAKE_CASE` for constants
- Use the same number of characters for related names (e.g. source/target) so they line up in the source.
- indexes and quantities
    - `count` to indicate number of items, and `index` for an index to an item
    - `size` to indicate the number of bytes, and `offet` to indicate the byte position

### Naming Handles

- the core type: `<type>`
- the handle: `<type>Id`
- the view: `<type>View`
- the handle pointing to a slice of handles: `<type>IdSpan`
- a variable, parameter, or struct field that's a handle: `<description>_id`
- a variable, parameter, or struct field whose type is a handle pointing to a slice of handles: `<description>_id_span`
- a variable, parameter, or struct field whose type is an *actual* slice of handles or a vector of handles: `<description>_ids`
- a variable, parameter, or struct field whose type is a view: `<description>_view`

## Layout

### Directory Layout

- Split into crates, keep the list of crates flats
- Within a crate, split stuff into modules as needed, keep structure mostly flat
- Have a folder for `common/` for miscellaneous files
- Extract anything self-contained into its own file

### File Layout

```rust
// mod statement 
// (mod-intra-ordering: suggested reading order)

// use std
// use third-party crates
// use project crates

// pub type definition 
// pub(crate) type definition       
// private type definition   
// (type-intra-ordering: from most important type definition, to least)   

// IF the module's interface is a set of functions:
// public function 
// functions

// impl pub type {
    // pub methods
    // pub(crate) methods
    // private methods
    // (method-intra-ordering: constructors, accessors, then modifiers)
// }   
// impl project trait for pub type        
// impl std trait for pub type 

// impl pub(crate) type {
    // pub methods
    // pub(crate) methods
    // private methods
    // (method-intra-ordering: constructors, accessors, then modifiers)
// }         
// impl project trait for pub(crate) 
// impl std trait for pub(crate) type

// impl private type {
    // pub methods
    // pub(crate) methods
    // private methods
    // (method-intra-ordering: constructors, accessors, then modifiers)
// }           
// impl project trait for private type
// impl std trait for private type

// (impl-intra-ordering: follows from the type definition intra ordering)

// #[cfg(test)]
// mod tests {

// }
```

## Commit Messages

https://scopedcommits.com/

## Doc Comments

```rust
/// <verbs>s ...
/// 
/// # Examples
/// 
/// ```rust
/// ```
```

- Common verbs:
    - **Query/access:** Returns, Gets, Contains, Indexes, Finds
    - **Insertion:** Adds, Appends, Inserts, Puts, Pushes
    - **Deletion:** Removes, Clears, Deletes, Trims, Drops
    - **Modification:** Replaces, Sets, Sorts, Reverses, Updates
    - **Construction:** Constructs, Creates
    - **Iteration/traversal:** Performs, Iterates, Traverses
    - **Copy/transform:** Copies, Converts, Wraps
