//! All futures belong to this supervisor task. Feature transactions own accepted work.
use super::{
    Handlers,
    handlers::Dispatch,
    planner::{Key, Output, PAGE, Planner, Time, later},
    queue::Clock,
};
use chrono::{DateTime, Duration, Utc};
use citadel_instance::{ScheduleMode, SchedulePurpose as Purpose, calendar_deadline};
use citadel_primitives::schedule::CronSchedule;
use citadel_runtime::notifications::{NotificationSource, RuntimeNotification};
use futures_util::{FutureExt, StreamExt, stream::FuturesUnordered};
use std::sync::Arc;
use tokio::sync::watch;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

pub(super) struct Request {
    pub key: Key,
    pub recurrence: Option<Arc<CronSchedule>>,
    pub cursor: Option<Uuid>,
    pub generation: u64,
}
impl Request {
    pub(super) async fn reconcile(
        self,
        h: &impl Dispatch,
        due: Time,
        token: &CancellationToken,
    ) -> Result<Output, String> {
        match self.key {
            Key::Eligibility => {
                h.refresh_eligibility().await?;
                Ok(Output::Eligibility)
            }
            Key::Load(p, id) => {
                let mut def = if h.eligible(p) {
                    h.reader().one(p, id).await.map_err(|e| e.to_string())?
                } else {
                    None
                };
                if let Some(d) = &mut def {
                    h.apply_policy(d);
                }
                Ok(Output::Definition(self.generation, def.map(Box::new)))
            }
            Key::Discover(p) if p.mode() == ScheduleMode::Calendar => {
                let mut defs = if h.eligible(p) {
                    h.reader()
                        .discover(p, self.cursor, PAGE)
                        .await
                        .map_err(|e| e.to_string())?
                } else {
                    Vec::new()
                };
                for d in &mut defs {
                    h.apply_policy(d);
                }
                Ok(Output::Page(self.generation, defs))
            }
            Key::Discover(p) => Ok(Output::Deadline(h.deadline(p).await?)),
            Key::Calendar(p, id) => {
                let Some(cron) = self.recurrence else {
                    return Ok(Output::Next(None));
                };
                let minute = DateTime::from_timestamp(due.timestamp().div_euclid(60) * 60, 0)
                    .expect("valid UTC minute");
                // Cron deadlines are exact UTC minutes. A later retry belongs to that same occurrence.
                if (due != minute || p == Purpose::Install)
                    && Utc::now() >= minute + Duration::minutes(1)
                {
                    return Ok(Output::Next(calendar_deadline(
                        &cron,
                        Some(minute),
                        Utc::now(),
                    )));
                }
                let accepted =
                    citadel_runtime::jobs::observe_work(h.occurrence(p, id, minute, token), |ok| {
                        Some((u64::from(*ok), "accepted occurrences"))
                    })
                    .await;
                let retry = later(5);
                if let Err(e) = &accepted {
                    tracing::warn!(%e,?p,%id,"Scheduled occurrence could not be accepted");
                }
                if !matches!(accepted, Ok(true))
                    && retry < minute + Duration::minutes(1)
                    && (p != Purpose::Install || accepted.is_err())
                {
                    return Ok(Output::Next(Some(retry)));
                }
                h.notify();
                Ok(Output::Next(calendar_deadline(
                    &cron,
                    Some(minute),
                    Utc::now(),
                )))
            }
            Key::GitRef(reference) => {
                citadel_runtime::jobs::observe(h.batch(Purpose::StackGit, Some(reference), token))
                    .await?;
                Ok(Output::BatchFinished)
            }
            Key::Batch(p) => {
                citadel_runtime::jobs::observe(h.batch(p, None, token)).await?;
                Ok(Output::BatchFinished)
            }
        }
    }
}

pub(crate) async fn run(
    h: Handlers,
    changes: Box<dyn NotificationSource>,
    licensing: watch::Receiver<()>,
    cancel: CancellationToken,
) -> Result<(), std::convert::Infallible> {
    let git = h.git.subscribe_completion();
    drive(h, changes, git, licensing, cancel).await
}

pub(crate) async fn drive(
    h: impl Dispatch,
    mut changes: Box<dyn NotificationSource>,
    mut git: watch::Receiver<(u64, Uuid)>,
    mut licensing: watch::Receiver<()>,
    cancel: CancellationToken,
) -> Result<(), std::convert::Infallible> {
    let mut git_sequence = git.borrow().0;
    let mut planner = Planner::new(&h);
    let mut work = FuturesUnordered::new();
    let mut release_token = cancel.child_token();
    let mut previous = Clock::now();
    let h = &h;
    loop {
        if cancel.is_cancelled() {
            break;
        }
        let clock = Clock::now();
        if (clock.wall - previous.project(clock.elapsed))
            .num_seconds()
            .unsigned_abs()
            > 5
        {
            planner.clock_changed(clock, h);
            previous = clock;
        }
        while let Some((key, due)) = planner.queue.start(clock.elapsed) {
            let token = if key == Key::Batch(Purpose::Release) {
                release_token.clone()
            } else {
                cancel.clone()
            };
            let request = planner.request(key);
            work.push(
                async move {
                    let pending = request.reconcile(h, due.unwrap_or(clock.wall), &token);
                    let result = match key {
                        Key::Batch(_) | Key::GitRef(_) | Key::Calendar(..) => pending.await,
                        _ => citadel_runtime::jobs::observe(pending).await,
                    };
                    (key, result)
                }
                .boxed(),
            );
        }
        let next = planner.queue.next_deadline();
        let until_due = next.map(|at| at.saturating_duration_since(clock.elapsed));
        if let Some(job) = citadel_runtime::jobs::JobHandle::current() {
            let (ready, saturated) = planner.queue.backlog();
            job.backlog(ready, saturated);
            if work.is_empty() {
                job.waiting("Next resource schedule or change", next);
            }
        }
        let delay = until_due
            .unwrap_or(std::time::Duration::MAX)
            .min(std::time::Duration::from_secs(30));
        tokio::select! {
            ()=cancel.cancelled()=>break,
            msg=changes.recv()=>{
                let msg=msg.unwrap_or(RuntimeNotification::Resync);
                if matches!(&msg,RuntimeNotification::Schedule(name) if name=="ReleasePolicy") || matches!(msg,RuntimeNotification::Resync){
                    release_token.cancel();
                    release_token=cancel.child_token();
                }
                planner.notification(msg,h);
            }
            Ok(())=licensing.changed()=>planner.queue.wake(Key::Eligibility),
            Ok(())=git.changed()=>{
                let (sequence,reference) = *git.borrow_and_update();
                if sequence == git_sequence.wrapping_add(1) { planner.wake_ref(reference); }
                else { planner.queue.wake(Key::Batch(Purpose::StackGit)); }
                git_sequence = sequence;
            },
            Some((key,result))=work.next(),if !work.is_empty()=>planner.finish(key,result,h),
            ()=tokio::time::sleep(delay)=>{}
        }
    }
    // A release check already committing and other accepted admissions must settle on shutdown.
    while work.next().await.is_some() {}
    Ok(())
}
