#[derive(Debug)]
pub struct Entity<const N: usize, T> {
    /// The coordinate to find this entity.
    pub coordinate: [usize; N],
    /// The value of this entity.
    pub value: T,
}
