impl BetaTicket {
    pub fn with_i(self, i: IDim) -> Self {
        let mut new = BetaTicket {
            i: i.into(),
            ..self
        };

        if operon::schema_base::Ticket::is_ready(&new) {
            new.status = operon::schema_base::TicketStatus::Queued;
        }

        new
    }
}
