#[derive(Debug)]
pub struct BetaPeerTxs {
    pub to_delta: operon::scheduler::PeerEventSender<schema::JobEnum, schema::ResolutionEnum>,
    pub to_epsilon: operon::scheduler::PeerEventSender<schema::JobEnum, schema::ResolutionEnum>,
}
