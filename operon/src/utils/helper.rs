/// Splitting an iterator into its first item and the rest, by value.
pub trait SplitFirstOwned<T> {
    /// The first item and everything after it, or `None` when there is no first item.
    fn split_first_owned(self) -> Option<(T, Vec<T>)>;
}

impl<T, I: IntoIterator<Item = T>> SplitFirstOwned<T> for I {
    fn split_first_owned(self) -> Option<(T, Vec<T>)> {
        let mut iter = self.into_iter();

        let first = iter.next()?;
        let rest: Vec<T> = iter.collect();
        Some((first, rest))
    }
}
