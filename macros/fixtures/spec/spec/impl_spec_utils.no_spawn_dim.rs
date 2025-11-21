impl EpsilonSpec {
    pub const fn job_meta(&self) -> operon::schema::JobMetadata<2usize> {
        metadata::job_epsilon_meta()
    }

    pub fn all_upstream_jobs(&self) -> std::collections::HashSet<&'static str> {
        std::collections::HashSet::from_iter(["alpha", "beta", "delta", "epsilon", "gamma"])
    }

    pub fn into_handler<Svc: CookingService, Sto: CookingStorage>(
        self,
    ) -> Box<dyn operon::scheduler::JobHandler<Svc, Sto>> {
        let job_meta = self.job_meta();
        let all_upstream_jobs = self.all_upstream_jobs();
        Box::new(operon::scheduler::SpecWithMetadata::new(
            self,
            job_meta,
            all_upstream_jobs,
        ))
    }
}
