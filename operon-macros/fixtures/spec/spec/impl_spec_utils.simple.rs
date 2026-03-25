impl BetaSpec {
    pub const fn job_meta(&self) -> operon::schema::JobMetadata<1usize> {
        metadata::job_beta_meta()
    }

    pub const fn spawn_dim_meta(&self) -> operon::schema::DimensionMetadata<1usize> {
        metadata::dimension_j_meta()
    }

    pub fn into_handler<Svc: CookingService, Sto: CookingStorage>(
        self,
    ) -> Box<dyn operon::scheduler::JobHandler<Svc, Sto>> {
        let job_meta = self.job_meta();
        Box::new(operon::scheduler::SpecWithMetadata::new(self, job_meta))
    }
}
