pub trait SplitFirstOwned<T> {
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
