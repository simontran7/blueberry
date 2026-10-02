pub(crate) trait MergeValue: Clone {
    type Error;
    fn merge(a: &Self, b: &Self) -> Result<Self, Self::Error>;
}
