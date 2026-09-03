use std::ops::{Add, AddAssign, Range, Sub};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
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

impl Sub for TextSize {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self {
        TextSize(self.0 - rhs.0)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(crate) struct TextRange {
    start: TextSize,
    end: TextSize,
}

impl TextRange {
    pub(crate) fn new(start: TextSize, end: TextSize) -> Self {
        Self { start, end }
    }

    pub(crate) fn start(&self) -> TextSize {
        self.start
    }

    pub(crate) fn end(&self) -> TextSize {
        self.end
    }
}

impl From<TextRange> for Range<usize> {
    fn from(range: TextRange) -> Self {
        usize::from(range.start)..usize::from(range.end)
    }
}

impl From<&TextRange> for Range<usize> {
    fn from(range: &TextRange) -> Self {
        Range::from(*range)
    }
}
