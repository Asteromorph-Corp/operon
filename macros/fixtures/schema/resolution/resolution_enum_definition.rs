/// An enum representing the resolution of any dimension.
#[derive(Debug, Clone)]
pub enum ResolutionEnum {
    I(operon::schema::Resolution<0usize>),
    J(operon::schema::Resolution<1usize>),
    K(operon::schema::Resolution<1usize>),
}
