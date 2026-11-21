use super::handlers::Dispatch;
use chrono::{DateTime, Duration, Utc};
use citadel_instance::{ScheduleDefinition, SchedulePurpose as Purpose, calendar_deadline};
use citadel_primitives::schedule::CronSchedule;
use citadel_runtime::{delayed_queue::DelayedQueue, notifications::RuntimeNotification};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};
use uuid::Uuid;
pub(super) type Time = DateTime<Utc>;
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum Key {
    Load(Purpose, Uuid),
    Calendar(Purpose, Uuid),
    Discover(Purpose),
    Batch(Purpose),
    Eligibility,
}
struct Calendar {
    expression: String,
    zone: String,
    parsed: Arc<CronSchedule>,
}
type Queue = DelayedQueue<Key, Time>;
pub(super) const PAGE: i64 = 128;

pub(super) const CALENDARS: [Purpose; 3] = [Purpose::Script, Purpose::Backup, Purpose::Install];
const DYNAMIC: [Purpose; 3] = [Purpose::Release, Purpose::Git, Purpose::Image];
pub(super) fn later(seconds: i64) -> Time {
    Utc::now() + Duration::seconds(seconds)
}

fn configure(
    q: &mut Queue,
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
    q.replace(Key::Calendar(p, id), next);
}

pub(super) enum Output {
    Next(Option<Time>),
    Definition(u64, Option<ScheduleDefinition>),
    BatchFinished,
    Deadline(Option<Time>),
    Page(u64, Vec<ScheduleDefinition>),
}

pub(super) struct Planner {
    pub queue: Queue,
    calendar: BTreeMap<(Purpose, Uuid), Calendar>,
    generation: u64,
    pages: BTreeMap<Purpose, Uuid>,
    stale: BTreeMap<Purpose, BTreeSet<Uuid>>,
    edited: BTreeSet<(Purpose, Uuid)>,
    pending_reads: BTreeSet<(Purpose, Uuid)>,
    overflow: bool,
    fixed: BTreeMap<Purpose, Time>,
    first_batch: Time,
}
impl Planner {
    pub fn new(h: &impl Dispatch) -> Self {
        let mut queue = Queue::new(128);
        queue.wake(Key::Eligibility, Utc::now());
        for p in DYNAMIC {
            queue.wake(Key::Discover(p), Utc::now());
        }
        let mut fixed = BTreeMap::new();
        for p in Purpose::ALL {
            if let Some(seconds) = p.fixed_interval() {
                let label = match p {
                    Purpose::DeploymentImages => "deployment-image-updates",
                    Purpose::ServiceImages => "service-image-updates",
                    Purpose::StackGit | Purpose::StackImages => "stack-updates",
                    _ => unreachable!(),
                };
                let at = later(seconds)
                    + Duration::from_std(super::super::schedule::startup_offset(label))
                        .unwrap_or_default();
                fixed.insert(p, at);
                queue.replace(Key::Batch(p), Some(at));
                h.set_deadline(p, at);
            }
        }
        Self {
            queue,
            calendar: Default::default(),
            generation: 0,
            pages: Default::default(),
            stale: Default::default(),
            edited: Default::default(),
            pending_reads: Default::default(),
            overflow: false,
            fixed,
            first_batch: later(60),
        }
    }

    pub fn clock_changed(&mut self, wall: Time) {
        for (&(p, id), c) in &self.calendar {
            self.queue.replace(
                Key::Calendar(p, id),
                calendar_deadline(&c.parsed, None, wall),
            );
        }
        self.queue.wake(Key::Eligibility, wall);
    }

    fn rebuild(&mut self) {
        self.generation = self.generation.wrapping_add(1);
        self.pages.clear();
        self.edited.clear();
        self.stale.clear();
        for &(p, id) in self.calendar.keys() {
            self.stale.entry(p).or_default().insert(id);
        }
        for p in CALENDARS {
            self.stale.entry(p).or_default();
            self.queue.wake(Key::Discover(p), Utc::now());
        }
        for p in DYNAMIC {
            self.queue.wake(Key::Discover(p), Utc::now());
        }
    }

    pub fn notification(&mut self, msg: RuntimeNotification, h: &impl Dispatch) {
        h.notify();
        let RuntimeNotification::Schedule(payload) = msg else {
            for p in DYNAMIC {
                self.queue.wake(Key::Discover(p), Utc::now());
            }
            self.queue.wake(Key::Eligibility, Utc::now());
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
        if CALENDARS.contains(&p) {
            let id = id.unwrap_or(Uuid::nil());
            if self.pending_reads.len() >= 256 && !self.pending_reads.contains(&(p, id)) {
                self.overflow = true;
                return;
            }
            if self.stale.contains_key(&p) {
                self.edited.insert((p, id));
            }
            self.pending_reads.insert((p, id));
            self.queue.wake(Key::Load(p, id), Utc::now());
        } else if DYNAMIC.contains(&p) {
            self.queue.wake(Key::Discover(p), Utc::now());
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

    fn page(&mut self, p: Purpose, epoch: u64, defs: Vec<ScheduleDefinition>) -> Option<Time> {
        if epoch != self.generation {
            return Some(Utc::now());
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
            return Some(Utc::now());
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
                let now = Utc::now();
                if *anchor <= now {
                    *anchor +=
                        Duration::seconds(((now - *anchor).num_seconds() / period + 1) * period);
                }
                h.set_deadline(p, *anchor);
                Some(*anchor)
            }
            (_, Err(e)) => {
                tracing::warn!(%e,?key,"Schedule reconciliation will retry");
                Some(later(match key {
                    Key::Batch(Purpose::Git | Purpose::Image) => 60,
                    Key::Batch(Purpose::Release) => 30,
                    _ => 10,
                }))
            }
            (Key::Load(..), Ok(Output::Definition(epoch, _))) if epoch != self.generation => {
                Some(Utc::now())
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
                let due = if matches!(p, Purpose::Git | Purpose::Image) {
                    due.map(|t| t.max(self.first_batch))
                } else {
                    due
                };
                self.queue.replace(Key::Batch(p), due);
                None
            }
            (Key::Eligibility, Ok(Output::Next(next))) => {
                self.rebuild();
                next
            }
            (Key::Batch(p), Ok(Output::BatchFinished)) => {
                self.queue.replace(Key::Discover(p), Some(later(2)));
                h.notify();
                None
            }
            (Key::Calendar(..), Ok(Output::Next(next))) => next,
            _ => unreachable!("reconciliation result must match its request"),
        };
        self.queue.finish(key, next);
        if let Key::Load(p, id) = key
            && !self.queue.contains(key)
        {
            self.pending_reads.remove(&(p, id));
        }
        if self.overflow && self.pending_reads.len() < 128 {
            self.overflow = false;
            self.queue.wake(Key::Eligibility, Utc::now());
        }
    }
}
