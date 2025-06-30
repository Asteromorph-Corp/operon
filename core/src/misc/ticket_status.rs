#[derive(Debug, Clone, Default, Copy, PartialEq, Eq, ::postgres_types::FromSql)]
#[postgres(name = "ticket_status")]
pub enum TicketStatus {
    #[postgres(name = "waiting")]
    #[default]
    Waiting,
    #[postgres(name = "queued")]
    Queued,
    #[postgres(name = "done")]
    Done,
}
impl ::std::fmt::Display for TicketStatus {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        match self {
            TicketStatus::Waiting => write!(f, "waiting"),
            TicketStatus::Queued => write!(f, "queued"),
            TicketStatus::Done => write!(f, "done"),
        }
    }
}
impl From<&str> for TicketStatus {
    fn from(s: &str) -> Self {
        match s {
            "waiting" => TicketStatus::Waiting,
            "queued" => TicketStatus::Queued,
            "done" => TicketStatus::Done,
            _ => panic!("Invalid ticket status: {s}"),
        }
    }
}
