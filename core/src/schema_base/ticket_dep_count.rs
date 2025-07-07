#[derive(Debug, Clone, Default)]
pub struct TicketDepCount<T>(pub Option<T>);

impl<T> TicketDepCount<T> {
    pub fn new(dim: Option<T>) -> Self {
        Self(dim)
    }

    pub fn some(dim: T) -> Self {
        Self(Some(dim))
    }

    pub fn none() -> Self {
        Self(None)
    }

    pub fn is_some(&self) -> bool {
        self.0.is_some()
    }

    pub fn is_none(&self) -> bool {
        self.0.is_none()
    }

    pub fn unwrap(self) -> T {
        self.0.unwrap()
    }

    pub fn unwrap_or(self, default: T) -> T {
        self.0.unwrap_or(default)
    }
}

impl TicketDepCount<usize> {
    pub fn from_sql(num: i64) -> Result<Self, std::num::TryFromIntError> {
        match num {
            -1 => Ok(Self(None)),
            n => Ok(Self(Some(usize::try_from(n)?))),
        }
    }

    pub fn to_sql(&self) -> Result<i64, std::num::TryFromIntError> {
        match self.0 {
            Some(n) => i64::try_from(n),
            None => Ok(-1),
        }
    }
}

impl<T> From<T> for TicketDepCount<T> {
    fn from(dim: T) -> Self {
        Self(Some(dim))
    }
}
