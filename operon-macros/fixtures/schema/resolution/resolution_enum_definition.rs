/// An enum representing the resolution of any dimension.
#[derive(Debug, Clone)]
pub enum ResolutionEnum {
    I(operon::__private::Resolution<0usize>),
    J(operon::__private::Resolution<1usize>),
    K(operon::__private::Resolution<1usize>),
}
