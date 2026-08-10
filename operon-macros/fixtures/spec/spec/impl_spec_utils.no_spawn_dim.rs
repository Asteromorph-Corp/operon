impl EpsilonSpec {
    pub const fn task_meta(&self) -> operon::__private::TaskMetadata<2usize> {
        metadata::task_epsilon_meta()
    }

    pub fn into_handler<Svc: CookingService, Sto: CookingStorage, MSto: operon::__private::MetaBackend>(
        self,
    ) -> Box<dyn operon::__private::TaskHandler<Svc, Sto, MSto>> {
        let task_meta = self.task_meta();
        Box::new(operon::__private::SpecWithMetadata::new(self, task_meta))
    }
}
