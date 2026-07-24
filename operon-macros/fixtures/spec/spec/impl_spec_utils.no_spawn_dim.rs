impl EpsilonSpec {
    pub const fn job_meta(&self) -> operon::__private::JobMetadata<2usize> {
        metadata::job_epsilon_meta()
    }

    pub fn into_handler<Svc: CookingService, Sto: CookingStorage, MSto: operon::__private::MetaBackend>(
        self,
    ) -> Box<dyn operon::__private::JobHandler<Svc, Sto, MSto>> {
        let job_meta = self.job_meta();
        Box::new(operon::__private::SpecWithMetadata::new(self, job_meta))
    }
}
