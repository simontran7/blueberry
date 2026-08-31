use std::ops::{Add, AddAssign};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct TextSize(u32);

impl TextSize {
    pub(crate) fn new(size: usize) -> Self {
        TextSize(size as u32)
    }
}

impl From<TextSize> for usize {
    fn from(value: TextSize) -> Self {
        value.0 as usize
    }
}

impl AddAssign for TextSize {
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0
    }
}

impl Add for TextSize {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        TextSize(self.0 + rhs.0)
    }
}
