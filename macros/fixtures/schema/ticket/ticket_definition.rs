#[derive(Debug, Clone, Default)]
pub struct BetaTicket {
    pub i: operon::schema_base::TicketDepCount<IDim>,
    deps_count: usize,
    deps_quota: usize,
    deps_done: bool,
    pub status: operon::schema_base::TicketStatus,
}
