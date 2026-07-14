use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::sync::RwLock;

use crate::meta_storage::mem::error::{MemResult, POISONED};
use crate::meta_storage::mem::store::MemStore;
use crate::meta_storage::{MetaStorageError, MetaTicketApi};
use crate::schema::{DimensionMetadata, Job, JobMetadata, Resolution, Ticket, TicketStatus};

/// A ticket's primary key: its coordinate, with unresolved dimensions left as `None`.
///
/// Stored natively rather than through the Postgres `-1` sentinel.
pub(super) type TicketKey<const N: usize> = [Option<usize>; N];

/// Derives a ticket's key from its coordinate.
fn key_of<const N: usize>(ticket: &Ticket<N>) -> TicketKey<N> {
    ticket.coordinate.map(|coord| coord.0)
}

/// One job's ticket table.
#[derive(Default)]
pub(super) struct TicketTable<const N: usize> {
    rows: RwLock<TicketRows<N>>,
}

/// A ticket table's rows, alongside the per-status counters they are summarized by.
///
/// The counters live under the same lock as the rows so they cannot drift from them.
#[derive(Default)]
struct TicketRows<const N: usize> {
    map: HashMap<TicketKey<N>, Ticket<N>>,
    done: i64,
    queued: i64,
    waiting: i64,
}

impl<const N: usize> TicketRows<N> {
    /// Adjusts the counter for `status` by `delta`.
    fn count(&mut self, status: TicketStatus, delta: i64) {
        match status {
            TicketStatus::Done => self.done += delta,
            TicketStatus::Queued => self.queued += delta,
            TicketStatus::Waiting => self.waiting += delta,
        }
    }

    /// Inserts a ticket, leaving an existing one at the same key untouched.
    fn insert_new(&mut self, ticket: Ticket<N>) {
        if let Entry::Vacant(entry) = self.map.entry(key_of(&ticket)) {
            entry.insert(ticket);
            self.count(ticket.status, 1);
        }
    }

    /// Replaces the ticket at `key`, keeping the counters in step.
    fn replace(&mut self, key: TicketKey<N>, old_status: TicketStatus, ticket: Ticket<N>) {
        self.map.insert(key, ticket);
        self.count(old_status, -1);
        self.count(ticket.status, 1);
    }

    /// Removes the ticket at `key`, keeping the counters in step.
    fn remove(&mut self, key: &TicketKey<N>) -> Option<Ticket<N>> {
        let ticket = self.map.remove(key)?;
        self.count(ticket.status, -1);
        Some(ticket)
    }

    fn clear(&mut self) {
        self.map.clear();
        self.done = 0;
        self.queued = 0;
        self.waiting = 0;
    }
}

/// A dimension pinned to a concrete value: the index into a job's `dims`, and the value the
/// ticket's coordinate must hold there.
type Pin = (usize, usize);

/// Whether a ticket's coordinate matches every pin.
fn matches<const N: usize>(ticket: &Ticket<N>, pins: &[Pin]) -> bool {
    pins.iter()
        .all(|&(idx, value)| ticket.coordinate[idx].0 == Some(value))
}

/// Helper struct for querying the in-memory tickets of a job.
pub struct MemTicketQueryBuilder<'a, const N: usize> {
    store: &'a MemStore,
    job_meta: JobMetadata<N>,
}

impl MemStore {
    /// Helper method to create a `MemTicketQueryBuilder` for a ticket of given job.
    pub(super) fn ticket<const N: usize>(
        &self,
        job_meta: JobMetadata<N>,
    ) -> MemTicketQueryBuilder<'_, N> {
        MemTicketQueryBuilder {
            store: self,
            job_meta,
        }
    }
}

impl<const N: usize> MemTicketQueryBuilder<'_, N> {
    /// Resolves the index of `dim` within this job's dimensions.
    fn dim_index(&self, dim: &str) -> MemResult<usize> {
        self.job_meta
            .dims
            .iter()
            .position(|d| *d == dim)
            .ok_or(MetaStorageError::Internal(
                "upstream dimension is not a dimension of this job",
            ))
    }

    /// The table this job's tickets live in, if it has been initialized.
    fn table(&self) -> MemResult<Option<std::sync::Arc<TicketTable<N>>>> {
        self.store.ticket_table::<N>(self.job_meta.id)
    }

    /// The table this job's tickets live in, erroring if it has not been initialized.
    fn require_table(&self) -> MemResult<std::sync::Arc<TicketTable<N>>> {
        self.table()?.ok_or(MetaStorageError::Internal(
            "ticket table was not initialized",
        ))
    }
}

impl<const N: usize> MetaTicketApi<N> for MemTicketQueryBuilder<'_, N> {
    type Error = std::convert::Infallible;

    /// Initializes the ticket table.
    async fn init(&self) -> MemResult<()> {
        self.store.init_ticket_table::<N>(self.job_meta.id);
        Ok(())
    }

    /// Clears the ticket table.
    async fn clear(&self) -> MemResult<()> {
        if let Some(table) = self.table()? {
            table.rows.write().expect(POISONED).clear();
        }
        Ok(())
    }

    /// Gets all tickets with a given status.
    async fn get_all(&self, status: TicketStatus) -> MemResult<Vec<Ticket<N>>> {
        let Some(table) = self.table()? else {
            return Ok(Vec::new());
        };
        let rows = table.rows.read().expect(POISONED);
        Ok(rows
            .map
            .values()
            .filter(|ticket| ticket.status == status)
            .copied()
            .collect())
    }

    /// Puts a ticket into the table, leaving an existing one at the same coordinate untouched.
    async fn put(&self, ticket: Ticket<N>) -> MemResult<()> {
        let table = self.require_table()?;
        table.rows.write().expect(POISONED).insert_new(ticket);
        Ok(())
    }

    /// Raises the `deps_done` count of eligible tickets by 1.
    ///
    /// Returns tickets that are newly `"queued"`.
    async fn raise_deps_done<const M: usize>(
        &self,
        upstream_meta: JobMetadata<M>,
        upstream_job: Job<M>,
        aggregate_dims: &[&'static str],
    ) -> MemResult<Vec<Ticket<N>>> {
        let pins = upstream_meta
            .dims
            .iter()
            .zip(upstream_job.coordinate)
            .filter(|(dim, _)| self.job_meta.dims.contains(dim) && !aggregate_dims.contains(dim))
            .map(|(dim, coord)| self.dim_index(dim).map(|idx| (idx, coord)))
            .collect::<MemResult<Vec<Pin>>>()?;

        let table = self.require_table()?;
        let mut rows = table.rows.write().expect(POISONED);

        let stale = rows
            .map
            .iter()
            .filter(|(_, ticket)| ticket.status == TicketStatus::Waiting && matches(ticket, &pins))
            .map(|(key, ticket)| (*key, *ticket))
            .collect::<Vec<_>>();

        let mut promoted = Vec::new();
        for (key, ticket) in stale {
            let deps_done = ticket.deps_done() + 1;
            let status = if deps_done >= ticket.deps_quota() {
                TicketStatus::Queued
            } else {
                TicketStatus::Waiting
            };
            let raised =
                Ticket::from_parts(ticket.coordinate, deps_done, ticket.deps_quota(), status);
            rows.replace(key, ticket.status, raised);
            if status == TicketStatus::Queued {
                promoted.push(raised);
            }
        }

        Ok(promoted)
    }

    /// Raises the `deps_quota` count of eligible tickets by resolution's `ub` minus 1.
    ///
    /// Returns tickets that are newly `"queued"`.
    async fn raise_deps_quota<const M: usize>(
        &self,
        upstream_meta: JobMetadata<M>,
        upstream_ticket: Ticket<M>,
        aggregate_dims: &[&'static str],
        ub: usize,
    ) -> MemResult<Vec<Ticket<N>>> {
        let pins = upstream_meta
            .dims
            .iter()
            .zip(upstream_ticket.coordinate)
            .filter_map(|(&dim, coord)| {
                if aggregate_dims.contains(&dim) {
                    return None;
                }
                Some((dim, coord.0?))
            })
            .map(|(dim, coord)| self.dim_index(dim).map(|idx| (idx, coord)))
            .collect::<MemResult<Vec<Pin>>>()?;

        let table = self.require_table()?;
        let mut rows = table.rows.write().expect(POISONED);

        let stale = rows
            .map
            .iter()
            .filter(|(_, ticket)| ticket.status == TicketStatus::Waiting && matches(ticket, &pins))
            .map(|(key, ticket)| (*key, *ticket))
            .collect::<Vec<_>>();

        let mut promoted = Vec::new();
        for (key, ticket) in stale {
            // Mirrors the backing `deps_quota + $ub - 1` arithmetic, which is signed.
            let quota = i64::try_from(ticket.deps_quota())? + i64::try_from(ub)? - 1;
            let status = if i64::try_from(ticket.deps_done())? >= quota {
                TicketStatus::Queued
            } else {
                TicketStatus::Waiting
            };
            let quota = usize::try_from(quota)?;
            let raised = Ticket::from_parts(ticket.coordinate, ticket.deps_done(), quota, status);
            rows.replace(key, ticket.status, raised);
            if status == TicketStatus::Queued {
                promoted.push(raised);
            }
        }

        Ok(promoted)
    }

    /// Explodes the ticket along a dimension at a given coordinate.
    ///
    /// Returns tickets affected.
    async fn explode<const M: usize, const IDX: usize>(
        &self,
        res_meta: DimensionMetadata<M>,
        res: Resolution<M>,
    ) -> MemResult<Vec<Ticket<N>>> {
        const { assert!(IDX < N) }
        if self.job_meta.dims[IDX] != res_meta.id {
            tracing::warn!("Invalid resolution received for explosion.");
            return Ok(vec![]);
        }

        let pins = res_meta
            .deps
            .iter()
            .zip(res.coordinate)
            .filter(|(dim, _)| self.job_meta.dims.contains(dim))
            .map(|(dim, coord)| self.dim_index(dim).map(|idx| (idx, coord)))
            .collect::<MemResult<Vec<Pin>>>()?;

        let table = self.require_table()?;
        let mut rows = table.rows.write().expect(POISONED);

        let popped_keys = rows
            .map
            .iter()
            .filter(|(_, ticket)| matches(ticket, &pins))
            .map(|(key, _)| *key)
            .collect::<Vec<_>>();

        let tickets = popped_keys
            .iter()
            .filter_map(|key| rows.remove(key))
            .collect::<Vec<_>>();

        if tickets
            .iter()
            .any(|ticket| ticket.coordinate[IDX].is_some())
        {
            return Err(MetaStorageError::invalid_explosion(
                self.job_meta.id,
                res_meta.id,
            ));
        }

        for ticket in &tickets {
            for coord in 0..res.ub {
                rows.insert_new(ticket.with_coordinate::<IDX>(coord).update_status());
            }
        }

        Ok(tickets)
    }

    /// Marks the ticket corresponding to a given job as done.
    async fn mark_done(&self, job: Job<N>) -> MemResult<()> {
        let table = self.require_table()?;
        let mut rows = table.rows.write().expect(POISONED);

        let key = job.coordinate.map(Some);
        let Some(ticket) = rows.map.get(&key).copied() else {
            return Ok(());
        };
        let done = Ticket::from_parts(
            ticket.coordinate,
            ticket.deps_done(),
            ticket.deps_quota(),
            TicketStatus::Done,
        );
        rows.replace(key, ticket.status, done);

        Ok(())
    }

    async fn get_status(&self) -> MemResult<(i64, i64, i64)> {
        let Some(table) = self.table()? else {
            return Err(MetaStorageError::missing_ticket_summary(self.job_meta.id));
        };
        let rows = table.rows.read().expect(POISONED);
        Ok((rows.done, rows.queued, rows.waiting))
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;
    use crate::meta_storage::mem::{MemConn, MemMetaStorage};
    use crate::meta_storage::{MetaBackend, MetaClientApi, MetaConnApi};

    /// A job over one dimension `i`, which is also the dimension it spawns.
    fn job_beta() -> JobMetadata<1> {
        JobMetadata {
            id: "beta",
            dims: ["i"],
            spawn_dim: Some("i"),
            priority: &[],
        }
    }

    /// An upstream job over the same dimension `i`.
    fn job_alpha() -> JobMetadata<1> {
        JobMetadata {
            id: "alpha",
            dims: ["i"],
            spawn_dim: None,
            priority: &[],
        }
    }

    /// Builds an initialized store, returning it alongside a connection to borrow clients from.
    async fn store() -> (MemMetaStorage, MemConn) {
        let backend = MemMetaStorage::default();
        let conn = backend.worker_conn().await.expect("conn");
        (backend, conn)
    }

    #[tokio::test]
    async fn put_counts_tickets_by_status() {
        let (_backend, conn) = store().await;
        let client = conn.as_client();
        let tickets = client.ticket(job_beta());
        tickets.init().await.unwrap();

        tickets
            .put(Ticket::new(1).with_coordinate::<0>(0))
            .await
            .unwrap();
        tickets
            .put(Ticket::new(0).with_coordinate::<0>(1))
            .await
            .unwrap();

        // (done, queued, waiting): a zero quota is ready on arrival.
        assert_eq!(tickets.get_status().await.unwrap(), (0, 1, 1));
        assert_eq!(
            tickets.get_all(TicketStatus::Queued).await.unwrap().len(),
            1
        );
    }

    #[tokio::test]
    async fn put_leaves_an_existing_coordinate_untouched() {
        let (_backend, conn) = store().await;
        let client = conn.as_client();
        let tickets = client.ticket(job_beta());
        tickets.init().await.unwrap();

        tickets
            .put(Ticket::new(1).with_coordinate::<0>(0))
            .await
            .unwrap();
        tickets
            .put(Ticket::new(0).with_coordinate::<0>(0))
            .await
            .unwrap();

        assert_eq!(tickets.get_status().await.unwrap(), (0, 0, 1));
    }

    #[tokio::test]
    async fn raise_deps_done_promotes_only_the_pinned_coordinate() {
        let (_backend, conn) = store().await;
        let client = conn.as_client();
        let tickets = client.ticket(job_beta());
        tickets.init().await.unwrap();

        tickets
            .put(Ticket::new(1).with_coordinate::<0>(0))
            .await
            .unwrap();
        tickets
            .put(Ticket::new(1).with_coordinate::<0>(1))
            .await
            .unwrap();

        let promoted = tickets
            .raise_deps_done(job_alpha(), Job { coordinate: [0] }, &[])
            .await
            .unwrap();

        assert_eq!(promoted.len(), 1);
        assert_eq!(promoted[0].coordinate[0].0, Some(0));
        assert_eq!(tickets.get_status().await.unwrap(), (0, 1, 1));
    }

    #[tokio::test]
    async fn raise_deps_done_holds_a_ticket_short_of_its_quota() {
        let (_backend, conn) = store().await;
        let client = conn.as_client();
        let tickets = client.ticket(job_beta());
        tickets.init().await.unwrap();

        tickets
            .put(Ticket::new(2).with_coordinate::<0>(0))
            .await
            .unwrap();

        let promoted = tickets
            .raise_deps_done(job_alpha(), Job { coordinate: [0] }, &[])
            .await
            .unwrap();

        assert!(promoted.is_empty());
        assert_eq!(tickets.get_status().await.unwrap(), (0, 0, 1));
    }

    #[tokio::test]
    async fn raise_deps_quota_reopens_a_ready_ticket() {
        let (_backend, conn) = store().await;
        let client = conn.as_client();
        let tickets = client.ticket(job_beta());
        tickets.init().await.unwrap();

        tickets
            .put(Ticket::new(1).with_coordinate::<0>(0))
            .await
            .unwrap();

        // A quota of 3 leaves the ticket needing 3 dependencies, none of which are done.
        let promoted = tickets
            .raise_deps_quota(job_alpha(), Ticket::new(1).with_coordinate::<0>(0), &[], 3)
            .await
            .unwrap();

        assert!(promoted.is_empty());
        assert_eq!(tickets.get_status().await.unwrap(), (0, 0, 1));
    }

    #[tokio::test]
    async fn explode_expands_a_ticket_along_its_spawn_dimension() {
        let (_backend, conn) = store().await;
        let client = conn.as_client();
        let tickets = client.ticket(job_beta());
        tickets.init().await.unwrap();

        tickets.put(Ticket::new(0)).await.unwrap();

        let popped = tickets
            .explode::<0, 0>(
                DimensionMetadata { id: "i", deps: [] },
                Resolution {
                    coordinate: [],
                    ub: 3,
                },
            )
            .await
            .unwrap();

        assert_eq!(popped.len(), 1);
        assert_eq!(tickets.get_status().await.unwrap(), (0, 3, 0));
        let queued = tickets.get_all(TicketStatus::Queued).await.unwrap();
        let mut coords = queued
            .iter()
            .map(|ticket| ticket.coordinate[0].0)
            .collect::<Vec<_>>();
        coords.sort();
        assert_eq!(coords, vec![Some(0), Some(1), Some(2)]);
    }

    #[tokio::test]
    async fn explode_rejects_an_already_resolved_dimension() {
        let (_backend, conn) = store().await;
        let client = conn.as_client();
        let tickets = client.ticket(job_beta());
        tickets.init().await.unwrap();

        tickets
            .put(Ticket::new(0).with_coordinate::<0>(0))
            .await
            .unwrap();

        let exploded = tickets
            .explode::<0, 0>(
                DimensionMetadata { id: "i", deps: [] },
                Resolution {
                    coordinate: [],
                    ub: 2,
                },
            )
            .await;

        assert!(matches!(
            exploded,
            Err(MetaStorageError::InvalidExplosion { .. })
        ));
    }

    #[tokio::test]
    async fn mark_done_moves_a_ticket_to_done() {
        let (_backend, conn) = store().await;
        let client = conn.as_client();
        let tickets = client.ticket(job_beta());
        tickets.init().await.unwrap();

        tickets
            .put(Ticket::new(0).with_coordinate::<0>(0))
            .await
            .unwrap();
        tickets.mark_done(Job { coordinate: [0] }).await.unwrap();

        assert_eq!(tickets.get_status().await.unwrap(), (1, 0, 0));
    }

    #[tokio::test]
    async fn clear_resets_the_rows_and_their_counters() {
        let (_backend, conn) = store().await;
        let client = conn.as_client();
        let tickets = client.ticket(job_beta());
        tickets.init().await.unwrap();

        tickets
            .put(Ticket::new(1).with_coordinate::<0>(0))
            .await
            .unwrap();
        tickets.clear().await.unwrap();

        assert_eq!(tickets.get_status().await.unwrap(), (0, 0, 0));
        assert!(
            tickets
                .get_all(TicketStatus::Waiting)
                .await
                .unwrap()
                .is_empty()
        );
    }

    #[tokio::test]
    async fn get_status_reports_a_missing_table() {
        let (_backend, conn) = store().await;
        let client = conn.as_client();
        let tickets = client.ticket(job_beta());

        assert!(matches!(
            tickets.get_status().await,
            Err(MetaStorageError::MissingTicketSummary { .. })
        ));
    }

    #[tokio::test]
    async fn init_is_idempotent() {
        let (_backend, conn) = store().await;
        let client = conn.as_client();
        let tickets = client.ticket(job_beta());
        tickets.init().await.unwrap();
        tickets
            .put(Ticket::new(1).with_coordinate::<0>(0))
            .await
            .unwrap();
        tickets.init().await.unwrap();

        assert_eq!(tickets.get_status().await.unwrap(), (0, 0, 1));
    }
}
