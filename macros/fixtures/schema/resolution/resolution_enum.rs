///An enum representing the resolution of any dimension.
#[derive(Debug, Clone)]
pub enum ResolutionEnum {
    I(IResolution),
    J(JResolution),
    K(KResolution),
}
