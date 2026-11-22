use super::{Dispatch, drive};
use chrono::{DateTime, Duration, Utc};
use citadel_instance::{
    InstanceError, ScheduleDefinition, SchedulePurpose as Purpose, ScheduleReader,
};
use citadel_runtime::notifications::{NotificationError, NotificationSource, RuntimeNotification};
use futures_util::future::BoxFuture;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};
use tokio::sync::{Notify, mpsc, watch};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

struct Signal(mpsc::Receiver<RuntimeNotification>);
impl NotificationSource for Signal {
    fn recv(&mut self) -> BoxFuture<'_, Result<RuntimeNotification, NotificationError>> {
        Box::pin(async {
            match self.0.recv().await {
                Some(v) => Ok(v),
                None => std::future::pending().await,
            }
        })
    }
}
struct Store {
    scripts: usize,
    backups: usize,
    accepted: Mutex<Vec<(Purpose, Uuid, DateTime<Utc>)>>,
    active: AtomicUsize,
    hold_only_scripts: AtomicBool,
    reads: Mutex<Vec<(Purpose, Uuid)>>,
    last_script_page: Notify,
    projections: Mutex<std::collections::BTreeMap<Purpose, DateTime<Utc>>>,
    maximum: AtomicUsize,
    pages: AtomicUsize,
    eligible: AtomicBool,
    blocked_script: Notify,
    wake: Notify,
    release: tokio::sync::Semaphore,
}
impl Store {
    fn new(scripts: usize, backups: usize) -> Arc<Self> {
        Arc::new(Self {
            scripts,
            backups,
            accepted: Default::default(),
            active: Default::default(),
            hold_only_scripts: Default::default(),
            reads: Default::default(),
            last_script_page: Default::default(),
            projections: Default::default(),
            maximum: Default::default(),
            pages: Default::default(),
            eligible: AtomicBool::new(true),
            blocked_script: Default::default(),
            wake: Default::default(),
            release: tokio::sync::Semaphore::new(0),
        })
    }
    fn definition(p: Purpose, id: Uuid) -> ScheduleDefinition {
        ScheduleDefinition {
            resource_status: None,
            needs_attention: false,
            latest_run: None,
            active_run_status: None,
            purpose: p,
            owner: id,
            name: "fixture".into(),
            enabled: true,
            script_timeout_seconds: None,
            blocker: None,
            platform: None,
            docker_image_id: None,
            cron: Some("* * * * *".into()),
            zone: Some("UTC".into()),
            seconds: None,
            anchor: None,
        }
    }
}
impl ScheduleReader for Store {
    fn counts<'a>(
        &'a self,
        _: &'a str,
        _: &'a citadel_instance::ScheduleCatalogPolicy,
    ) -> BoxFuture<'a, Result<Vec<citadel_instance::ScheduleCount>, InstanceError>> {
        unreachable!("workers do not read catalog counts")
    }
    fn page<'a>(
        &'a self,
        _: &'a [Purpose],
        _: &'a str,
        _: i64,
        _: i64,
        _: &'a citadel_instance::ScheduleCatalogPolicy,
    ) -> BoxFuture<'a, Result<Vec<ScheduleDefinition>, InstanceError>> {
        unreachable!("workers do not read numbered catalog pages")
    }
    fn discover(
        &self,
        p: Purpose,
        after: Option<Uuid>,
        limit: i64,
    ) -> BoxFuture<'_, Result<Vec<ScheduleDefinition>, InstanceError>> {
        Box::pin(async move {
            self.pages.fetch_add(1, Ordering::SeqCst);
            let count = match p {
                Purpose::Script => self.scripts,
                Purpose::Backup => self.backups,
                _ => 0,
            };
            if p == Purpose::Script
                && after.map_or(0, |id| id.as_u128()) + limit as u128 > count as u128
            {
                self.last_script_page.notify_one();
            }
            Ok((1..=count)
                .map(|i| Uuid::from_u128(i as u128))
                .filter(|id| after.is_none_or(|last| *id > last))
                .take(limit as usize)
                .map(|id| Self::definition(p, id))
                .collect())
        })
    }
    fn one(
        &self,
        p: Purpose,
        id: Uuid,
    ) -> BoxFuture<'_, Result<Option<ScheduleDefinition>, InstanceError>> {
        Box::pin(async move {
            self.reads.lock().unwrap().push((p, id));
            self.wake.notify_one();
            Ok(Some(Self::definition(p, id)))
        })
    }
    fn batch_deadline(
        &self,
        _: Purpose,
    ) -> BoxFuture<'_, Result<Option<DateTime<Utc>>, InstanceError>> {
        Box::pin(async { Ok(None) })
    }
}
impl Dispatch for Arc<Store> {
    fn eligible(&self, purpose: Purpose) -> bool {
        let allowed = self.eligible.load(Ordering::SeqCst);
        if !allowed && purpose == Purpose::Script {
            self.blocked_script.notify_one();
        }
        allowed
    }
    fn reader(&self) -> &dyn ScheduleReader {
        self.as_ref()
    }
    fn notify(&self) {}
    fn set_deadline(&self, p: Purpose, at: DateTime<Utc>) {
        self.projections.lock().unwrap().insert(p, at);
    }
    async fn occurrence(
        &self,
        p: Purpose,
        id: Uuid,
        minute: DateTime<Utc>,
        cancel: &CancellationToken,
    ) -> Result<bool, String> {
        let active = self.active.fetch_add(1, Ordering::SeqCst) + 1;
        self.maximum.fetch_max(active, Ordering::SeqCst);
        self.accepted.lock().unwrap().push((p, id, minute));
        self.wake.notify_one();
        if p == Purpose::Script || !self.hold_only_scripts.load(Ordering::SeqCst) {
            tokio::select! {permit=self.release.acquire()=>permit.unwrap().forget(),()=cancel.cancelled()=>{}}
        }
        self.active.fetch_sub(1, Ordering::SeqCst);
        Ok(true)
    }
    async fn batch(
        &self,
        _: Purpose,
        _: Option<uuid::Uuid>,
        _: &CancellationToken,
    ) -> Result<(), String> {
        Ok(())
    }
    async fn deadline(&self, _: Purpose) -> Result<Option<DateTime<Utc>>, String> {
        Ok(None)
    }
}
#[tokio::test]
async fn mixed_burst_is_bounded_and_an_edit_during_admission_is_reconciled() {
    let store = Store::new(20, 20);
    let token = CancellationToken::new();
    let (send, recv) = mpsc::channel(16);
    let (_g, git) = watch::channel((0, uuid::Uuid::nil()));
    let (_l, license) = watch::channel(());
    let run = tokio::spawn(drive(
        store.clone(),
        Box::new(Signal(recv)),
        git,
        license,
        token.clone(),
    ));
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        while store.maximum.load(Ordering::SeqCst) < 8 {
            store.wake.notified().await;
        }
        let (p, id, minute) = store.accepted.lock().unwrap()[0];
        send.send(RuntimeNotification::Schedule(format!("{}:{id}", p.name())))
            .await
            .unwrap();
        while !store.reads.lock().unwrap().contains(&(p, id)) {
            store.wake.notified().await;
        }
        store.release.add_permits(50);
        loop {
            let done = {
                let a = store.accepted.lock().unwrap();
                let unique: std::collections::BTreeSet<_> =
                    a.iter().map(|(p, id, _)| (*p, *id)).collect();
                unique.len() == 40
                    && a.iter()
                        .filter(|&&(kind, owner, t)| kind == p && id == owner && t == minute)
                        .count()
                        == 2
            };
            if done {
                break;
            }
            store.wake.notified().await;
        }
    })
    .await
    .unwrap();
    token.cancel();
    run.await.unwrap().unwrap();
    assert!(store.maximum.load(Ordering::SeqCst) <= 12);
    assert_eq!(store.active.load(Ordering::SeqCst), 0);
}
#[tokio::test]
async fn expired_busy_retry_cannot_create_a_late_execution() {
    use super::{
        coordinator::Request,
        planner::{Key, Output},
    };
    let store = Store::new(0, 0);
    let now = Utc::now();
    let old = now - Duration::minutes(2);
    let due = DateTime::from_timestamp(old.timestamp().div_euclid(60) * 60 + 5, 0).unwrap();
    let request = Request {
        key: Key::Calendar(Purpose::Backup, Uuid::nil()),
        recurrence: Some(Arc::new(
            citadel_primitives::schedule::CronSchedule::parse("* * * * *", "UTC").unwrap(),
        )),
        cursor: None,
        generation: 0,
    };
    let next = request
        .reconcile(&store, due, &CancellationToken::new())
        .await
        .unwrap();
    assert!(
        matches!(next,Output::Next(Some(at)) if at.timestamp().div_euclid(60) == now.timestamp().div_euclid(60))
    );
    assert!(store.accepted.lock().unwrap().is_empty());
}

#[test]
fn rebuild_pages_cannot_override_edits_or_leak_a_removed_deadline() {
    use super::planner::{Key, Output, Planner};
    use super::queue::Queues;
    let store = Store::new(0, 0);
    let mut planner = Planner::new(&store);
    planner.queue = Queues::default();
    let id = Uuid::from_u128(42);
    let purpose = Purpose::Script;
    let discover = Key::Discover(purpose);
    let load = Key::Load(purpose, id);
    let calendar = Key::Calendar(purpose, id);
    // Start discovery, then commit a disable while its page is in flight.
    let claim = |planner: &mut Planner, key| {
        planner.queue.wake(key);
        assert_eq!(
            planner.queue.start(tokio::time::Instant::now()).unwrap().0,
            key
        );
        planner.request(key).generation
    };
    // A generation owns its page cursors and tracks edits until the last page.
    let _ = claim(&mut planner, Key::Eligibility);
    planner.finish(Key::Eligibility, Ok(Output::Eligibility), &store);
    planner.queue = Queues::default();
    let generation = claim(&mut planner, discover);
    planner.notification(
        RuntimeNotification::Schedule(format!("Script:{id}")),
        &store,
    );
    // The edit is pending while the older page finishes in this purpose's read slot.
    assert!(planner.queue.start(tokio::time::Instant::now()).is_none());
    planner.finish(
        discover,
        Ok(Output::Page(
            generation,
            vec![Store::definition(purpose, id)],
        )),
        &store,
    );
    assert!(!planner.queue.contains(calendar));
    let _ = claim(&mut planner, load);
    planner.finish(load, Ok(Output::Definition(generation, None)), &store);
    assert!(!planner.queue.contains(calendar));
}

#[tokio::test]
async fn queued_burst_keeps_the_original_minute_and_skips_late_installation() {
    use super::{coordinator::Request, planner::Key};
    let store = Store::new(0, 0);
    store.release.add_permits(2);
    let occurrence =
        DateTime::from_timestamp((Utc::now().timestamp().div_euclid(60) - 1) * 60, 0).unwrap();
    let cron =
        Arc::new(citadel_primitives::schedule::CronSchedule::parse("* * * * *", "UTC").unwrap());
    for p in [Purpose::Script, Purpose::Backup, Purpose::Install] {
        let req = Request {
            key: Key::Calendar(p, Uuid::nil()),
            recurrence: Some(cron.clone()),
            cursor: None,
            generation: 0,
        };
        req.reconcile(&store, occurrence, &CancellationToken::new())
            .await
            .unwrap();
    }
    let accepted = store.accepted.lock().unwrap();
    assert_eq!(
        accepted.as_slice(),
        &[
            (Purpose::Script, Uuid::nil(), occurrence),
            (Purpose::Backup, Uuid::nil(), occurrence)
        ]
    );
}

#[tokio::test]
async fn entitlement_change_discovers_a_previously_blocked_schedule_without_editing_it() {
    let store = Store::new(1, 0);
    store.eligible.store(false, Ordering::SeqCst);
    let cancel = CancellationToken::new();
    let (_send, recv) = mpsc::channel(1);
    let (_g, git) = watch::channel((0, uuid::Uuid::nil()));
    let (license, changes) = watch::channel(());
    let task = tokio::spawn(drive(
        store.clone(),
        Box::new(Signal(recv)),
        git,
        changes,
        cancel.clone(),
    ));
    tokio::time::timeout(
        std::time::Duration::from_secs(3),
        store.blocked_script.notified(),
    )
    .await
    .unwrap();
    assert!(store.accepted.lock().unwrap().is_empty());
    store.eligible.store(true, Ordering::SeqCst);
    license.send_replace(());
    tokio::time::timeout(std::time::Duration::from_secs(3), store.wake.notified())
        .await
        .unwrap();
    assert_eq!(store.accepted.lock().unwrap()[0].0, Purpose::Script);
    cancel.cancel();
    task.await.unwrap().unwrap();
}

#[tokio::test]
async fn blocked_scripts_leave_capacity_for_other_purposes_and_definition_reads() {
    for size in [4, 512] {
        let store = Store::new(size, 0);
        store.hold_only_scripts.store(true, Ordering::SeqCst);
        let cancel = CancellationToken::new();
        let (tx, rx) = mpsc::channel(8);
        let (_git, g) = watch::channel((0, uuid::Uuid::nil()));
        let (_license, l) = watch::channel(());
        let task = tokio::spawn(drive(
            store.clone(),
            Box::new(Signal(rx)),
            g,
            l,
            cancel.clone(),
        ));
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            store.last_script_page.notified().await;
            while store.active.load(Ordering::SeqCst) != 4 {
                store.wake.notified().await;
            }
            // The Script FIFO and its dispatch slots remain occupied during both reads and admissions.
            for p in [Purpose::Backup, Purpose::Install] {
                tx.send(RuntimeNotification::Schedule(format!(
                    "{}:{}",
                    p.name(),
                    Uuid::nil()
                )))
                .await
                .unwrap();
            }
            loop {
                let accepted = store.accepted.lock().unwrap().clone();
                if [Purpose::Backup, Purpose::Install]
                    .iter()
                    .all(|p| accepted.iter().any(|(kind, _, _)| kind == p))
                {
                    break;
                }
                store.wake.notified().await;
            }
            assert_eq!(store.reads.lock().unwrap().len(), 2);
            // Each calendar family has one initial page, then one more per full Script page.
            assert_eq!(
                store.pages.load(Ordering::SeqCst),
                3 + size / super::planner::PAGE as usize
            );
            assert_eq!(
                store
                    .accepted
                    .lock()
                    .unwrap()
                    .iter()
                    .filter(|(p, _, _)| *p == Purpose::Script)
                    .count(),
                4
            );
            assert!(store.maximum.load(Ordering::SeqCst) <= 12);
        })
        .await
        .unwrap();
        cancel.cancel();
        task.await.unwrap().unwrap();
        assert_eq!(store.active.load(Ordering::SeqCst), 0);
    }
}

#[tokio::test(start_paused = true)]
async fn clock_corrections_preserve_elapsed_timers_and_recompute_calendar_occurrences() {
    use super::{
        planner::{Key, Output, Planner},
        queue::{Clock, Deadline},
    };
    use std::time::Duration as Elapsed;
    let store = Store::new(0, 0);
    let clock = Clock::now();
    let mut planner = Planner::new_at(&store, clock);
    let id = Uuid::from_u128(712);
    let load = Key::Load(Purpose::Script, id);
    planner.queue.wake(load);
    while let Some((key, _)) = planner.queue.start(clock.elapsed) {
        if key == load {
            planner.finish(
                key,
                Ok(Output::Definition(
                    0,
                    Some(Store::definition(Purpose::Script, id)),
                )),
                &store,
            );
            break;
        }
        planner.queue.finish(key, None);
    }
    let retry = Key::Batch(Purpose::Release);
    planner.queue.replace(retry, Some(Deadline::after(30)));
    let git = Key::Batch(Purpose::Git);
    let due = clock.wall + Duration::hours(2);
    planner.queue.replace_wall(git, Some(due), clock);

    for correction in [Duration::hours(-1), Duration::hours(1)] {
        let elapsed = Elapsed::from_secs(10);
        let changed = Clock {
            wall: clock.wall + Duration::seconds(10) + correction,
            elapsed: clock.elapsed + elapsed,
        };
        planner.clock_changed(changed, &store);
        for p in [
            Purpose::StackGit,
            Purpose::DeploymentImages,
            Purpose::StackImages,
            Purpose::ServiceImages,
        ] {
            let citadel_instance::ScheduleMode::Fixed {
                seconds,
                startup_label,
            } = p.mode()
            else {
                panic!()
            };
            let interval = Elapsed::from_secs(seconds as u64)
                + crate::workers::schedule::startup_offset(startup_label);
            assert_eq!(
                planner.queue.scheduled(Key::Batch(p)).unwrap().0,
                clock.elapsed + interval
            );
            assert_eq!(
                store.projections.lock().unwrap()[&p],
                changed.wall + Duration::from_std(interval - elapsed).unwrap()
            );
        }
        let calendar = planner
            .queue
            .scheduled(Key::Calendar(Purpose::Script, id))
            .unwrap();
        assert_eq!(calendar.0, changed.elapsed);
        assert_eq!(
            calendar.1.unwrap().timestamp(),
            changed.wall.timestamp().div_euclid(60) * 60
        );
        assert_eq!(
            planner.queue.scheduled(retry).unwrap().0,
            clock.elapsed + Elapsed::from_secs(30)
        );
        assert_eq!(
            planner.queue.scheduled(git).unwrap().0,
            changed.elapsed + (due - changed.wall).to_std().unwrap()
        );
    }
}

#[tokio::test(start_paused = true)]
async fn full_execution_lanes_reserve_reads_without_a_zero_delay_timer() {
    use super::{planner::Key, queue::Queues};
    let mut q = Queues::default();
    for i in 0..512 {
        q.wake(Key::Calendar(Purpose::Script, Uuid::from_u128(i)));
        q.wake(Key::Calendar(Purpose::Backup, Uuid::from_u128(i)));
    }
    for purpose in [Purpose::StackGit, Purpose::Image, Purpose::Release] {
        q.wake(Key::Batch(purpose));
    }
    for purpose in Purpose::ALL {
        q.wake(Key::Discover(purpose));
    }
    let mut executions = 0;
    let mut reads = 0;
    while let Some((key, _)) = q.start(tokio::time::Instant::now()) {
        if matches!(key, Key::Discover(_)) {
            reads += 1;
        } else {
            executions += 1;
        }
    }
    assert_eq!((executions, reads), (10, 2));
    assert!(
        q.next_deadline().is_none(),
        "only completion or an external change can free capacity"
    );
}

#[tokio::test]
async fn all_schedule_purposes_dispatch_in_their_declared_mode() {
    use super::{
        coordinator::Request,
        planner::{Key, Output},
    };
    use citadel_instance::ScheduleMode;
    let store = Store::new(0, 0);
    store.hold_only_scripts.store(true, Ordering::SeqCst);
    store.release.add_permits(1);
    let cron =
        Arc::new(citadel_primitives::schedule::CronSchedule::parse("* * * * *", "UTC").unwrap());
    let now = Utc::now();
    let minute = DateTime::from_timestamp(now.timestamp().div_euclid(60) * 60, 0).unwrap();
    let mut counts = [0; 3];
    let mut unique = std::collections::BTreeSet::new();
    for purpose in Purpose::ALL {
        assert!(unique.insert(purpose));
        let (key, index) = match purpose.mode() {
            ScheduleMode::Calendar => (Key::Calendar(purpose, Uuid::nil()), 0),
            ScheduleMode::PersistedDeadline => (Key::Batch(purpose), 1),
            ScheduleMode::Fixed { .. } => (Key::Batch(purpose), 2),
        };
        let request = Request {
            key,
            recurrence: Some(cron.clone()),
            cursor: None,
            generation: 0,
        };
        let result = request
            .reconcile(&store, minute, &CancellationToken::new())
            .await
            .unwrap();
        assert_eq!(matches!(result, Output::Next(_)), index == 0);
        assert_eq!(matches!(result, Output::BatchFinished), index != 0);
        counts[index] += 1;
    }
    assert_eq!(counts, [3, 3, 4]);
}

#[tokio::test(start_paused = true)]
async fn repeated_ref_wakes_coalesce_and_overflow_preserves_the_periodic_sweep() {
    use super::planner::{Key, Planner};
    let store = Store::new(0, 0);
    let mut planner = Planner::new(&store);
    let fallback = Key::Batch(Purpose::StackGit);
    let original = planner.queue.scheduled(fallback).unwrap();
    let first = Uuid::from_u128(1);
    for _ in 0..200 {
        planner.wake_ref(first);
    }
    assert!(planner.queue.contains(Key::GitRef(first)));
    assert_eq!(planner.queue.scheduled(fallback), Some(original));
    for i in 2..=128 {
        planner.wake_ref(Uuid::from_u128(i));
    }
    let overflow = Uuid::from_u128(129);
    planner.wake_ref(overflow);
    assert!(!planner.queue.contains(Key::GitRef(overflow)));
    assert!(planner.queue.scheduled(fallback).unwrap().0 < original.0);
}
