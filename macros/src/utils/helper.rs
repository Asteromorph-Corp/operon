use crate::configs::DimensionId;

pub fn dim_msg(dim: &DimensionId) -> String {
    format!("{dim} = {{{dim}}}")
}
