impl DeltaTicket {
    pub fn with_i(self, i: IDim) -> Self {
        let mut new = DeltaTicket {
            i: i.into(),
            ..self
        };

        if operon::schema_base::Ticket::is_ready(&new) {
            new.status = operon::schema_base::TicketStatus::Queued;
        }

        new
    }

    pub fn with_j(self, j: JDim) -> Self {
        let mut new = DeltaTicket {
            j: j.into(),
            ..self
        };

        if operon::schema_base::Ticket::is_ready(&new) {
            new.status = operon::schema_base::TicketStatus::Queued;
        }

        new
    }

    pub fn with_k(self, k: KDim) -> Self {
        let mut new = DeltaTicket {
            k: k.into(),
            ..self
        };

        if operon::schema_base::Ticket::is_ready(&new) {
            new.status = operon::schema_base::TicketStatus::Queued;
        }

        new
    }
}
