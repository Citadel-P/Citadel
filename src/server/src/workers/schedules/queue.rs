//! A bounded ready FIFO per purpose keeps a large due family from hiding other work.
use super::planner::{Key, Time};
use chrono::{Duration as WallDuration, Utc};
use citadel_instance::SchedulePurpose;
use citadel_runtime::delayed_queue::DelayedQueue;
use std::{collections::BTreeMap, time::Duration};
use tokio::time::Instant;

const EXECUTIONS: usize = 10;
const RECONCILIATIONS: usize = 2;
const PER_PURPOSE: usize = 4;
const READY_PER_LANE: usize = 8;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Lane {
    Reconcile(Option<SchedulePurpose>),
    Execute(SchedulePurpose),
}
impl Key {
    fn lane(self) -> Lane {
        match self {
            Key::Eligibility => Lane::Reconcile(None),
            Key::Discover(p) | Key::Load(p, _) => Lane::Reconcile(Some(p)),
            Key::Calendar(p, _) | Key::Batch(p) => Lane::Execute(p),
        }
    }
}

#[derive(Clone, Copy)]
pub(super) struct Clock {
    pub wall: Time,
    pub elapsed: Instant,
}
impl Clock {
    pub fn now() -> Self {
        Self {
            wall: Utc::now(),
            elapsed: Instant::now(),
        }
    }
    pub fn project(self, at: Instant) -> Time {
        if at >= self.elapsed {
            self.wall + WallDuration::from_std(at - self.elapsed).expect("scheduler interval")
        } else {
            self.wall - WallDuration::from_std(self.elapsed - at).expect("scheduler interval")
        }
    }
}

#[derive(Clone, Copy)]
pub(super) enum Deadline {
    Wall(Time),
    Elapsed(Instant),
}
impl Deadline {
    pub fn after(seconds: u64) -> Self {
        Self::Elapsed(Instant::now() + Duration::from_secs(seconds))
    }
    fn entry(self, clock: Clock) -> Entry {
        match self {
            Self::Wall(at) => (
                clock.elapsed + (at - clock.wall).to_std().unwrap_or_default(),
                Some(at),
            ),
            Self::Elapsed(at) => (at, None),
        }
    }
}
// Keep the intended wall occurrence even when a slot opens after its minute.
type Entry = (Instant, Option<Time>);

#[derive(Default)]
pub(super) struct Queues {
    lanes: BTreeMap<Lane, DelayedQueue<Key, Entry>>,
    last: Option<Lane>,
}
impl Queues {
    fn lane(&mut self, key: Key) -> &mut DelayedQueue<Key, Entry> {
        self.lanes
            .entry(key.lane())
            .or_insert_with(|| DelayedQueue::new(READY_PER_LANE))
    }
    pub fn replace(&mut self, key: Key, next: Option<Deadline>) {
        self.lane(key)
            .replace(key, next.map(|d| d.entry(Clock::now())));
    }
    pub fn replace_wall(&mut self, key: Key, at: Option<Time>, clock: Clock) {
        self.lane(key)
            .replace(key, at.map(|t| Deadline::Wall(t).entry(clock)));
    }
    #[cfg(test)]
    pub fn scheduled(&self, key: Key) -> Option<Entry> {
        self.lanes.get(&key.lane())?.scheduled(key)
    }
    pub fn wake(&mut self, key: Key) {
        self.lane(key).wake(key, (Instant::now(), None));
    }
    pub fn finish(&mut self, key: Key, next: Option<Deadline>) {
        self.lane(key)
            .finish(key, next.map(|d| d.entry(Clock::now())));
    }
    pub fn contains(&self, key: Key) -> bool {
        self.lanes.get(&key.lane()).is_some_and(|q| q.contains(key))
    }
    pub fn rebase_wall(&mut self, key: Key, clock: Clock) {
        let q = self.lane(key);
        if let Some((_, Some(at))) = q.scheduled(key) {
            q.replace(key, Some(Deadline::Wall(at).entry(clock)));
        }
    }
    fn capacity(&self) -> (usize, usize) {
        let (mut reads, mut executions) = (0, 0);
        for (lane, queue) in &self.lanes {
            match lane {
                Lane::Reconcile(_) => reads += queue.running_len(),
                Lane::Execute(_) => executions += queue.running_len(),
            }
        }
        (RECONCILIATIONS - reads, EXECUTIONS - executions)
    }
    fn available(lane: Lane, running: usize, (reads, executions): (usize, usize)) -> bool {
        match lane {
            Lane::Reconcile(_) => reads > 0 && running == 0,
            Lane::Execute(_) => executions > 0 && running < PER_PURPOSE,
        }
    }
    pub fn start(&mut self, now: Instant) -> Option<(Key, Option<Time>)> {
        let capacity = self.capacity();
        let mut first = None;
        let mut after = None;
        for (&lane, q) in &mut self.lanes {
            if !Self::available(lane, q.running_len(), capacity) {
                continue;
            }
            q.promote((now, Some(Time::MAX_UTC)));
            if q.ready_len() == 0 {
                continue;
            }
            first.get_or_insert(lane);
            if Some(lane) > self.last {
                after = Some(lane);
                break;
            }
        }
        let lane = after.or(first)?;
        self.last = Some(lane);
        self.lanes
            .get_mut(&lane)?
            .start()
            .map(|(key, (_, due))| (key, due))
    }
    pub fn next_deadline(&self) -> Option<Instant> {
        let capacity = self.capacity();
        self.lanes
            .iter()
            .filter(|(lane, q)| Self::available(**lane, q.running_len(), capacity))
            .filter_map(|(_, q)| q.next_deadline().map(|(at, _)| at))
            .min()
    }
    pub fn backlog(&self) -> (u64, bool) {
        (
            self.lanes.values().map(|q| q.ready_len() as u64).sum(),
            self.lanes.values().any(|q| q.ready_len() == READY_PER_LANE),
        )
    }
}
