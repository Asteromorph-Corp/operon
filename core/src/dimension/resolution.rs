pub trait Resolution: Clone + Send + Sync + 'static {
    fn primary(dim: usize) -> Self;
}
