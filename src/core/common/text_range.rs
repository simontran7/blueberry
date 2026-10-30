use std::ops::{Add, AddAssign, Range, Sub};

/// A position in a file: the number of bytes before it.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub(crate) struct TextOffset(u32);

/// An amount of text, in bytes: a node's width, say, or the distance between two offsets.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub(crate) struct TextLength(u32);

/// A range of a file, from its start.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(crate) struct AbsoluteTextRange {
    start: TextOffset,
    end: TextOffset,
}

/// A range measured from the start of its definition (its anchor) rather than from the start of
/// the file, so that it stays the same when an edit above the definition moves it.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(crate) struct RelativeTextRange {
    /// the distance from the anchor to the range's start
    start: TextLength,
    /// the distance from the anchor to the range's end
    end: TextLength,
}

impl AbsoluteTextRange {
    pub(crate) fn new(start: TextOffset, end: TextOffset) -> Self {
        Self { start, end }
    }

    pub(crate) fn start(self) -> TextOffset {
        self.start
    }

    pub(crate) fn end(self) -> TextOffset {
        self.end
    }

    pub(crate) fn contains(self, other: AbsoluteTextRange) -> bool {
        self.start <= other.start && other.end <= self.end
    }

    pub(crate) fn relative_to(self, anchor: TextOffset) -> RelativeTextRange {
        RelativeTextRange {
            start: self.start - anchor,
            end: self.end - anchor,
        }
    }
}

impl RelativeTextRange {
    pub(crate) fn anchored_at(self, anchor: TextOffset) -> AbsoluteTextRange {
        AbsoluteTextRange::new(anchor + self.start, anchor + self.end)
    }
}

impl From<AbsoluteTextRange> for Range<usize> {
    fn from(range: AbsoluteTextRange) -> Self {
        usize::from(range.start)..usize::from(range.end)
    }
}

impl From<&AbsoluteTextRange> for Range<usize> {
    fn from(range: &AbsoluteTextRange) -> Self {
        Range::from(*range)
    }
}

impl TextOffset {
    pub(crate) fn new(offset: usize) -> Self {
        TextOffset(u32::try_from(offset).unwrap())
    }
}

impl TextLength {
    pub(crate) fn new(length: usize) -> Self {
        TextLength(u32::try_from(length).unwrap())
    }
}

impl From<TextOffset> for usize {
    fn from(offset: TextOffset) -> Self {
        offset.0 as usize
    }
}

impl From<TextLength> for usize {
    fn from(length: TextLength) -> Self {
        length.0 as usize
    }
}

/// moving forward by a length gives another offset
impl Add<TextLength> for TextOffset {
    type Output = TextOffset;

    fn add(self, rhs: TextLength) -> TextOffset {
        TextOffset(self.0 + rhs.0)
    }
}

impl AddAssign<TextLength> for TextOffset {
    fn add_assign(&mut self, rhs: TextLength) {
        self.0 += rhs.0;
    }
}

/// moving back by a length gives another offset
impl Sub<TextLength> for TextOffset {
    type Output = TextOffset;

    fn sub(self, rhs: TextLength) -> TextOffset {
        TextOffset(self.0 - rhs.0)
    }
}

/// the distance between two offsets is a length
impl Sub for TextOffset {
    type Output = TextLength;

    fn sub(self, rhs: TextOffset) -> TextLength {
        TextLength(self.0 - rhs.0)
    }
}

impl Add for TextLength {
    type Output = TextLength;

    fn add(self, rhs: TextLength) -> TextLength {
        TextLength(self.0 + rhs.0)
    }
}

impl AddAssign for TextLength {
    fn add_assign(&mut self, rhs: TextLength) {
        self.0 += rhs.0;
    }
}
