#[derive(Debug)]
pub struct BetaPeerTxs {
    pub to_delta: operon::__private::PeerEventSender<
        schema::JobEnum,
        schema::ResolutionEnum,
        schema::TicketEnum,
    >,
    pub to_epsilon: operon::__private::PeerEventSender<
        schema::JobEnum,
        schema::ResolutionEnum,
        schema::TicketEnum,
    >,
}
