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
    // Pull the "done" beta jobs from the metadata storage...
    let Some(coordinates) = tickets
        .iter()
        .map(|ticket| ticket.resolve().map(|job| job.coordinate))
        .collect::<Option<Vec<_>>>()
    else {
        operon::__private::tracing::info!(
            "Some `beta` tickets are corrupt in the metadata storage."
        );
        return Ok(false);
    };

    // ...and map them with the dimensions they spawned...
    let mut tags = Vec::new();
    for coordinate in coordinates {
        let Some(res) = client
            .resolution(self.spawn_dim_meta())
            .get(coordinate)
            .await?
        else {
            operon::__private::tracing::info!(
                "No `j` resolution found for `beta_{:?}` in the metadata storage.",
                coordinate
            );
            return Ok(false);
        };
        for j in 0..(res.ub) {
            tags.push((coordinate, j));
        }
    }

    // ...and check if the data storage holds all the data for them.
    let tags_to_check = match mode {
        operon::ui::CheckMode::MetadataOnly => return Ok(true),
        operon::ui::CheckMode::Exhaustive => tags,
        operon::ui::CheckMode::Quick => operon::utils::get_dop_tags(&tags),
        _ => unreachable!(),
    };

    for ([i], j) in tags_to_check {
        if storage.get_b([i, j]).await?.is_none() {
            operon::__private::tracing::info!("Data storage does not hold `B_{:?}`.", [i, j]);
            return Ok(false);
        }
    }

    Ok(true)
}
