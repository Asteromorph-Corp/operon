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
