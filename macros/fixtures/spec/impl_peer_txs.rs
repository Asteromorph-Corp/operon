#[operon::async_trait::async_trait]
#[automatically_derived]
impl operon::scheduler::PeerEventSenders<schema::JobEnum, schema::ResolutionEnum> for BetaPeerTxs {
    fn gather_from(
        mut senders: operon::scheduler::PeerEventSenderMap<schema::JobEnum, schema::ResolutionEnum>,
    ) -> Self {
        BetaPeerTxs {
            to_delta: senders
                .remove(<schema::DeltaJob as operon::schema_base::Job>::id())
                .unwrap_or_else(|| {
                    panic!(
                        "No sender for job `{}` found",
                        <schema::DeltaJob as operon::schema_base::Job>::id()
                    )
                }),
            to_epsilon: senders
                .remove(<schema::EpsilonJob as operon::schema_base::Job>::id())
                .unwrap_or_else(|| {
                    panic!(
                        "No sender for job `{}` found",
                        <schema::EpsilonJob as operon::schema_base::Job>::id()
                    )
                }),
        }
    }

    fn downgrade_all(&mut self) {
        self.to_delta.downgrade();
        self.to_epsilon.downgrade();
    }
}
