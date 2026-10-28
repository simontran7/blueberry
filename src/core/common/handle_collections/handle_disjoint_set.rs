#[expect(
    clippy::module_inception,
    reason = "the folder is named after its main type's file, like the rest of `common`"
)]
pub(in crate::core) mod handle_disjoint_set;
pub(in crate::core) mod merge_value;

pub(in crate::core) use handle_disjoint_set::HandleDisjointSet;
pub(in crate::core) use merge_value::MergeValue;
