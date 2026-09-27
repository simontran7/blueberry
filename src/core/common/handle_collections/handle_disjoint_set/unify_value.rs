pub(crate) trait UnifyValue: Clone {
    type Error;
    fn unify(a: &Self, b: &Self) -> Result<Self, Self::Error>;
}
