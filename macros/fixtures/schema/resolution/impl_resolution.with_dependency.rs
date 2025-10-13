#[automatically_derived]
impl operon::schema_base::Resolution for JResolution {
    type PrimaryKey = (IDim,);

    #[allow(clippy::unused_unit)]
    fn primary_key(&self) -> Self::PrimaryKey {
        (self.1,)
    }

    fn ub(&self) -> usize {
        self.0
    }

    #[allow(unused_variables)]
    fn new(ub: usize, primary_key: Self::PrimaryKey) -> Self {
        Self(ub, primary_key.0)
    }
}
