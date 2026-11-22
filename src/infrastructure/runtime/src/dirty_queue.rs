//! Bounded reconciliation requests. An active key retains its slot until completion.
use crate::QueueSendError;
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::{sync::Notify, time::Instant};
use tokio_util::sync::CancellationToken;

struct Entry {
    generation: u64,
    due: Option<Instant>,
    retry: bool,
}
struct State<K> {
    entries: BTreeMap<K, Entry>,
    closed: bool,
}
struct Shared<K> {
    state: Mutex<State<K>>,
    changed: Notify,
    capacity: usize,
    debounce: Duration,
}
#[derive(Clone)]
pub struct DirtySender<K>(Arc<Shared<K>>);
pub struct DirtyReceiver<K>(Arc<Shared<K>>);

pub fn channel<K>(capacity: usize, debounce: Duration) -> (DirtySender<K>, DirtyReceiver<K>) {
    let shared = Arc::new(Shared {
        state: Mutex::new(State {
            entries: BTreeMap::new(),
            closed: false,
        }),
        changed: Notify::new(),
        capacity: capacity.max(1),
        debounce,
    });
    (DirtySender(shared.clone()), DirtyReceiver(shared))
}
impl<K: Ord + Copy> DirtySender<K> {
    pub async fn mark(&self, key: K, cancel: &CancellationToken) -> Result<(), QueueSendError<K>> {
        loop {
            let changed = self.0.changed.notified();
            tokio::pin!(changed);
            changed.as_mut().enable();
            {
                let mut state = self.0.state.lock().unwrap();
                if cancel.is_cancelled() {
                    return Err(QueueSendError::Cancelled(key));
                }
                if state.closed {
                    return Err(QueueSendError::Closed(key));
                }
                if let Some(entry) = state.entries.get_mut(&key) {
                    entry.generation = entry.generation.wrapping_add(1);
                    return Ok(());
                }
                if state.entries.len() < self.0.capacity {
                    state.entries.insert(
                        key,
                        Entry {
                            generation: 0,
                            due: Some(Instant::now() + self.0.debounce),
                            retry: false,
                        },
                    );
                    self.0.changed.notify_waiters();
                    return Ok(());
                }
            }
            tokio::select! {
                () = cancel.cancelled() => return Err(QueueSendError::Cancelled(key)),
                () = changed => {},
            }
        }
    }
    pub fn occupancy(&self) -> usize {
        self.0.state.lock().unwrap().entries.len()
    }
}
#[derive(Clone, Copy)]
pub struct DirtyWork<K> {
    pub key: K,
    generation: u64,
    pub retry: bool,
}
impl<K: Ord + Copy> DirtyReceiver<K> {
    pub async fn next(&self) -> DirtyWork<K> {
        loop {
            let changed = self.0.changed.notified();
            tokio::pin!(changed);
            changed.as_mut().enable();
            let deadline = {
                let mut state = self.0.state.lock().unwrap();
                let first = state
                    .entries
                    .iter_mut()
                    .filter_map(|(key, entry)| Some((entry.due?, *key, entry)))
                    .min_by_key(|(due, _, _)| *due);
                match first {
                    Some((due, key, entry)) if due <= Instant::now() => {
                        entry.due = None;
                        return DirtyWork {
                            key,
                            generation: entry.generation,
                            retry: entry.retry,
                        };
                    }
                    Some((due, _, _)) => Some(due),
                    None => None,
                }
            };
            match deadline {
                Some(at) => {
                    tokio::select! { () = tokio::time::sleep_until(at) => {}, () = changed => {} }
                }
                None => changed.await,
            }
        }
    }
    pub fn current(&self, work: DirtyWork<K>) -> bool {
        self.0
            .state
            .lock()
            .unwrap()
            .entries
            .get(&work.key)
            .is_some_and(|entry| entry.generation == work.generation)
    }
    /// A successful commit raced by another mark still leaves one follow-up.
    pub fn finish(&self, work: DirtyWork<K>, committed: bool, retry: Option<Duration>) -> bool {
        let mut state = self.0.state.lock().unwrap();
        let entry = state
            .entries
            .get_mut(&work.key)
            .expect("active key retains its slot");
        let complete = committed && entry.generation == work.generation;
        if complete {
            state.entries.remove(&work.key);
        } else {
            entry.retry = retry.is_some();
            entry.due = Some(Instant::now() + retry.unwrap_or(self.0.debounce));
        }
        self.0.changed.notify_waiters();
        complete
    }
}
impl<K> Drop for DirtyReceiver<K> {
    fn drop(&mut self) {
        let mut state = self.0.state.lock().unwrap();
        state.closed = true;
        state.entries.clear();
        self.0.changed.notify_waiters();
    }
}
