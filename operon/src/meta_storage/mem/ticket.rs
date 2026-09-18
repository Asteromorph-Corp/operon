use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet};
use std::sync::RwLock;

use crate::meta_storage::mem::error::{MemMetaError, MemResult};
use crate::meta_storage::mem::store::MemStore;
use crate::meta_storage::{MetaStorageError, MetaTicketApi};
use crate::schema::{
    DimensionMetadata, Job, OptionCoordinate, Resolution, TableShape, TaskMetadata, Ticket,
    TicketStatus,
};

/// A ticket's primary key: its coordinate, with unresolved dimensions left as `None`.
type TicketKey = Box<[OptionCoordinate]>;

/// A stored ticket's state, keyed in the table by its coordinate.
#[derive(Clone, Copy)]
struct TicketRow {
    deps_done: usize,
    deps_quota: usize,
    status: TicketStatus,
}

/// One task's ticket table.
#[derive(Default)]
pub(super) struct TicketTable {
    rows: RwLock<TicketRows>,
}

/// A ticket table's rows, alongside the per-status counters they are summarized by and the
/// secondary indexes they are queried through.
///
/// One lock spans all three: it makes a slice op's scan and write-back atomic, and it keeps a
/// reader from observing the counters or an index midway through an update.
#[derive(Default)]
struct TicketRows {
    map: HashMap<TicketKey, TicketRow>,
    indexes: HashMap<PinMask, PinIndex>,
    done: i64,
    queued: i64,
    waiting: i64,
}

impl TicketRows {
    /// Adjusts the counter for `status` by `delta`.
    fn count(&mut self, status: TicketStatus, delta: i64) {
        match status {
            TicketStatus::Done => self.done += delta,
            TicketStatus::Queued => self.queued += delta,
            TicketStatus::Waiting => self.waiting += delta,
        }
    }

    /// Adds a key to every index built so far.
    fn index(&mut self, key: &TicketKey) {
        for (mask, index) in &mut self.indexes {
            index
                .entry(mask.apply(key))
                .or_default()
                .insert(key.clone());
        }
    }

    /// Drops a key from every index built so far.
    fn unindex(&mut self, key: &TicketKey) {
        for (mask, index) in &mut self.indexes {
            let Entry::Occupied(mut bucket) = index.entry(mask.apply(key)) else {
                continue;
            };
            bucket.get_mut().remove(key);
            if bucket.get().is_empty() {
                bucket.remove();
            }
        }
    }

    /// The keys whose coordinate matches every pin, building the index for that pin set on first
    /// use.
    ///
    /// A pin set covering every dimension names one coordinate outright, so it is served straight
    /// from the rows.
    fn matching(&mut self, mask: &PinMask, pinned: &TicketKey) -> Vec<TicketKey> {
        if mask.is_full() {
            return Vec::from_iter(self.map.contains_key(pinned).then(|| pinned.clone()));
        }

        if !self.indexes.contains_key(mask) {
            let mut index = PinIndex::default();
            for key in self.map.keys() {
                index
                    .entry(mask.apply(key))
                    .or_default()
                    .insert(key.clone());
            }
            self.indexes.insert(mask.clone(), index);
        }

        self.indexes[mask]
            .get(pinned)
            .map(|bucket| bucket.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// The waiting tickets whose coordinate matches every pin, alongside their keys.
    fn matching_waiting(
        &mut self,
        mask: &PinMask,
        pinned: &TicketKey,
    ) -> Vec<(TicketKey, TicketRow)> {
        self.matching(mask, pinned)
            .into_iter()
            .filter_map(|key| self.map.get(&key).map(|row| (key, *row)))
            .filter(|(_, row)| row.status == TicketStatus::Waiting)
            .collect()
    }

    /// Inserts a ticket, keeping the counters in step.
    fn insert(&mut self, key: TicketKey, row: TicketRow) {
        self.count(row.status, 1);
        self.index(&key);
        self.map.insert(key, row);
    }

    /// Inserts a ticket, leaving an existing one at the same key untouched.
    fn insert_new(&mut self, key: TicketKey, row: TicketRow) {
        if self.map.contains_key(&key) {
            return;
        }
        self.insert(key, row);
    }

    /// Replaces the row at `key`, keeping the counters in step.
    fn replace(&mut self, key: &TicketKey, old_status: TicketStatus, row: TicketRow) {
        self.map.insert(key.clone(), row);
        self.count(old_status, -1);
        self.count(row.status, 1);
    }

    /// Removes the row at `key`, keeping the counters in step.
    fn remove(&mut self, key: &TicketKey) -> Option<TicketRow> {
        let row = self.map.remove(key)?;
        self.count(row.status, -1);
        self.unindex(key);
        Some(row)
    }

    fn clear(&mut self) {
        self.map.clear();
        self.indexes.clear();
        self.done = 0;
        self.queued = 0;
        self.waiting = 0;
    }
}

/// One of a task's dimensions, as a query sees it: pinned to a concrete value, or free to vary.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
enum PinSlot {
    Free,
    Pinned,
}

/// The set of a task's dimensions that a query pins, one slot per dimension.
///
/// Every op pins a subset that the DAG fixes in advance, so a task sees only a handful of masks.
#[derive(Clone, PartialEq, Eq, Hash)]
struct PinMask(Box<[PinSlot]>);

impl PinMask {
    /// Builds the pin set a query fixes and the coordinate it pins to, from the upstream dimensions
    /// and the values they carry.
    ///
    /// A dimension is pinned when it belongs to this task, is not aggregated, and carries a
    /// resolved value; every other dimension stays free.
    fn project<const N: usize>(
        task_meta: TaskMetadata<N>,
        pins: impl IntoIterator<Item = (&'static str, OptionCoordinate)>,
        aggregate_dims: &[&'static str],
    ) -> (Self, TicketKey) {
        let mut slots = vec![PinSlot::Free; N];
        let mut pinned = vec![OptionCoordinate::none(); N];
        for (dim, coord) in pins {
            if aggregate_dims.contains(&dim) {
                continue;
            }
            let Some(value) = coord.0 else {
                continue;
            };
            let Some(idx) = task_meta.dims.iter().position(|d| *d == dim) else {
                continue;
            };
            slots[idx] = PinSlot::Pinned;
            pinned[idx] = OptionCoordinate::some(value);
        }
        (Self(slots.into()), pinned.into())
    }

    /// Projects a coordinate onto this pin set, nulling every dimension the set leaves free.
    ///
    /// Two coordinates share a projection exactly when they agree on every pinned dimension, so a
    /// projection is the bucket key both a stored ticket and a query resolve to.
    fn apply(&self, key: &[OptionCoordinate]) -> TicketKey {
        self.0
            .iter()
            .zip(key)
            .map(|(slot, &coord)| match slot {
                PinSlot::Pinned => coord,
                PinSlot::Free => OptionCoordinate::none(),
            })
            .collect()
    }

    /// Whether the set pins every dimension, which names one coordinate outright.
    fn is_full(&self) -> bool {
        self.0.iter().all(|slot| *slot == PinSlot::Pinned)
    }
}

/// A secondary index over one pin set, mapping a projected coordinate to the keys holding it.
type PinIndex = HashMap<TicketKey, HashSet<TicketKey>>;

/// Helper struct for querying the in-memory tickets of a task.
pub struct MemTicketQueryBuilder<'a, const N: usize> {
    store: &'a MemStore,
    task_meta: TaskMetadata<N>,
}

impl MemStore {
    /// Helper method to create a `MemTicketQueryBuilder` for the tickets of a given task.
    pub(super) fn ticket<const N: usize>(
        &self,
        task_meta: TaskMetadata<N>,
    ) -> MemTicketQueryBuilder<'_, N> {
        MemTicketQueryBuilder {
            store: self,
            task_meta,
        }
    }
}

impl<const N: usize> MemTicketQueryBuilder<'_, N> {
    /// The table this task's tickets live in, if it has been initialized.
    fn table(&self) -> MemResult<Option<std::sync::Arc<TicketTable>>> {
        self.store.ticket_table(self.task_meta.id)
    }

    /// The table this task's tickets live in, erroring if it has not been initialized.
    fn require_table(&self) -> MemResult<std::sync::Arc<TicketTable>> {
        self.table()?.ok_or(MetaStorageError::Internal(
            "ticket table was not initialized",
        ))
    }

    /// Reconstructs a typed ticket from a stored key and row.
    fn ticket_of(key: &TicketKey, row: &TicketRow) -> Ticket<N> {
        let coordinate = std::array::from_fn(|i| key[i]);
        Ticket::from_parts(coordinate, row.deps_done, row.deps_quota, row.status)
    }

    /// Splits a typed ticket into the key and row it is stored as.
    fn split(ticket: &Ticket<N>) -> (TicketKey, TicketRow) {
        let key = Box::from(&ticket.coordinate[..]);
        let row = TicketRow {
            deps_done: ticket.deps_done(),
            deps_quota: ticket.deps_quota(),
            status: ticket.status,
        };
        (key, row)
    }
}

impl<const N: usize> MetaTicketApi<N> for MemTicketQueryBuilder<'_, N> {
    type Error = MemMetaError;

    async fn init(&self) -> MemResult<TableShape> {
        self.store.init_ticket_table(self.task_meta.id)?;
        Ok(TableShape::CURRENT)
    }

    async fn clear(&self) -> MemResult<()> {
        if let Some(table) = self.table()? {
            table.rows.write()?.clear();
        }
        Ok(())
    }

    async fn get_all(&self, status: TicketStatus) -> MemResult<Vec<Ticket<N>>> {
        let Some(table) = self.table()? else {
            return Ok(Vec::new());
        };
        let rows = table.rows.read()?;
        Ok(rows
            .map
            .iter()
            .filter(|(_, row)| row.status == status)
            .map(|(key, row)| Self::ticket_of(key, row))
            .collect())
    }

    async fn put(&self, ticket: Ticket<N>) -> MemResult<()> {
        let table = self.require_table()?;
        let (key, row) = Self::split(&ticket);
        table.rows.write()?.insert_new(key, row);
        Ok(())
    }

    async fn dump(&self) -> MemResult<Vec<Ticket<N>>> {
        let Some(table) = self.table()? else {
            return Ok(Vec::new());
        };
        let rows = table.rows.read()?;
        let tickets = rows
            .map
            .iter()
            .map(|(key, row)| Self::ticket_of(key, row))
            .collect::<Vec<_>>();
        Ok(tickets)
    }

    async fn hydrate(&self, tickets: Vec<Ticket<N>>) -> MemResult<()> {
        let table = self.require_table()?;
        let mut rows = table.rows.write()?;
        rows.clear();
        for ticket in &tickets {
            let (key, row) = Self::split(ticket);
            rows.insert(key, row);
        }
        Ok(())
    }

    async fn raise_deps_done<const M: usize>(
        &self,
        upstream_meta: TaskMetadata<M>,
        upstream_job: Job<M>,
        aggregate_dims: &[&'static str],
    ) -> MemResult<Vec<Ticket<N>>> {
        let pins = upstream_meta
            .dims
            .iter()
            .zip(upstream_job.coordinate)
            .map(|(dim, coord)| (*dim, OptionCoordinate::some(coord)));

        let table = self.require_table()?;
        let mut rows = table.rows.write()?;

        let (mask, pinned) = PinMask::project(self.task_meta, pins, aggregate_dims);
        let stale = rows.matching_waiting(&mask, &pinned);

        let mut promoted = Vec::new();
        for (key, row) in stale {
            let deps_done = row.deps_done + 1;
            let status = if deps_done >= row.deps_quota {
                TicketStatus::Queued
            } else {
                TicketStatus::Waiting
            };
            let raised = TicketRow {
                deps_done,
                deps_quota: row.deps_quota,
                status,
            };
            rows.replace(&key, row.status, raised);
            if status == TicketStatus::Queued {
                promoted.push(Self::ticket_of(&key, &raised));
            }
        }

        Ok(promoted)
    }

    async fn raise_deps_quota<const M: usize>(
        &self,
        upstream_meta: TaskMetadata<M>,
        upstream_ticket: Ticket<M>,
        aggregate_dims: &[&'static str],
        ub: usize,
    ) -> MemResult<Vec<Ticket<N>>> {
        let pins = upstream_meta
            .dims
            .iter()
            .copied()
            .zip(upstream_ticket.coordinate);

        let table = self.require_table()?;
        let mut rows = table.rows.write()?;

        let (mask, pinned) = PinMask::project(self.task_meta, pins, aggregate_dims);
        let stale = rows.matching_waiting(&mask, &pinned);

        let mut promoted = Vec::new();
        for (key, row) in stale {
            // Mirrors the backing `deps_quota + $ub - 1` arithmetic, which is signed.
            let quota = i64::try_from(row.deps_quota)? + i64::try_from(ub)? - 1;
            let status = if i64::try_from(row.deps_done)? >= quota {
                TicketStatus::Queued
            } else {
                TicketStatus::Waiting
            };
            let quota = usize::try_from(quota)?;
            let raised = TicketRow {
                deps_done: row.deps_done,
                deps_quota: quota,
                status,
            };
            rows.replace(&key, row.status, raised);
            if status == TicketStatus::Queued {
                promoted.push(Self::ticket_of(&key, &raised));
            }
        }

        Ok(promoted)
    }

    async fn explode<const M: usize, const IDX: usize>(
        &self,
        res_meta: DimensionMetadata<M>,
        res: Resolution<M>,
    ) -> MemResult<Vec<Ticket<N>>> {
        const { assert!(IDX < N) }
        if self.task_meta.dims[IDX] != res_meta.id {
            tracing::warn!("Invalid resolution received for explosion.");
            return Ok(vec![]);
        }

        let pins = res_meta
            .deps
            .iter()
            .zip(res.coordinate)
            .map(|(dim, coord)| (*dim, OptionCoordinate::some(coord)));

        let table = self.require_table()?;
        let mut rows = table.rows.write()?;

        let (mask, pinned) = PinMask::project(self.task_meta, pins, &[]);
        let popped_keys = rows.matching(&mask, &pinned);

        let popped = popped_keys
            .into_iter()
            .filter_map(|key| rows.remove(&key).map(|row| (key, row)))
            .collect::<Vec<_>>();
        let tickets = popped
            .iter()
            .map(|(key, row)| Self::ticket_of(key, row))
            .collect::<Vec<_>>();

        if tickets
            .iter()
            .any(|ticket| ticket.coordinate[IDX].is_some())
        {
            return Err(MetaStorageError::invalid_explosion(
                self.task_meta.id,
                res_meta.id,
            ));
        }

        for ticket in &tickets {
            for coord in 0..res.ub {
                let exploded = ticket.with_coordinate::<IDX>(coord).update_status();
                let (key, row) = Self::split(&exploded);
                rows.insert_new(key, row);
            }
        }

        Ok(tickets)
    }

    async fn mark_done(&self, job: Job<N>) -> MemResult<()> {
        let table = self.require_table()?;
        let mut rows = table.rows.write()?;

        let key: TicketKey = job
            .coordinate
            .iter()
            .map(|&c| OptionCoordinate::some(c))
            .collect();
        let Some(row) = rows.map.get(&key).copied() else {
            return Ok(());
        };
        let done = TicketRow {
            status: TicketStatus::Done,
            ..row
        };
        rows.replace(&key, row.status, done);

        Ok(())
    }

    async fn get_status(&self) -> MemResult<(i64, i64, i64)> {
        let Some(table) = self.table()? else {
            return Err(MetaStorageError::missing_ticket_summary(self.task_meta.id));
        };
        let rows = table.rows.read()?;
        Ok((rows.done, rows.queued, rows.waiting))
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;
    use crate::meta_storage::mem::{MemConn, MemMetaStorage};
    use crate::meta_storage::{MetaBackend, MetaClientApi, MetaConnApi};

    /// A task over one dimension `i`, which is also the dimension it spawns.
    fn task_beta() -> TaskMetadata<1> {
        TaskMetadata {
            id: "beta",
            dims: ["i"],
            spawn_dim: Some("i"),
            priority: &[],
        }
    }

    /// An upstream task over the same dimension `i`.
    fn task_alpha() -> TaskMetadata<1> {
        TaskMetadata {
            id: "alpha",
            dims: ["i"],
            spawn_dim: None,
            priority: &[],
        }
    }

    /// A task over two dimensions, whose upstreams pin one dimension each.
    fn task_gamma() -> TaskMetadata<2> {
        TaskMetadata {
            id: "gamma",
            dims: ["i", "j"],
            spawn_dim: None,
            priority: &[],
        }
    }

    /// An upstream task over `j` alone.
    fn task_over_j() -> TaskMetadata<1> {
        TaskMetadata {
            id: "over_j",
            dims: ["j"],
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
        let tickets = client.ticket(task_beta());
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
    async fn put_leaves_existing_coordinate_untouched() {
        let (_backend, conn) = store().await;
        let client = conn.as_client();
        let tickets = client.ticket(task_beta());
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
    async fn raise_deps_done_promotes_only_pinned_coordinate() {
        let (_backend, conn) = store().await;
        let client = conn.as_client();
        let tickets = client.ticket(task_beta());
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
            .raise_deps_done(task_alpha(), Job { coordinate: [0] }, &[])
            .await
            .unwrap();

        assert_eq!(promoted.len(), 1);
        assert_eq!(promoted[0].coordinate[0].0, Some(0));
        assert_eq!(tickets.get_status().await.unwrap(), (0, 1, 1));
    }

    #[tokio::test]
    async fn raise_deps_done_holds_ticket_short_of_its_quota() {
        let (_backend, conn) = store().await;
        let client = conn.as_client();
        let tickets = client.ticket(task_beta());
        tickets.init().await.unwrap();

        tickets
            .put(Ticket::new(2).with_coordinate::<0>(0))
            .await
            .unwrap();

        let promoted = tickets
            .raise_deps_done(task_alpha(), Job { coordinate: [0] }, &[])
            .await
            .unwrap();

        assert!(promoted.is_empty());
        assert_eq!(tickets.get_status().await.unwrap(), (0, 0, 1));
    }

    #[tokio::test]
    async fn raise_deps_quota_reopens_ready_ticket() {
        let (_backend, conn) = store().await;
        let client = conn.as_client();
        let tickets = client.ticket(task_beta());
        tickets.init().await.unwrap();

        tickets
            .put(Ticket::new(1).with_coordinate::<0>(0))
            .await
            .unwrap();

        // A quota of 3 leaves the ticket needing 3 dependencies, none of which are done.
        let promoted = tickets
            .raise_deps_quota(task_alpha(), Ticket::new(1).with_coordinate::<0>(0), &[], 3)
            .await
            .unwrap();

        assert!(promoted.is_empty());
        assert_eq!(tickets.get_status().await.unwrap(), (0, 0, 1));
    }

    #[tokio::test]
    async fn explode_expands_ticket_along_its_spawn_dimension() {
        let (_backend, conn) = store().await;
        let client = conn.as_client();
        let tickets = client.ticket(task_beta());
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
    async fn explode_rejects_already_resolved_dimension() {
        let (_backend, conn) = store().await;
        let client = conn.as_client();
        let tickets = client.ticket(task_beta());
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
    async fn mark_done_moves_ticket_to_done() {
        let (_backend, conn) = store().await;
        let client = conn.as_client();
        let tickets = client.ticket(task_beta());
        tickets.init().await.unwrap();

        tickets
            .put(Ticket::new(0).with_coordinate::<0>(0))
            .await
            .unwrap();
        tickets.mark_done(Job { coordinate: [0] }).await.unwrap();

        assert_eq!(tickets.get_status().await.unwrap(), (1, 0, 0));
    }

    #[tokio::test]
    async fn clear_resets_rows_and_counters() {
        let (_backend, conn) = store().await;
        let client = conn.as_client();
        let tickets = client.ticket(task_beta());
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
    async fn get_status_reports_missing_table() {
        let (_backend, conn) = store().await;
        let client = conn.as_client();
        let tickets = client.ticket(task_beta());

        assert!(matches!(
            tickets.get_status().await,
            Err(MetaStorageError::MissingTicketSummary { .. })
        ));
    }

    #[tokio::test]
    async fn put_reaches_index_built_before_it() {
        let (_backend, conn) = store().await;
        let client = conn.as_client();
        let tickets = client.ticket(task_beta());
        tickets.init().await.unwrap();

        tickets
            .put(Ticket::new(1).with_coordinate::<0>(0))
            .await
            .unwrap();
        tickets
            .raise_deps_done(task_alpha(), Job { coordinate: [0] }, &[])
            .await
            .unwrap();

        tickets
            .put(Ticket::new(1).with_coordinate::<0>(1))
            .await
            .unwrap();
        let promoted = tickets
            .raise_deps_done(task_alpha(), Job { coordinate: [1] }, &[])
            .await
            .unwrap();

        assert_eq!(promoted.len(), 1);
        assert_eq!(promoted[0].coordinate[0].0, Some(1));
    }

    #[tokio::test]
    async fn explosion_reaches_index_built_before_it() {
        let (_backend, conn) = store().await;
        let client = conn.as_client();
        let tickets = client.ticket(task_beta());
        tickets.init().await.unwrap();

        tickets.put(Ticket::new(1)).await.unwrap();
        // Builds the index over `i` while the only ticket is still unexploded.
        tickets
            .raise_deps_done(task_alpha(), Job { coordinate: [1] }, &[])
            .await
            .unwrap();

        tickets
            .explode::<0, 0>(
                DimensionMetadata { id: "i", deps: [] },
                Resolution {
                    coordinate: [],
                    ub: 3,
                },
            )
            .await
            .unwrap();

        let promoted = tickets
            .raise_deps_done(task_alpha(), Job { coordinate: [1] }, &[])
            .await
            .unwrap();

        assert_eq!(promoted.len(), 1);
        assert_eq!(promoted[0].coordinate[0].0, Some(1));
        assert_eq!(tickets.get_status().await.unwrap(), (0, 1, 2));
    }

    #[tokio::test]
    async fn clear_drops_index_built_before_it() {
        let (_backend, conn) = store().await;
        let client = conn.as_client();
        let tickets = client.ticket(task_beta());
        tickets.init().await.unwrap();

        tickets
            .put(Ticket::new(1).with_coordinate::<0>(0))
            .await
            .unwrap();
        tickets
            .raise_deps_done(task_alpha(), Job { coordinate: [0] }, &[])
            .await
            .unwrap();
        tickets.clear().await.unwrap();

        tickets
            .put(Ticket::new(1).with_coordinate::<0>(0))
            .await
            .unwrap();
        let promoted = tickets
            .raise_deps_done(task_alpha(), Job { coordinate: [0] }, &[])
            .await
            .unwrap();

        assert_eq!(promoted.len(), 1);
        assert_eq!(tickets.get_status().await.unwrap(), (0, 1, 0));
    }

    #[tokio::test]
    async fn upstreams_pinning_different_dimensions_keep_separate_indexes() {
        let (_backend, conn) = store().await;
        let client = conn.as_client();
        let tickets = client.ticket(task_gamma());
        tickets.init().await.unwrap();

        for i in 0..2 {
            for j in 0..2 {
                tickets
                    .put(
                        Ticket::new(2)
                            .with_coordinate::<0>(i)
                            .with_coordinate::<1>(j),
                    )
                    .await
                    .unwrap();
            }
        }

        // Pins `i` alone: reaches (0,0) and (0,1), neither of which meets its quota of 2.
        let promoted = tickets
            .raise_deps_done(task_alpha(), Job { coordinate: [0] }, &[])
            .await
            .unwrap();
        assert!(promoted.is_empty());

        // Pins `j` alone: reaches (0,0) and (1,0), so only (0,0) reaches its quota.
        let promoted = tickets
            .raise_deps_done(task_over_j(), Job { coordinate: [0] }, &[])
            .await
            .unwrap();

        assert_eq!(promoted.len(), 1);
        assert_eq!(promoted[0].coordinate.map(|c| c.0), [Some(0), Some(0)]);
        assert_eq!(tickets.get_status().await.unwrap(), (0, 1, 3));
    }

    #[tokio::test]
    async fn init_is_idempotent() {
        let (_backend, conn) = store().await;
        let client = conn.as_client();
        let tickets = client.ticket(task_beta());
        tickets.init().await.unwrap();
        tickets
            .put(Ticket::new(1).with_coordinate::<0>(0))
            .await
            .unwrap();
        tickets.init().await.unwrap();

        assert_eq!(tickets.get_status().await.unwrap(), (0, 0, 1));
    }

    /// Every operation on a table whose lock a panic has poisoned reports it as a backend error.
    #[tokio::test]
    async fn a_poisoned_table_errors_rather_than_panicking() {
        let store = MemStore::default();
        let tickets = store.ticket(task_beta());
        tickets.init().await.unwrap();

        let table = store.ticket_table("beta").unwrap().expect("table");
        let panicked = std::thread::spawn(move || {
            let _guard = table.rows.write().expect("uncontended");
            panic!("a store operation unwinds while holding the lock");
        })
        .join();
        assert!(panicked.is_err());

        assert!(matches!(
            tickets.get_status().await,
            Err(MetaStorageError::Backend(MemMetaError::Poisoned))
        ));
        assert!(matches!(
            tickets.put(Ticket::new(0)).await,
            Err(MetaStorageError::Backend(MemMetaError::Poisoned))
        ));
        assert!(matches!(
            tickets.get_all(TicketStatus::Queued).await,
            Err(MetaStorageError::Backend(MemMetaError::Poisoned))
        ));
    }
}
