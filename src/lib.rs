// Pedantic lints that fight the codebase's deliberate style:
// - `elidable_lifetime_names`: the salsa database lifetime is always spelled `'db` (and a view's
//   borrow `'a`), even where it could be elided, so that every signature reads the same way
// - `too_many_lines`: the parser, lowerers, dumpers, and diagnostic descriptions are each one
//   `match` over every kind of node or diagnostic, which reads best in one place
// - `unused_self`: `Cfg`'s `allocate_*` constructors and the dumpers' helpers are methods so that
//   callers reach them through the value they work with, whether or not they use it yet
// - `struct_field_names`: fields are named by role with a `_id` (or `_handle`) suffix, per
//   `docs/style-guide.md`
// - `needless_pass_by_value`: AST nodes are cheap handles that the lowerers take by value
//   throughout, and the parser's markers are moved into the call that uses them up
#![allow(
    clippy::elidable_lifetime_names,
    clippy::too_many_lines,
    clippy::unused_self,
    clippy::struct_field_names,
    clippy::needless_pass_by_value
)]

pub mod batch;
pub mod cli;
pub(crate) mod core;

pub(crate) use salsa::Database as Db;
