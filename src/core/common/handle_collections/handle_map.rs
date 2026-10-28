#[expect(
    clippy::module_inception,
    reason = "the folder is named after its main type's file, like the rest of `common`"
)]
pub(in crate::core) mod handle_map;
pub(in crate::core) mod side_handle_map;

pub(in crate::core) use handle_map::HandleMap;
pub(in crate::core) use side_handle_map::SideHandleMap;
