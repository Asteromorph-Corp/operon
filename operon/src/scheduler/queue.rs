use std::cmp::Ordering;
use std::collections::{BinaryHeap, VecDeque};

use crate::schema::{Direction, Job, JobMetadata};

pub(super) trait JobQueue<T> {
    fn push(&mut self, item: T);
    fn extend<I>(&mut self, iter: I)
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
    fn push(&mut self, item: T) {
        self.0.push_back(item);
    }

    fn extend<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = T>,
    {
        self.0.extend(iter);
    }

    fn pop(&mut self) -> Option<T> {
        self.0.pop_front()
    }

    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

struct PriorityEntry<const N: usize> {
    order: &'static [(usize, Direction)],
    seq: u64,
    item: Job<N>,
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
            let a = self.item.coordinate[idx];
            let b = other.item.coordinate[idx];
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
    order: &'static [(usize, Direction)],
}

impl<const N: usize> PriorityJobQueue<N> {
    fn new(
        vec: Vec<Job<N>>,
        priority: &[(&'static str, Direction)],
        dims: &[&'static str; N],
    ) -> Self {
        let order: &'static [(usize, Direction)] = priority
            .iter()
            .map(|&(dim, dir)| {
                dims.iter()
                    .position(|d| *d == dim)
                    .map(|idx| (idx, dir))
                    // Validated at macro-expansion time
                    .expect("priority dimension not found in job dims")
            })
            .collect::<Vec<_>>()
            .leak();
        let mut queue = Self {
            queue: BinaryHeap::with_capacity(vec.len()),
            next_seq: 0,
            order,
        };
        queue.extend(vec);
        queue
    }
}

impl<const N: usize> JobQueue<Job<N>> for PriorityJobQueue<N> {
    fn push(&mut self, item: Job<N>) {
        let seq = self.next_seq;
        self.next_seq = self.next_seq.wrapping_add(1);
        self.queue.push(PriorityEntry {
            order: self.order,
            seq,
            item,
        });
    }

    fn extend<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = Job<N>>,
    {
        for item in iter {
            self.push(item);
        }
    }

    fn pop(&mut self) -> Option<Job<N>> {
        self.queue.pop().map(|e| e.item)
    }

    fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }
}

pub(super) enum AnyJobQueue<const N: usize> {
    Deque(DequeJobQueue<Job<N>>),
    Priority(PriorityJobQueue<N>),
}

impl<const N: usize> AnyJobQueue<N> {
    pub fn from_meta(vec: Vec<Job<N>>, meta: &JobMetadata<N>) -> Self {
        if meta.priority.is_empty() {
            Self::Deque(vec.into())
        } else {
            Self::Priority(PriorityJobQueue::new(vec, meta.priority, &meta.dims))
        }
    }
}

impl<const N: usize> JobQueue<Job<N>> for AnyJobQueue<N> {
    fn push(&mut self, item: Job<N>) {
        match self {
            Self::Deque(q) => q.push(item),
            Self::Priority(q) => q.push(item),
        }
    }

    fn extend<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = Job<N>>,
    {
        match self {
            Self::Deque(q) => q.extend(iter),
            Self::Priority(q) => q.extend(iter),
        }
    }

    fn pop(&mut self) -> Option<Job<N>> {
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
    use crate::schema::{Direction, Job, JobMetadata};

    fn job<const N: usize>(coords: [usize; N]) -> Job<N> {
        Job { coordinate: coords }
    }

    fn drain_coords<const N: usize>(q: &mut impl JobQueue<Job<N>>) -> Vec<[usize; N]> {
        std::iter::from_fn(|| q.pop())
            .map(|t| std::array::from_fn(|i| t.coordinate[i]))
            .collect()
    }

    #[test]
    fn ascending_single_dim() {
        let mut q = PriorityJobQueue::<1>::new(
            vec![job([3]), job([1]), job([2])],
            &[("i", Direction::Ascending)],
            &["i"],
        );
        assert_eq!(drain_coords(&mut q), [[1], [2], [3]]);
    }

    #[test]
    fn descending_single_dim() {
        let mut q = PriorityJobQueue::<1>::new(
            vec![job([1]), job([3]), job([2])],
            &[("i", Direction::Descending)],
            &["i"],
        );
        assert_eq!(drain_coords(&mut q), [[3], [2], [1]]);
    }

    #[test]
    fn lexicographic_multi_dim() {
        let mut q = PriorityJobQueue::<2>::new(
            vec![job([2, 1]), job([1, 2]), job([1, 1])],
            &[("tier", Direction::Ascending), ("id", Direction::Ascending)],
            &["tier", "id"],
        );
        assert_eq!(drain_coords(&mut q), [[1, 1], [1, 2], [2, 1]]);
    }

    #[test]
    fn mixed_directions() {
        // tier asc, id desc: tier=1 before tier=2; within tier=1, higher id first
        let mut q = PriorityJobQueue::<2>::new(
            vec![job([1, 1]), job([1, 2]), job([2, 1])],
            &[
                ("tier", Direction::Ascending),
                ("id", Direction::Descending),
            ],
            &["tier", "id"],
        );
        assert_eq!(drain_coords(&mut q), [[1, 2], [1, 1], [2, 1]]);
    }

    #[test]
    fn fifo_tiebreak_within_equal_priority() {
        // Only `tier` is a priority dim; `id` is a passenger coordinate for identification.
        let mut q =
            PriorityJobQueue::<2>::new(vec![], &[("tier", Direction::Ascending)], &["tier", "id"]);
        q.push(job([1, 10]));
        q.push(job([1, 20]));
        q.push(job([1, 30]));
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
        let q = AnyJobQueue::from_meta(vec![], &meta);
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
        let q = AnyJobQueue::from_meta(vec![], &meta);
        assert!(matches!(q, AnyJobQueue::Priority(_)));
    }
}
