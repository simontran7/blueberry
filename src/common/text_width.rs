use std::ops::AddAssign;

#[derive(Clone, Copy)]
pub(crate) struct TextWidth(u32);

impl TextWidth {
    pub(crate) fn new(width: usize) -> Self {
        TextWidth(width as u32)
    }
}

impl From<TextWidth> for usize {
    fn from(value: TextWidth) -> Self {
        value.0 as usize
    }
}

impl AddAssign for TextWidth {
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0
    }
}
