impl operon::schema_base::ResolutionEnum for ResolutionEnum {
    fn primary(resolution: usize) -> Self {
        Self::I(IResolution(resolution))
    }
}
