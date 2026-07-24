impl BetaSpec {
    pub const fn job_meta(&self) -> operon::__private::JobMetadata<1usize> {
        metadata::job_beta_meta()
    }

    pub const fn spawn_dim_meta(&self) -> operon::__private::DimensionMetadata<1usize> {
        metadata::dimension_j_meta()
    }

    pub fn into_handler<Svc: CookingService, Sto: CookingStorage, MSto: operon::__private::MetaBackend>(
        self,
    ) -> Box<dyn operon::__private::JobHandler<Svc, Sto, MSto>> {
        let job_meta = self.job_meta();
        Box::new(operon::__private::SpecWithMetadata::new(self, job_meta))
    }
}
