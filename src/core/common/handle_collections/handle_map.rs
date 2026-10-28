#[expect(
    clippy::module_inception,
    reason = "the folder is named after its main type's file, like the rest of `common`"
)]
pub(crate) mod handle_map;
pub(crate) mod side_handle_map;

pub(crate) use handle_map::HandleMap;
pub(crate) use side_handle_map::SideHandleMap;
