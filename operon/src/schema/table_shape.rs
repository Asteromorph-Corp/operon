/// Whether a table matches the shape of the metadata it was built under.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TableShape {
    pub is_stale: bool,
}

impl TableShape {
    pub const CURRENT: Self = Self { is_stale: false };
    pub const STALE: Self = Self { is_stale: true };
}
