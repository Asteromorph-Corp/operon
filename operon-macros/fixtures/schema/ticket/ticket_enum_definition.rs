/// An enum representing the ticket of any task.
#[derive(Debug, Clone)]
pub enum TicketEnum {
    Alpha(operon::__private::Ticket<0usize>),
    Beta(operon::__private::Ticket<1usize>),
    Gamma(operon::__private::Ticket<1usize>),
    Delta(operon::__private::Ticket<3usize>),
    Epsilon(operon::__private::Ticket<2usize>),
    Zeta(operon::__private::Ticket<1usize>),
}
