use std::num::TryFromIntError;

use postgres_types::ToSql;

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

#[repr(transparent)]
pub struct SqlParams(Vec<Box<dyn ToSql + Send + Sync + 'static>>);

impl SqlParams {
    pub fn from_usize(items: impl IntoIterator<Item = usize>) -> Result<Self, TryFromIntError> {
        let items = items
            .into_iter()
            .map(|item| {
                i64::try_from(item).map(|x| Box::new(x) as Box<dyn ToSql + Send + Sync + 'static>)
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self(items))
    }

    pub fn borrow(&self) -> Vec<&(dyn ToSql + Sync + 'static)> {
        self.0
            .iter()
            .map(|x| x.as_ref() as &(dyn ToSql + Sync + 'static))
            .collect()
    }
}
