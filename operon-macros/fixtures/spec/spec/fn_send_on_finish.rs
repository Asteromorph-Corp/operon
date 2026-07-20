async fn send_on_finish(
    &self,
    peer_txs: &Self::PeerEventSenders,
    job: Self::Job,
    resolution: Self::Resolution,
) -> Result<(), operon::error::SchedulerError<MSto::Error, Sto::Error>> {
    match peer_txs
        .to_delta
        .send(operon::__private::PeerEvent::Resolution(
            schema::ResolutionEnum::J(resolution),
        ))
        .await
    {
        Ok(_) => {
            operon::__private::tracing::trace!("`beta` sent peer event to `delta`: {resolution:?}")
        }
        // Verbosity should be low here, since this can happen
        // an arbitrary number of times
        // if a descendant scheduler errored out.
        Err(_) => operon::__private::tracing::trace!(
            "`delta`'s peer channel closed before handling `beta`'s {resolution:?}"
        ),
    }
    // out-dependencies (delta, epsilon)
    match peer_txs
        .to_delta
        .send(operon::__private::PeerEvent::Job(schema::JobEnum::Beta(
            job,
        )))
        .await
    {
        Ok(_) => operon::__private::tracing::trace!("`beta` sent peer event to `delta`: {job:?}"),
        Err(_) => {
            operon::__private::tracing::trace!(
                "`delta`'s peer channel closed before handling `beta`'s {job:?}"
            )
        }
    }
    match peer_txs
        .to_epsilon
        .send(operon::__private::PeerEvent::Job(schema::JobEnum::Beta(
            job,
        )))
        .await
    {
        Ok(_) => operon::__private::tracing::trace!("`beta` sent peer event to `epsilon`: {job:?}"),
        Err(_) => {
            operon::__private::tracing::trace!(
                "`epsilon`'s peer channel closed before handling `beta`'s {job:?}"
            )
        }
    }
    Ok(())
}
