async fn check_consistency(
    &self,
    storage: &Sto,
    client: MSto::Client<'_>,
    mode: operon::__private::CheckMode,
) -> Result<bool, operon::error::SchedulerError<Svc::Error, Sto::Error, MSto::Error>> {
    if mode == operon::__private::CheckMode::TrustAll {
        return Ok(true);
    }

    let tickets = client
        .ticket(self.task_meta())
        .get_all(operon::__private::TicketStatus::Done)
        .await?;
    // Pull the "done" epsilon jobs from the metadata storage...
    let Some(coordinates) = tickets
        .iter()
        .map(|ticket| ticket.resolve().map(|job| job.coordinate))
        .collect::<Option<Vec<_>>>()
    else {
        operon::__private::tracing::info!(
            "Some `epsilon` tickets are corrupt in the metadata storage."
        );
        return Ok(false);
    };

    // ...and check if the data storage holds all the data for them.
    let coordinates_to_check = match mode {
        operon::__private::CheckMode::MetadataOnly => return Ok(true),
        operon::__private::CheckMode::Exhaustive => coordinates,
        operon::__private::CheckMode::Quick => operon::__private::get_dop_coords(&coordinates),
        _ => unreachable!(),
    };

    for coordinate in coordinates_to_check {
        if storage.get_e(coordinate).await?.is_none() {
            // TODO: improve error message coordinate display
            operon::__private::tracing::info!("Data storage does not hold `E_{:?}`.", coordinate);
            return Ok(false);
        }
    }

    Ok(true)
}
