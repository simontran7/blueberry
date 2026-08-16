pub trait Handle: Copy + Eq {
    fn new(index: usize) -> Self;
    fn index(&self) -> usize;
}