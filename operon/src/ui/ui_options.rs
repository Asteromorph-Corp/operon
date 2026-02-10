#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiOptions {
    #[default]
    Interactive,
    Headless,
}
