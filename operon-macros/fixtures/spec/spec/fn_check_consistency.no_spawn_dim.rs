async fn check_consistency(
    &self,
    storage: &Sto,
    client: operon::meta_storage::MetaClient<'_>,
    mode: operon::ui::CheckMode,
) -> Result<bool, operon::scheduler::SchedulerError> {
    if mode == operon::ui::CheckMode::TrustAll {
        return Ok(true);
    }

    let tickets = client
        .ticket(self.job_meta())
        .get_all(operon::schema::TicketStatus::Done)
        .await?;
    // Pull the "done" epsilon jobs from the metadata storage...
    let Some(coordinates) = tickets
        .iter()
        .map(|ticket| ticket.resolve().map(|job| job.coordinate))
        .collect::<Option<Vec<_>>>()
    else {
        operon::log::info!("Some `epsilon` tickets are corrupt in the metadata storage.");
        return Ok(false);
    };

    // ...and check if the data storage holds all the data for them.
    let coordinates_to_check = match mode {
        operon::ui::CheckMode::MetadataOnly => return Ok(true),
        operon::ui::CheckMode::Exhaustive => {
            coordinates
        }
        operon::ui::CheckMode::Quick => {
            operon::utils::get_dop_coords(&coordinates)
        }
        _ => unreachable!(),
    };

    for coordinate in coordinates_to_check {
        if storage.get_e(coordinate).await?.is_none() {
            // TODO: improve error message coordinate display
            operon::log::info!("Data storage does not hold `E_{:?}`.", coordinate);
            return Ok(false);
        }
    }

    Ok(true)
}
