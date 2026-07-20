#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct OptionCoordinate(pub Option<usize>);

impl OptionCoordinate {
    pub const fn new(dim: Option<usize>) -> Self {
        Self(dim)
    }

    pub const fn some(dim: usize) -> Self {
        Self(Some(dim))
    }

    pub const fn none() -> Self {
        Self(None)
    }

    pub const fn is_some(&self) -> bool {
        self.0.is_some()
    }

    pub const fn is_none(&self) -> bool {
        self.0.is_none()
    }

    pub fn unwrap(self) -> usize {
        self.0.unwrap()
    }

    pub fn unwrap_or(self, default: usize) -> usize {
        self.0.unwrap_or(default)
    }

    pub fn as_sql_param(&self) -> Result<i64, std::num::TryFromIntError> {
        match self.0 {
            Some(n) => i64::try_from(n),
            None => Ok(-1),
        }
    }

    pub fn from_sql_value(value: i64) -> Result<Self, std::num::TryFromIntError> {
        match value {
            -1 => Ok(Self(None)),
            n => Ok(Self(Some(usize::try_from(n)?))),
        }
    }
}

impl From<usize> for OptionCoordinate {
    fn from(dim: usize) -> Self {
        Self(Some(dim))
    }
}
