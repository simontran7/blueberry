pub(crate) mod handle;
pub(crate) mod handle_map;
pub(crate) mod handle_range;
pub(crate) mod side_handle_map;

pub(crate) use handle::Handle;
pub(crate) use handle_map::{HandleMap, IntoIter, Iter, IterMut};
pub(crate) use handle_range::HandleRange;
pub(crate) use side_handle_map::SideHandleMap;

macro_rules! handle_impl {
    ($vis:vis $name:ident) => {
        #[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
        $vis struct $name(core::num::NonZeroU32);

        impl $crate::core::common::handlemap::Handle for $name {
            fn new(i: usize) -> Self {
                Self(core::num::NonZeroU32::new(u32::try_from(i).expect("index too large") + 1).expect("index too large"))
            }

            fn index(&self) -> usize {
                (self.0.get() - 1) as usize
            }
        }
    };
}
pub(crate) use handle_impl;
