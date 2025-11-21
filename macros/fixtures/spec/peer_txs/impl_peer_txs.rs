#[operon::async_trait::async_trait]
#[automatically_derived]
impl operon::scheduler::PeerEventSenders<schema::JobEnum, schema::ResolutionEnum> for BetaPeerTxs {
    fn gather_from(
        mut senders: operon::scheduler::PeerEventSenderMap<schema::JobEnum, schema::ResolutionEnum>,
    ) -> Self {
        BetaPeerTxs {
            to_delta: senders
                .remove(metadata::DELTA_ID)
                .unwrap_or_else(|| panic!("No sender for job `{}` found", metadata::DELTA_ID)),
            to_epsilon: senders
                .remove(metadata::EPSILON_ID)
                .unwrap_or_else(|| panic!("No sender for job `{}` found", metadata::EPSILON_ID)),
        }
    }

    fn downgrade_all(&mut self) {
        self.to_delta.downgrade();
        self.to_epsilon.downgrade();
    }
}
