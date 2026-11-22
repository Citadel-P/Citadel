//! Exercise the production delayed coordinator and release handler with an isolated settings source.
use super::runtime;
use crate::workers::schedules::{Dispatch, drive};
use chrono::{DateTime, Utc};
use citadel_instance::{
    InstanceError, ScheduleDefinition, SchedulePurpose as Purpose, ScheduleReader,
};
use citadel_runtime::notifications::{NotificationError, NotificationSource, RuntimeNotification};
use citadel_updates::ReleaseCheckService;
use futures_util::future::BoxFuture;
use std::sync::Arc;
use tokio::sync::watch;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

#[derive(Clone, Copy, Default)]
pub(super) struct CheckSchedule {
    pub enabled: bool,
    pub interval_hours: i32,
}
enum Policy {
    Watch(watch::Receiver<CheckSchedule>),
    Instance(Arc<citadel_instance::InstanceService>),
}
struct ReleaseFixture {
    service: ReleaseCheckService,
    policy: Policy,
}
impl ReleaseFixture {
    async fn settings(&self) -> CheckSchedule {
        match &self.policy {
            Policy::Watch(w) => *w.borrow(),
            Policy::Instance(s) => {
                let v = s.installation_settings().await.unwrap();
                CheckSchedule {
                    enabled: v.initialized && v.update_checks_enabled,
                    interval_hours: v.update_check_interval_hours,
                }
            }
        }
    }
}
struct Changes<T>(watch::Receiver<T>);
impl<T: Send + Sync> NotificationSource for Changes<T> {
    fn recv(&mut self) -> BoxFuture<'_, Result<RuntimeNotification, NotificationError>> {
        Box::pin(async move {
            if self.0.changed().await.is_err() {
                return std::future::pending().await;
            }
            Ok(RuntimeNotification::Schedule("ReleasePolicy".into()))
        })
    }
}
impl ScheduleReader for ReleaseFixture {
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
        _: Purpose,
        _: Option<Uuid>,
        _: i64,
    ) -> BoxFuture<'_, Result<Vec<ScheduleDefinition>, InstanceError>> {
        Box::pin(async { Ok(vec![]) })
    }
    fn one(
        &self,
        _: Purpose,
        _: Uuid,
    ) -> BoxFuture<'_, Result<Option<ScheduleDefinition>, InstanceError>> {
        unreachable!()
    }
    fn batch_deadline(
        &self,
        _: Purpose,
    ) -> BoxFuture<'_, Result<Option<DateTime<Utc>>, InstanceError>> {
        Box::pin(async { Ok(None) })
    }
}
impl Dispatch for ReleaseFixture {
    fn reader(&self) -> &dyn ScheduleReader {
        self
    }
    fn notify(&self) {}
    fn set_deadline(&self, _: Purpose, _: DateTime<Utc>) {}
    async fn occurrence(
        &self,
        _: Purpose,
        _: Uuid,
        _: DateTime<Utc>,
        _: &CancellationToken,
    ) -> Result<bool, String> {
        unreachable!()
    }
    async fn deadline(&self, p: Purpose) -> Result<Option<DateTime<Utc>>, String> {
        if p != Purpose::Release {
            return Ok(None);
        }
        let cfg = self.settings().await;
        runtime::deadline(&self.service, cfg.enabled, cfg.interval_hours).await
    }
    async fn batch(
        &self,
        p: Purpose,
        _: Option<uuid::Uuid>,
        cancel: &CancellationToken,
    ) -> Result<(), String> {
        if p != Purpose::Release {
            return Ok(());
        }
        let cfg = self.settings().await;
        runtime::check(&self.service, cfg.enabled, cfg.interval_hours, cancel).await
    }
}
pub(super) async fn run_enabled(
    service: ReleaseCheckService,
    policy: watch::Receiver<CheckSchedule>,
    cancel: CancellationToken,
) -> Result<(), std::convert::Infallible> {
    let (_keep, git) = watch::channel((0, uuid::Uuid::nil()));
    let (_license, license) = watch::channel(());
    drive(
        ReleaseFixture {
            service,
            policy: Policy::Watch(policy.clone()),
        },
        Box::new(Changes(policy)),
        git,
        license,
        cancel,
    )
    .await
}
pub(super) async fn first_run(
    service: ReleaseCheckService,
    instance: Arc<citadel_instance::InstanceService>,
    setup: watch::Receiver<()>,
    cancel: CancellationToken,
) -> Result<(), std::convert::Infallible> {
    let (_keep, git) = watch::channel((0, uuid::Uuid::nil()));
    let (_license, license) = watch::channel(());
    drive(
        ReleaseFixture {
            service,
            policy: Policy::Instance(instance),
        },
        Box::new(Changes(setup)),
        git,
        license,
        cancel,
    )
    .await
}
