use std::num::TryFromIntError;

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

pub trait OptionExt<T> {
    fn resolve(&self) -> Result<i64, TryFromIntError>;
}

impl OptionExt<i64> for Option<usize> {
    fn resolve(&self) -> Result<i64, TryFromIntError> {
        match self {
            Some(i) => i64::try_from(*i),
            None => Ok(-1),
        }
    }
}
