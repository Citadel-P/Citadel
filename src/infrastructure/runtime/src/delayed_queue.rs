//! Keyed deadlines and a bounded FIFO of runnable work. The coordinator owns the queue.
use std::collections::{BTreeMap, BTreeSet, VecDeque};

pub struct DelayedQueue<K, D> {
    deadlines: BTreeSet<(D, K)>,
    by_key: BTreeMap<K, D>,
    ready: VecDeque<(K, D)>,
    // A replacement (including removal) received during dispatch wins over its result.
    running: BTreeMap<K, Option<Option<D>>>,
    capacity: usize,
}

impl<K: Ord + Copy, D: Ord + Copy> DelayedQueue<K, D> {
    pub fn new(ready_capacity: usize) -> Self {
        assert!(ready_capacity > 0);
        Self {
            deadlines: BTreeSet::new(),
            by_key: BTreeMap::new(),
            ready: VecDeque::new(),
            running: BTreeMap::new(),
            capacity: ready_capacity,
        }
    }

    pub fn replace(&mut self, key: K, deadline: Option<D>) {
        if let Some(pending) = self.running.get_mut(&key) {
            *pending = Some(deadline);
            return;
        }
        if let Some(previous) = self.by_key.remove(&key) {
            self.deadlines.remove(&(previous, key));
        }
        self.ready.retain(|(other, _)| *other != key);
        if let Some(at) = deadline {
            self.by_key.insert(key, at);
            self.deadlines.insert((at, key));
        }
    }

    /// Coalesce signals without moving an already-ready key behind newer work.
    pub fn wake(&mut self, key: K, now: D) {
        if self.by_key.get(&key).is_some_and(|at| *at <= now)
            || self.ready.iter().any(|(ready, _)| *ready == key)
        {
            return;
        }
        self.replace(key, Some(now));
    }

    pub fn promote(&mut self, now: D) {
        while self.ready.len() < self.capacity {
            let Some(&(at, key)) = self.deadlines.first() else {
                break;
            };
            if at > now {
                break;
            }
            self.deadlines.pop_first();
            self.by_key.remove(&key);
            self.ready.push_back((key, at));
        }
    }

    pub fn start(&mut self) -> Option<(K, D)> {
        let (key, at) = self.ready.pop_front()?;
        self.running.insert(key, None);
        Some((key, at))
    }

    pub fn finish(&mut self, key: K, next: Option<D>) {
        let pending = self
            .running
            .remove(&key)
            .expect("finishing an unclaimed schedule");
        self.replace(key, pending.unwrap_or(next));
    }

    pub fn contains(&self, key: K) -> bool {
        self.by_key.contains_key(&key)
            || self.running.contains_key(&key)
            || self.ready.iter().any(|(k, _)| *k == key)
    }

    pub fn next_deadline(&self) -> Option<D> {
        self.deadlines.first().map(|(at, _)| *at)
    }

    /// The waiting deadline, including a replacement received while the key is running.
    pub fn scheduled(&self, key: K) -> Option<D> {
        self.by_key
            .get(&key)
            .copied()
            .or_else(|| {
                self.ready
                    .iter()
                    .find(|(other, _)| *other == key)
                    .map(|(_, at)| *at)
            })
            .or_else(|| self.running.get(&key).copied().flatten().flatten())
    }

    pub fn running_len(&self) -> usize {
        self.running.len()
    }

    pub fn ready_len(&self) -> usize {
        self.ready.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn edits_during_dispatch_replace_stale_results_and_preserve_fairness() {
        let mut q = DelayedQueue::new(2);
        for i in 1..10000 {
            q.replace(1, Some(i));
        }
        assert_eq!(q.deadlines.len(), 1);
        for k in [1, 2, 3] {
            q.replace(k, Some(5));
        }
        q.promote(5);
        assert_eq!(q.ready_len(), 2);
        assert_eq!(q.start(), Some((1, 5)));
        q.replace(1, Some(8));
        q.replace(1, Some(6));
        q.promote(6);
        assert_eq!(q.start(), Some((2, 5)));
        q.replace(2, None);
        q.finish(2, Some(7));
        q.finish(1, Some(100));
        assert_eq!(q.next_deadline(), Some(6));
        q.promote(6);
        assert_eq!(q.start(), Some((3, 5)));
        assert_eq!(q.start(), Some((1, 6)));
        q.finish(3, None);
        q.finish(1, None);
        assert_eq!(q.next_deadline(), None);
    }
}
