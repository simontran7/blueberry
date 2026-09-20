pub(crate) trait Handle: Copy + Eq {
    fn new(index: usize) -> Self;
    fn index(&self) -> usize;
}

macro_rules! handle_impl {
    ($vis:vis $name:ident) => {
        #[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
        $vis struct $name(core::num::NonZeroU32);

        impl $crate::core::common::handle_collections::Handle for $name {
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
