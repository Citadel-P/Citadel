use super::{
    handlers::Dispatch,
    queue::{Clock, Deadline, Queues},
};
use chrono::{DateTime, Duration, Utc};
use citadel_instance::{
    ScheduleDefinition, ScheduleMode, SchedulePurpose as Purpose, calendar_deadline,
};
use citadel_primitives::schedule::CronSchedule;
use citadel_runtime::notifications::RuntimeNotification;
use std::time::Duration as Elapsed;
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};
use tokio::time::Instant;
use uuid::Uuid;
pub(super) type Time = DateTime<Utc>;
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum Key {
    Load(Purpose, Uuid),
    Calendar(Purpose, Uuid),
    Discover(Purpose),
    Batch(Purpose),
    Eligibility,
    GitRef(Uuid),
}
struct Calendar {
    expression: String,
    zone: String,
    parsed: Arc<CronSchedule>,
}
pub(super) const PAGE: i64 = 128;

fn purposes(mode: ScheduleMode) -> impl Iterator<Item = Purpose> {
    Purpose::ALL.into_iter().filter(move |p| p.mode() == mode)
}
pub(super) fn later(seconds: i64) -> Time {
    Utc::now() + Duration::seconds(seconds)
}

fn configure(
    q: &mut Queues,
    cal: &mut BTreeMap<(Purpose, Uuid), Calendar>,
    p: Purpose,
    id: Uuid,
    def: Option<ScheduleDefinition>,
) {
    let key = (p, id);
    let next = def
        .filter(|d| d.enabled && d.blocker.is_none())
        .and_then(|d| {
            let (cron, zone) = (d.cron?, d.zone?);
            if cal
                .get(&key)
                .is_none_or(|c| c.expression != cron || c.zone != zone)
            {
                let parsed = Arc::new(CronSchedule::parse(&cron, &zone).ok()?);
                cal.insert(
                    key,
                    Calendar {
                        expression: cron,
                        zone,
                        parsed,
                    },
                );
            }
            calendar_deadline(&cal[&key].parsed, d.anchor, Utc::now())
        });
    if next.is_none() {
        cal.remove(&key);
    }
    q.replace(Key::Calendar(p, id), next.map(Deadline::Wall));
}

pub(super) enum Output {
    Next(Option<Time>),
    Definition(u64, Option<ScheduleDefinition>),
    BatchFinished,
    Eligibility,
    Deadline(Option<Time>),
    Page(u64, Vec<ScheduleDefinition>),
}

pub(super) struct Planner {
    pub queue: Queues,
    calendar: BTreeMap<(Purpose, Uuid), Calendar>,
    generation: u64,
    pages: BTreeMap<Purpose, Uuid>,
    stale: BTreeMap<Purpose, BTreeSet<Uuid>>,
    edited: BTreeSet<(Purpose, Uuid)>,
    pending_reads: BTreeSet<(Purpose, Uuid)>,
    overflow: bool,
    fixed: BTreeMap<Purpose, Instant>,
    pending_refs: BTreeSet<Uuid>,
    first_batch: Instant,
}
impl Planner {
    pub fn new(h: &impl Dispatch) -> Self {
        Self::new_at(h, Clock::now())
    }

    pub(super) fn new_at(h: &impl Dispatch, clock: Clock) -> Self {
        let mut queue = Queues::default();
        queue.wake(Key::Eligibility);
        for p in purposes(ScheduleMode::PersistedDeadline) {
            queue.wake(Key::Discover(p));
        }
        let mut fixed = BTreeMap::new();
        for p in Purpose::ALL {
            if let ScheduleMode::Fixed {
                seconds,
                startup_label,
            } = p.mode()
            {
                let at = clock.elapsed
                    + Elapsed::from_secs(seconds as u64)
                    + super::super::schedule::startup_offset(startup_label);
                fixed.insert(p, at);
                queue.replace(Key::Batch(p), Some(Deadline::Elapsed(at)));
                h.set_deadline(p, clock.project(at));
            }
        }
        Self {
            pending_refs: Default::default(),
            queue,
            calendar: Default::default(),
            generation: 0,
            pages: Default::default(),
            stale: Default::default(),
            edited: Default::default(),
            pending_reads: Default::default(),
            overflow: false,
            fixed,
            first_batch: clock.elapsed + Elapsed::from_secs(60),
        }
    }

    pub fn clock_changed(&mut self, clock: Clock, h: &impl Dispatch) {
        for (&p, &at) in &self.fixed {
            h.set_deadline(p, clock.project(at));
        }
        for p in purposes(ScheduleMode::PersistedDeadline) {
            self.queue.rebase_wall(Key::Batch(p), clock);
        }
        for (&(p, id), c) in &self.calendar {
            self.queue.replace_wall(
                Key::Calendar(p, id),
                calendar_deadline(&c.parsed, None, clock.wall),
                clock,
            );
        }
        self.queue.wake(Key::Eligibility);
    }

    fn rebuild(&mut self) {
        self.generation = self.generation.wrapping_add(1);
        self.pages.clear();
        self.edited.clear();
        self.stale.clear();
        for &(p, id) in self.calendar.keys() {
            self.stale.entry(p).or_default().insert(id);
        }
        for p in purposes(ScheduleMode::Calendar) {
            self.stale.entry(p).or_default();
            self.queue.wake(Key::Discover(p));
        }
        for p in purposes(ScheduleMode::PersistedDeadline) {
            self.queue.wake(Key::Discover(p));
        }
    }

    pub fn notification(&mut self, msg: RuntimeNotification, h: &impl Dispatch) {
        h.notify();
        let RuntimeNotification::Schedule(payload) = msg else {
            for p in purposes(ScheduleMode::PersistedDeadline) {
                self.queue.wake(Key::Discover(p));
            }
            self.queue.wake(Key::Eligibility);
            return;
        };
        let (name, id) = payload
            .split_once(':')
            .map_or((payload.as_str(), None), |(name, id)| {
                (name, Uuid::parse_str(id).ok())
            });
        let Some(p) = Purpose::parse(if name == "ReleasePolicy" {
            "Release"
        } else {
            name
        }) else {
            return;
        };
        if p.mode() == ScheduleMode::Calendar {
            let id = id.unwrap_or(Uuid::nil());
            if self.pending_reads.len() >= 256 && !self.pending_reads.contains(&(p, id)) {
                self.overflow = true;
                return;
            }
            if self.stale.contains_key(&p) {
                self.edited.insert((p, id));
            }
            self.pending_reads.insert((p, id));
            self.queue.wake(Key::Load(p, id));
        } else if p.mode() == ScheduleMode::PersistedDeadline {
            self.queue.wake(Key::Discover(p));
        }
    }

    pub fn wake_ref(&mut self, reference: Uuid) {
        if self.pending_refs.len() < 128 || self.pending_refs.contains(&reference) {
            self.pending_refs.insert(reference);
            self.queue.wake(Key::GitRef(reference));
        } else {
            self.queue.wake(Key::Batch(Purpose::StackGit));
        }
    }
    pub fn request(&self, key: Key) -> super::coordinator::Request {
        let recurrence = match key {
            Key::Calendar(p, id) => self.calendar.get(&(p, id)).map(|c| c.parsed.clone()),
            _ => None,
        };
        let cursor = match key {
            Key::Discover(p) => self.pages.get(&p).copied(),
            _ => None,
        };
        super::coordinator::Request {
            key,
            recurrence,
            cursor,
            generation: self.generation,
        }
    }

    fn page(&mut self, p: Purpose, epoch: u64, defs: Vec<ScheduleDefinition>) -> Option<Deadline> {
        if epoch != self.generation {
            return Some(Deadline::after(0));
        }
        let last = defs.last().map(|d| d.owner);
        let more = defs.len() == PAGE as usize;
        for def in defs {
            let id = def.owner;
            if let Some(stale) = self.stale.get_mut(&p) {
                stale.remove(&id);
            }
            if !self.edited.contains(&(p, id)) {
                configure(&mut self.queue, &mut self.calendar, p, id, Some(def));
            }
        }
        if more {
            self.pages.insert(p, last.expect("full schedule page"));
            return Some(Deadline::after(0));
        }
        self.pages.remove(&p);
        self.edited.retain(|(kind, _)| *kind != p);
        for id in self.stale.remove(&p).unwrap_or_default() {
            configure(&mut self.queue, &mut self.calendar, p, id, None);
        }
        None
    }

    pub fn finish(&mut self, key: Key, result: Result<Output, String>, h: &impl Dispatch) {
        let next = match (key, result) {
            (Key::Batch(p), outcome) if self.fixed.contains_key(&p) => {
                if let Err(error) = outcome {
                    tracing::warn!(%error, ?p, "Scheduled check failed");
                }
                // A failed sweep keeps the feature's normal cadence. Events can
                // still wake Stack Git reconciliation before its next fallback.
                let period = p.fixed_interval().expect("fixed schedule period");
                let anchor = self.fixed.get_mut(&p).expect("fixed schedule anchor");
                let clock = Clock::now();
                if *anchor <= clock.elapsed {
                    let periods = (clock.elapsed - *anchor).as_secs() / period as u64 + 1;
                    *anchor += Elapsed::from_secs(periods * period as u64);
                }
                h.set_deadline(p, clock.project(*anchor));
                Some(Deadline::Elapsed(*anchor))
            }
            (Key::GitRef(_), outcome) => {
                if let Err(error) = outcome {
                    tracing::warn!(%error,"Stack Git check failed; the scheduled fallback will reconcile");
                }
                None
            }
            (_, Err(e)) => {
                tracing::warn!(%e,?key,"Schedule reconciliation will retry");
                Some(Deadline::after(match key {
                    Key::Batch(Purpose::Git | Purpose::Image) => 60,
                    Key::Batch(Purpose::Release) => 30,
                    _ => 10,
                }))
            }
            (Key::Load(..), Ok(Output::Definition(epoch, _))) if epoch != self.generation => {
                Some(Deadline::after(0))
            }
            (Key::Load(p, id), Ok(Output::Definition(_, def))) => {
                if let Some(stale) = self.stale.get_mut(&p) {
                    stale.remove(&id);
                }
                configure(&mut self.queue, &mut self.calendar, p, id, def);
                None
            }
            (Key::Discover(p), Ok(Output::Page(epoch, defs))) => self.page(p, epoch, defs),
            (Key::Discover(p), Ok(Output::Deadline(due))) => {
                let due = due.map(|at| {
                    if matches!(p, Purpose::Git | Purpose::Image)
                        && Instant::now() < self.first_batch
                        && at <= Clock::now().project(self.first_batch)
                    {
                        Deadline::Elapsed(self.first_batch)
                    } else {
                        Deadline::Wall(at)
                    }
                });
                self.queue.replace(Key::Batch(p), due);
                None
            }
            (Key::Eligibility, Ok(Output::Eligibility)) => {
                self.rebuild();
                Some(Deadline::after(15 * 60))
            }
            (Key::Batch(p), Ok(Output::BatchFinished)) => {
                self.queue
                    .replace(Key::Discover(p), Some(Deadline::after(2)));
                h.notify();
                None
            }
            (Key::Calendar(..), Ok(Output::Next(next))) => next.map(Deadline::Wall),
            _ => unreachable!("reconciliation result must match its request"),
        };
        self.queue.finish(key, next);
        if let Key::GitRef(id) = key
            && !self.queue.contains(key)
        {
            self.pending_refs.remove(&id);
        }
        if let Key::Load(p, id) = key
            && !self.queue.contains(key)
        {
            self.pending_reads.remove(&(p, id));
        }
        if self.overflow && self.pending_reads.len() < 128 {
            self.overflow = false;
            self.queue.wake(Key::Eligibility);
        }
    }
}
