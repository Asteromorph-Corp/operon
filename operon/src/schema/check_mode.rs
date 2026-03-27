#[derive(clap::ValueEnum, Debug, Clone, PartialEq, Eq, Copy)]
#[clap(rename_all = "kebab-case")]
pub enum CheckMode {
    TrustAll,
    MetadataOnly,
    Quick,
    Exhaustive,
}
