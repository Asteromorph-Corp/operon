impl BetaSpec {
    pub const fn task_meta(&self) -> operon::__private::TaskMetadata<1usize> {
        metadata::task_beta_meta()
    }

    pub const fn spawn_dim_meta(&self) -> operon::__private::DimensionMetadata<1usize> {
        metadata::dimension_j_meta()
    }

    pub fn into_handler<Svc: CookingService, Sto: CookingStorage, MSto: operon::__private::MetaBackend>(
        self,
    ) -> Box<dyn operon::__private::TaskHandler<Svc, Sto, MSto>> {
        let task_meta = self.task_meta();
        Box::new(operon::__private::SpecWithMetadata::new(self, task_meta))
    }
}
