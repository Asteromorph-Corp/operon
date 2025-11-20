#[operon::async_trait::async_trait]
#[automatically_derived]
impl operon::schema_base::Ticket for BetaTicket {
    type Job = operon::schema_base::Job<1usize>;

    #[allow(clippy::needless_update)]
    fn new() -> Self {
        Self {
            deps_count: 0,
            deps_quota: 1usize,
            deps_done: false,
            status: operon::schema_base::TicketStatus::Waiting,
            ..Default::default()
        }
    }

    fn update_deps_done(mut self) -> Self {
        self.deps_done = self.deps_count >= self.deps_quota;
        if self.is_ready() {
            self.status = operon::schema_base::TicketStatus::Queued;
        }
        self
    }

    fn raise_dependency_count(mut self) -> Self {
        self.deps_count += 1;
        self.update_deps_done()
    }

    fn raise_dependency_quota(mut self, explosion_ub: usize) -> Self {
        self.deps_quota = self.deps_quota + explosion_ub - 1;
        self.update_deps_done()
    }

    fn is_ready(&self) -> bool {
        self.is_resolved() && self.deps_done
    }

    fn is_resolved(&self) -> bool {
        self.i.is_some()
    }

    fn resolve(&self) -> Option<Self::Job> {
        if !self.is_ready() {
            return None;
        }
        let job = operon::schema_base::Job {
            coordinate: [self.i.0?],
        };
        Some(job)
    }
}
