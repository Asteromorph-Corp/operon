impl EpsilonSpec {
    pub const fn job_meta(&self) -> operon::schema::JobMetadata<2usize> {
        metadata::job_epsilon_meta()
    }

    pub fn into_handler<Svc: CookingService, Sto: CookingStorage>(
        self,
    ) -> Box<dyn operon::scheduler::JobHandler<Svc, Sto>> {
        let job_meta = self.job_meta();
        Box::new(operon::scheduler::SpecWithMetadata::new(self, job_meta))
    }
}
