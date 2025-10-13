#[automatically_derived]
impl operon::schema_base::Resolution for IResolution {
    type PrimaryKey = ();
    #[allow(clippy::unused_unit)]
    fn primary_key(&self) -> Self::PrimaryKey {
        ()
    }
    fn ub(&self) -> usize {
        self.0
    }
    #[allow(unused_variables)]
    fn new(ub: usize, primary_key: Self::PrimaryKey) -> Self {
        Self(ub)
    }
}
