use std::cmp::Ordering;
use std::collections::{BinaryHeap, VecDeque};
use std::sync::Arc;

use super::SchedulerError;
use crate::schema::{Direction, JobMetadata, Ticket};

pub(super) trait JobQueue<T> {
    fn push(&mut self, item: T) -> Result<(), SchedulerError>;
    fn extend<I>(&mut self, iter: I) -> Result<(), SchedulerError>
    where
        I: IntoIterator<Item = T>;
    fn pop(&mut self) -> Option<T>;
    fn is_empty(&self) -> bool;
}

#[derive(Default)]
pub(super) struct DequeJobQueue<T>(VecDeque<T>);
impl<T> From<Vec<T>> for DequeJobQueue<T> {
    fn from(vec: Vec<T>) -> Self {
        Self(vec.into())
    }
}
impl<T> JobQueue<T> for DequeJobQueue<T> {
    fn push(&mut self, item: T) -> Result<(), SchedulerError> {
        self.0.push_back(item);
        Ok(())
    }

    fn extend<I>(&mut self, iter: I) -> Result<(), SchedulerError>
    where
        I: IntoIterator<Item = T>,
    {
        self.0.extend(iter);
        Ok(())
    }

    fn pop(&mut self) -> Option<T> {
        self.0.pop_front()
    }

    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

struct PriorityEntry<const N: usize> {
    order: Arc<[(usize, Direction)]>,
    seq: u64,
    item: Ticket<N>,
}
impl<const N: usize> Eq for PriorityEntry<N> {}
impl<const N: usize> PartialEq for PriorityEntry<N> {
    fn eq(&self, other: &Self) -> bool {
        self.seq == other.seq
    }
}
impl<const N: usize> PartialOrd for PriorityEntry<N> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl<const N: usize> Ord for PriorityEntry<N> {
    fn cmp(&self, other: &Self) -> Ordering {
        for &(idx, dir) in self.order.iter() {
            // Both are Some() since the JobQueue only ever pushes ready tickets,
            // with all coordinates resolved.
            let a = self.item.coordinate[idx].0;
            let b = other.item.coordinate[idx].0;
            let ord = dir.apply(a.cmp(&b));
            if ord != Ordering::Equal {
                return ord;
            }
        }
        // Monotonic FIFO tie-break, robust to `next_seq` wrapping.
        // Correct as long as the live-entry seq window stays below 2^63.
        // Notably does not simplify to `other.seq.cmp(&self.seq)`.
        (other.seq.wrapping_sub(self.seq) as i64).cmp(&0)
    }
}

pub(super) struct PriorityJobQueue<const N: usize> {
    queue: BinaryHeap<PriorityEntry<N>>,
    next_seq: u64,
    order: Arc<[(usize, Direction)]>,
}

impl<const N: usize> PriorityJobQueue<N> {
    fn new(
        vec: Vec<Ticket<N>>,
        priority: &[(&'static str, Direction)],
        dims: &[&'static str; N],
    ) -> Result<Self, SchedulerError> {
        let order: Arc<[(usize, Direction)]> = priority
            .iter()
            .map(|&(dim, dir)| {
                dims.iter()
                    .position(|d| *d == dim)
                    .map(|idx| (idx, dir))
                    .ok_or_else(|| {
                        SchedulerError::other(format!(
                            "Priority dimension `{dim}` not found in job dimensions"
                        ))
                    })
            })
            .collect::<Result<Vec<_>, _>>()?
            .into();
        let mut queue = Self {
            queue: BinaryHeap::with_capacity(vec.len()),
            next_seq: 0,
            order,
        };
        queue.extend(vec)?;
        Ok(queue)
    }
}

impl<const N: usize> JobQueue<Ticket<N>> for PriorityJobQueue<N> {
    fn push(&mut self, item: Ticket<N>) -> Result<(), SchedulerError> {
        for &(idx, _) in self.order.iter() {
            if item.coordinate[idx].0.is_none() {
                return Err(SchedulerError::other(format!(
                    "Unresolved coordinate at index {idx} pushed to priority job queue"
                )));
            }
        }
        let seq = self.next_seq;
        self.next_seq = self.next_seq.wrapping_add(1);
        self.queue.push(PriorityEntry {
            order: Arc::clone(&self.order),
            seq,
            item,
        });
        Ok(())
    }

    fn extend<I>(&mut self, iter: I) -> Result<(), SchedulerError>
    where
        I: IntoIterator<Item = Ticket<N>>,
    {
        for item in iter {
            self.push(item)?;
        }
        Ok(())
    }

    fn pop(&mut self) -> Option<Ticket<N>> {
        self.queue.pop().map(|e| e.item)
    }

    fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }
}

pub(super) enum AnyJobQueue<const N: usize> {
    Deque(DequeJobQueue<Ticket<N>>),
    Priority(PriorityJobQueue<N>),
}

impl<const N: usize> AnyJobQueue<N> {
    pub fn from_meta(vec: Vec<Ticket<N>>, meta: &JobMetadata<N>) -> Result<Self, SchedulerError> {
        if meta.priority.is_empty() {
            Ok(Self::Deque(vec.into()))
        } else {
            Ok(Self::Priority(PriorityJobQueue::new(
                vec,
                meta.priority,
                &meta.dims,
            )?))
        }
    }
}

impl<const N: usize> JobQueue<Ticket<N>> for AnyJobQueue<N> {
    fn push(&mut self, item: Ticket<N>) -> Result<(), SchedulerError> {
        match self {
            Self::Deque(q) => q.push(item),
            Self::Priority(q) => q.push(item),
        }
    }

    fn extend<I>(&mut self, iter: I) -> Result<(), SchedulerError>
    where
        I: IntoIterator<Item = Ticket<N>>,
    {
        match self {
            Self::Deque(q) => q.extend(iter),
            Self::Priority(q) => q.extend(iter),
        }
    }

    fn pop(&mut self) -> Option<Ticket<N>> {
        match self {
            Self::Deque(q) => q.pop(),
            Self::Priority(q) => q.pop(),
        }
    }

    fn is_empty(&self) -> bool {
        match self {
            Self::Deque(q) => q.is_empty(),
            Self::Priority(q) => q.is_empty(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{AnyJobQueue, JobQueue, PriorityJobQueue};
    use crate::schema::{Direction, JobMetadata, OptionCoordinate, Ticket};

    fn ticket<const N: usize>(coords: [usize; N]) -> Ticket<N> {
        let mut t = Ticket::new(0);
        for (i, c) in coords.into_iter().enumerate() {
            t.coordinate[i] = OptionCoordinate::some(c);
        }
        t
    }

    fn drain_coords<const N: usize>(q: &mut impl JobQueue<Ticket<N>>) -> Vec<[usize; N]> {
        std::iter::from_fn(|| q.pop())
            .map(|t| std::array::from_fn(|i| t.coordinate[i].0.unwrap()))
            .collect()
    }

    #[test]
    fn ascending_single_dim() {
        let mut q = PriorityJobQueue::<1>::new(
            vec![ticket([3]), ticket([1]), ticket([2])],
            &[("i", Direction::Ascending)],
            &["i"],
        )
        .unwrap();
        assert_eq!(drain_coords(&mut q), [[1], [2], [3]]);
    }

    #[test]
    fn descending_single_dim() {
        let mut q = PriorityJobQueue::<1>::new(
            vec![ticket([1]), ticket([3]), ticket([2])],
            &[("i", Direction::Descending)],
            &["i"],
        )
        .unwrap();
        assert_eq!(drain_coords(&mut q), [[3], [2], [1]]);
    }

    #[test]
    fn lexicographic_multi_dim() {
        let mut q = PriorityJobQueue::<2>::new(
            vec![ticket([2, 1]), ticket([1, 2]), ticket([1, 1])],
            &[("tier", Direction::Ascending), ("id", Direction::Ascending)],
            &["tier", "id"],
        )
        .unwrap();
        assert_eq!(drain_coords(&mut q), [[1, 1], [1, 2], [2, 1]]);
    }

    #[test]
    fn mixed_directions() {
        // tier asc, id desc: tier=1 before tier=2; within tier=1, higher id first
        let mut q = PriorityJobQueue::<2>::new(
            vec![ticket([1, 1]), ticket([1, 2]), ticket([2, 1])],
            &[
                ("tier", Direction::Ascending),
                ("id", Direction::Descending),
            ],
            &["tier", "id"],
        )
        .unwrap();
        assert_eq!(drain_coords(&mut q), [[1, 2], [1, 1], [2, 1]]);
    }

    #[test]
    fn fifo_tiebreak_within_equal_priority() {
        // Only `tier` is a priority dim; `id` is a passenger coordinate for identification.
        let mut q =
            PriorityJobQueue::<2>::new(vec![], &[("tier", Direction::Ascending)], &["tier", "id"])
                .unwrap();
        q.push(ticket([1, 10])).unwrap();
        q.push(ticket([1, 20])).unwrap();
        q.push(ticket([1, 30])).unwrap();
        // All same tier: pop in push order (FIFO)
        assert_eq!(drain_coords(&mut q), [[1, 10], [1, 20], [1, 30]]);
    }

    #[test]
    fn any_job_queue_dispatches_deque_for_empty_priority() {
        let meta = JobMetadata {
            id: "test",
            dims: ["i"],
            spawn_dim: None,
            priority: &[],
        };
        let q = AnyJobQueue::from_meta(vec![], &meta).unwrap();
        assert!(matches!(q, AnyJobQueue::Deque(_)));
    }

    #[test]
    fn any_job_queue_dispatches_priority_for_nonempty_priority() {
        let meta = JobMetadata {
            id: "test",
            dims: ["i"],
            spawn_dim: None,
            priority: &[("i", Direction::Ascending)],
        };
        let q = AnyJobQueue::from_meta(vec![], &meta).unwrap();
        assert!(matches!(q, AnyJobQueue::Priority(_)));
    }

    #[test]
    fn push_unresolved_priority_coordinate_errors() {
        let mut q =
            PriorityJobQueue::<1>::new(vec![], &[("i", Direction::Ascending)], &["i"]).unwrap();
        // coordinate[0] is None (unresolved)
        let unresolved = Ticket::new(0);
        assert!(q.push(unresolved).is_err());
    }
}
