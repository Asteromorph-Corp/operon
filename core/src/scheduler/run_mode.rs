#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunMode {
    Clean,
    Rebuild,
    Restore,
}
