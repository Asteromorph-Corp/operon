/// An enum representing any job.
#[derive(Debug, Clone)]
pub enum TicketEnum {
    Alpha(operon::schema::Ticket<0usize>),
    Beta(operon::schema::Ticket<1usize>),
    Gamma(operon::schema::Ticket<1usize>),
    Delta(operon::schema::Ticket<3usize>),
    Epsilon(operon::schema::Ticket<2usize>),
    Zeta(operon::schema::Ticket<1usize>),
}
