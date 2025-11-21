///An enum representing the resolution of any dimension.
#[derive(Debug, Clone)]
pub enum ResolutionEnum {
    I(operon::schema_base::Resolution<0usize>),
    J(operon::schema_base::Resolution<1usize>),
    K(operon::schema_base::Resolution<1usize>),
}
