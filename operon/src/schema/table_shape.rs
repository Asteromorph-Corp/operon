/// Whether a table matches the shape of the metadata it was built under.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TableShape {
    /// The recorded shape matches the metadata.
    Current,
    /// The recorded shape differs, so initialization rebuilds the table and discards its rows.
    Stale,
}

impl TableShape {
    /// Whether the table is due a rebuild.
    pub fn is_stale(self) -> bool {
        self == TableShape::Stale
    }

    /// `Stale` if either of the two is.
    pub fn merge(self, other: TableShape) -> TableShape {
        match (self, other) {
            (TableShape::Current, TableShape::Current) => TableShape::Current,
            _ => TableShape::Stale,
        }
    }
}
