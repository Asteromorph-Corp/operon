use std::num::TryFromIntError;

pub trait OptionExt<T> {
    fn to_sql(&self) -> Result<i64, TryFromIntError>;
}

impl OptionExt<i64> for Option<usize> {
    fn to_sql(&self) -> Result<i64, TryFromIntError> {
        match self {
            Some(i) => i64::try_from(*i),
            None => Ok(-1),
        }
    }
}
