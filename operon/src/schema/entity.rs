pub struct Entity<const N: usize, T> {
    pub coordinate: [usize; N],
    pub value: T,
}
