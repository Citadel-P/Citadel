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
    fn page(
        &self,
        p: Option<Purpose>,
        after: Option<(Purpose, Uuid)>,
        limit: i64,
    ) -> BoxFuture<'_, Result<Vec<ScheduleDefinition>, InstanceError>> {
        Box::pin(async move {
            self.pages.fetch_add(1, Ordering::SeqCst);
            let p = p.unwrap();
            let count = match p {
                Purpose::Script => self.scripts,
                Purpose::Backup => self.backups,
                _ => 0,
            };
            Ok((1..=count)
                .map(|i| Uuid::from_u128(i as u128))
                .filter(|id| after.is_none_or(|(_, last)| *id > last))
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
        Box::pin(async move { Ok(Some(Self::definition(p, id))) })
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
    fn set_deadline(&self, _: Purpose, _: DateTime<Utc>) {}
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
        tokio::select! {permit=self.release.acquire()=>permit.unwrap().forget(),()=cancel.cancelled()=>{}}
        self.active.fetch_sub(1, Ordering::SeqCst);
        Ok(true)
    }
    async fn batch(&self, _: Purpose, _: &CancellationToken) -> Result<(), String> {
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
    let (_g, git) = watch::channel(());
    let (_l, license) = watch::channel(());
    let run = tokio::spawn(drive(
        store.clone(),
        Box::new(Signal(recv)),
        git,
        license,
        token.clone(),
    ));
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        while store.maximum.load(Ordering::SeqCst) < 12 {
            store.wake.notified().await;
        }
        let (p, id, minute) = store.accepted.lock().unwrap()[0];
        send.send(RuntimeNotification::Schedule(format!("{}:{id}", p.name())))
            .await
            .unwrap();
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
    use citadel_runtime::delayed_queue::DelayedQueue;
    let store = Store::new(0, 0);
    let mut planner = Planner::new(&store);
    planner.queue = DelayedQueue::new(8);
    let id = Uuid::from_u128(42);
    let purpose = Purpose::Script;
    let discover = Key::Discover(purpose);
    let load = Key::Load(purpose, id);
    let calendar = Key::Calendar(purpose, id);
    // Start discovery, then commit a disable while its page is in flight.
    let claim = |planner: &mut Planner, key| {
        planner.queue.wake(key, Utc::now());
        planner.queue.promote(Utc::now());
        assert_eq!(planner.queue.start().unwrap().0, key);
        planner.request(key).generation
    };
    // A generation owns its page cursors and tracks edits until the last page.
    let _ = claim(&mut planner, Key::Eligibility);
    planner.finish(Key::Eligibility, Ok(Output::Next(None)), &store);
    planner.queue = DelayedQueue::new(8);
    let generation = claim(&mut planner, discover);
    planner.notification(
        RuntimeNotification::Schedule(format!("Script:{id}")),
        &store,
    );
    let _ = claim(&mut planner, load);
    planner.finish(load, Ok(Output::Definition(generation, None)), &store);
    // A page from before the edit must not put the calendar back.
    planner.finish(
        discover,
        Ok(Output::Page(
            generation,
            vec![Store::definition(purpose, id)],
        )),
        &store,
    );
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
    let (_g, git) = watch::channel(());
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
