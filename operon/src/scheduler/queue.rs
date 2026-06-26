use std::cmp::Ordering;
use std::collections::{BinaryHeap, VecDeque};

use crate::schema::{JobMetadata, Ticket};

use super::SchedulerError;

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

struct PriorityEntry<T> {
    key: Box<[i64]>,
    seq: u64,
    item: T,
}
impl<T> Eq for PriorityEntry<T> {}
impl<T> PartialEq for PriorityEntry<T> {
    fn eq(&self, other: &Self) -> bool {
        self.seq == other.seq
    }
}
impl<T> PartialOrd for PriorityEntry<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl<T> Ord for PriorityEntry<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.key
            .cmp(&other.key)
            .then_with(|| (other.seq.wrapping_sub(self.seq) as i64).cmp(&0))
    }
}

pub(super) struct PriorityJobQueue<const N: usize> {
    queue: BinaryHeap<PriorityEntry<Ticket<N>>>,
    next_seq: u64,
    priority_indices: Box<[(usize, bool)]>,
}

impl<const N: usize> PriorityJobQueue<N> {
    fn new(
        vec: Vec<Ticket<N>>,
        priority: &[(&'static str, bool)],
        dims: &[&'static str; N],
    ) -> Result<Self, SchedulerError> {
        let priority_indices = priority
            .iter()
            .map(|&(dim, desc)| {
                dims.iter()
                    .position(|d| *d == dim)
                    .map(|idx| (idx, desc))
                    .ok_or_else(|| {
                        SchedulerError::other(format!(
                            "Priority dimension `{dim}` not found in job dimensions"
                        ))
                    })
            })
            .collect::<Result<Box<[_]>, _>>()?;
        let mut queue = Self {
            queue: BinaryHeap::with_capacity(vec.len()),
            next_seq: 0,
            priority_indices,
        };
        queue.extend(vec)?;
        Ok(queue)
    }

    fn make_key(&self, ticket: &Ticket<N>) -> Result<Box<[i64]>, SchedulerError> {
        self.priority_indices
            .iter()
            .map(|&(idx, desc)| {
                ticket.coordinate[idx]
                    .0
                    .map(|val| if desc { val as i64 } else { -(val as i64) })
                    .ok_or_else(|| {
                        SchedulerError::other(format!(
                            "Unresolved coordinate at index {idx} pushed to priority job queue"
                        ))
                    })
            })
            .collect()
    }
}

impl<const N: usize> JobQueue<Ticket<N>> for PriorityJobQueue<N> {
    fn push(&mut self, item: Ticket<N>) -> Result<(), SchedulerError> {
        let key = self.make_key(&item)?;
        let seq = self.next_seq;
        self.next_seq = self.next_seq.wrapping_add(1);
        self.queue.push(PriorityEntry { key, seq, item });
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
    pub fn from_meta(
        vec: Vec<Ticket<N>>,
        meta: &JobMetadata<N>,
    ) -> Result<Self, SchedulerError> {
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
