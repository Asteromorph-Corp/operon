use std::collections::HashMap;

use futures::{StreamExt, TryStreamExt};

use crate::{
    meta_storage::{MetaClient, init_footprint, init_schema},
    scheduler::{
        IndividualSpec, JobRebuilder, PeerEvent, PeerEventSender, PrimarySpec, SchedulerError,
        SpecWithRx, SpecsWithChannels,
    },
    service::OperonService,
    storage::OperonStorage,
    ui::UiState,
};

pub struct SchedulerSpec<Svc, Sto> {
    pub primary_spec: Box<dyn PrimarySpec<Svc, Sto>>,
    pub individual_specs: Vec<Box<dyn IndividualSpec<Svc, Sto>>>,
}

impl<Svc, Sto> SchedulerSpec<Svc, Sto>
where
    Svc: OperonService,
    Sto: OperonStorage,
{
    pub fn new(
        primary_schedule: Box<dyn PrimarySpec<Svc, Sto>>,
        schedules: Vec<Box<dyn IndividualSpec<Svc, Sto>>>,
    ) -> Self {
        Self {
            primary_spec: primary_schedule,
            individual_specs: schedules,
        }
    }

    pub(crate) fn prepare_channels(&self, channel_size: usize) -> SpecsWithChannels<'_, Svc, Sto> {
        let len = self.individual_specs.len();

        let mut schedules_with_rx = Vec::with_capacity(len);
        let mut peer_txs = HashMap::with_capacity(len);

        self.individual_specs.iter().for_each(|schedule| {
            let (peer_tx, peer_rx) = tokio::sync::mpsc::channel::<PeerEvent>(channel_size);
            peer_txs.insert(schedule.id(), PeerEventSender::Up(peer_tx));
            schedules_with_rx.push(SpecWithRx::new(schedule.as_ref(), peer_rx));
        });

        SpecsWithChannels::new(schedules_with_rx, peer_txs)
    }

    pub(crate) async fn init_meta_storage(
        &self,
        client: MetaClient<'_>,
    ) -> Result<(), SchedulerError> {
        init_schema(client).await?;
        self.primary_spec.init_resolution(client).await?;
        for schedule in &self.individual_specs {
            schedule.init_resolution(client).await?;
        }
        for schedule in &self.individual_specs {
            schedule.init_tickets(client).await?;
        }
        init_footprint(client).await?;

        Ok(())
    }

    /// Run a check on the data consistency between the data storage and the metadata storage.
    /// Return `true` if the data storage holds all needed data to restore, or `false` if it does not.
    ///
    /// This should be called only when the recovery state is either `AbortedUnchecked` or `GracefullyStopped`.
    pub(crate) async fn check_consistency(
        &self,
        storage: &Sto,
        client: MetaClient<'_>,
        primary_ub: usize,
    ) -> Result<bool, SchedulerError> {
        if !self
            .primary_spec
            .check_consistency(storage, client, primary_ub)
            .await?
        {
            return Ok(false);
        }
        for schedule in &self.individual_specs {
            if !schedule.check_consistency(storage, client).await? {
                return Ok(false);
            }
        }
        Ok(true)
    }

    pub(crate) async fn clear_resolution(
        &self,
        client: MetaClient<'_>,
    ) -> Result<(), SchedulerError> {
        self.primary_spec.clear_resolution(client).await?;
        for schedule in &self.individual_specs {
            schedule.clear_resolution(client).await?;
        }
        Ok(())
    }

    pub(crate) async fn get_primary_resolution(
        &self,
        client: MetaClient<'_>,
    ) -> Result<Option<usize>, SchedulerError> {
        let primary_resolution = self.primary_spec.get_primary_resolution(client).await?;
        Ok(primary_resolution)
    }

    pub(crate) async fn put_primary_resolution(
        &self,
        client: MetaClient<'_>,
        primary_ub: usize,
    ) -> Result<(), SchedulerError> {
        self.primary_spec
            .put_primary_resolution(client, primary_ub)
            .await?;
        Ok(())
    }

    pub(crate) async fn clear_tickets(&self, client: MetaClient<'_>) -> Result<(), SchedulerError> {
        for schedule in &self.individual_specs {
            schedule.clear_tickets(client).await?;
        }
        Ok(())
    }

    pub(crate) async fn put_default_tickets(
        &self,
        client: MetaClient<'_>,
    ) -> Result<(), SchedulerError> {
        for schedule in &self.individual_specs {
            schedule.put_default_tickets(client).await?;
        }
        Ok(())
    }

    pub(crate) async fn update_ui(
        &self,
        client: MetaClient<'_>,
        ui_state: &mut UiState,
    ) -> Result<(), SchedulerError> {
        for schedule in &self.individual_specs {
            schedule.update_ui(client, ui_state).await?;
        }
        Ok(())
    }

    pub(crate) async fn prepare_rebuilders(
        &self,
        storage: &Sto,
        client: MetaClient<'_>,
    ) -> Result<Vec<Box<dyn JobRebuilder>>, SchedulerError> {
        futures::stream::iter(&self.individual_specs)
            .then(|schedule| schedule.prepare_rebuild(storage, client))
            .try_collect::<Vec<_>>()
            .await
    }
}
