#[expect(
    clippy::module_inception,
    reason = "the folder is named after its main type's file, like the rest of `common`"
)]
pub(crate) mod handle_disjoint_set;
pub(crate) mod merge_value;

pub(crate) use handle_disjoint_set::HandleDisjointSet;
pub(crate) use merge_value::MergeValue;
