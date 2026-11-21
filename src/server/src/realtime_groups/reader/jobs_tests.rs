use super::jobs::stream;
use super::*;
use citadel_instance::{
    DatabaseConnections, InstallationSettings, InstanceError, InstanceRepository, InstanceService,
    PatchInstallationSettings,
};
use citadel_runtime::jobs::{JobCatalog, JobOutcome, JobOwnership};
use futures_util::{StreamExt, future::BoxFuture};
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

struct FixtureJobs(JobCatalog);
impl citadel_instance::InstanceJobsReader for FixtureJobs {
    fn snapshot(&self) -> citadel_instance::JobsSnapshot {
        serde_json::from_value(serde_json::to_value(self.0.snapshot()).unwrap()).unwrap()
    }
    fn changes(&self) -> tokio::sync::watch::Receiver<u64> {
        self.0.subscribe()
    }
}
struct NoDatabase;
impl InstanceRepository for NoDatabase {
    fn database_connections(&self) -> BoxFuture<'_, Result<DatabaseConnections, InstanceError>> {
        panic!("Jobs must not query the database")
    }
    fn installation_settings(&self) -> BoxFuture<'_, Result<InstallationSettings, InstanceError>> {
        panic!("Jobs must not query settings")
    }
    fn patch_installation_settings<'a>(
        &'a self,
        _: &'a ActorPrincipal,
        _: &'a PatchInstallationSettings,
        _: chrono::DateTime<chrono::Utc>,
    ) -> BoxFuture<'a, Result<InstallationSettings, InstanceError>> {
        panic!("Jobs must not write")
    }
}
fn fixture() -> (Arc<InstanceService>, JobCatalog, ActorPrincipal) {
    let catalog = JobCatalog::new(Uuid::now_v7());
    let service = InstanceService::new(
        Arc::new(NoDatabase),
        Arc::new(citadel_identity::SystemClock),
    )
    .with_jobs(Arc::new(FixtureJobs(catalog.clone())));
    let id = Uuid::now_v7();
    let principal = ActorPrincipal {
        subject_id: id,
        actor_id: citadel_primitives::ActorId::new(id),
        name: "admin".into(),
        principal_type: citadel_identity::AuthenticatedPrincipalType::User,
        credential_id: None,
        roles: vec!["Admin".into()],
    };
    (Arc::new(service), catalog, principal)
}
#[tokio::test(start_paused = true)]
async fn initial_trailing_and_reconnect_snapshots_converge_without_database_work() {
    let (service, catalog, principal) = fixture();
    let job = catalog.register(
        "fixture",
        citadel_primitives::JobCategory::Other,
        JobOwnership::Coordinator,
    );
    let token = CancellationToken::new();
    let mut updates = stream(service.clone(), &principal, &token).unwrap();
    assert_eq!(
        updates.next().await.unwrap().unwrap().arguments[0]["jobs"][0]["iterations"],
        0
    );
    for _ in 0..1000 {
        job.begin()
            .finish(JobOutcome::Productive, Some((1, "rows")));
    }
    // A slow browser holds no worker locks; only the final current state is retained.
    let latest = updates.next().await.unwrap().unwrap();
    assert_eq!(latest.arguments[0]["jobs"][0]["iterations"], 1000);
    assert_eq!(latest.arguments[0]["jobs"][0]["state"], "waiting");
    assert_eq!(latest.arguments[0]["jobs"][0]["resourceType"], "Other");
    assert!(
        tokio::time::timeout(std::time::Duration::from_secs(5), updates.next())
            .await
            .is_err(),
        "idle subscriptions must not send ticks"
    );
    let mut rejoined = stream(service, &principal, &token).unwrap();
    assert_eq!(
        rejoined.next().await.unwrap().unwrap().arguments[0]["revision"],
        latest.arguments[0]["revision"]
    );
    token.cancel();
    assert!(updates.next().await.is_none());
}
#[test]
fn snapshot_and_stream_reject_non_administrators_and_service_accounts() {
    let (service, _, mut principal) = fixture();
    principal.roles.clear();
    assert!(matches!(
        service.jobs(&principal),
        Err(InstanceError::Forbidden)
    ));
    assert!(stream(service.clone(), &principal, &CancellationToken::new()).is_err());
    principal.roles.push("Admin".into());
    principal.principal_type = citadel_identity::AuthenticatedPrincipalType::ServiceAccount;
    assert!(matches!(
        service.jobs(&principal),
        Err(InstanceError::Forbidden)
    ));
    assert!(stream(service, &principal, &CancellationToken::new()).is_err());
}
#[test]
fn restart_replaces_identity_and_counters() {
    let (_, old, _) = fixture();
    old.register(
        "fixture",
        citadel_primitives::JobCategory::Other,
        JobOwnership::Coordinator,
    )
    .begin()
    .finish(JobOutcome::Completed, None);
    let (_, fresh, _) = fixture();
    fresh.register(
        "fixture",
        citadel_primitives::JobCategory::Other,
        JobOwnership::Coordinator,
    );
    assert_ne!(old.snapshot().boot_id, fresh.snapshot().boot_id);
    assert_eq!(fresh.snapshot().jobs[0].iterations, 0);
}

#[tokio::test]
async fn history_requires_administrator_before_storage_and_validates_ranges() {
    let (service, _, mut principal) = fixture();
    principal.roles.clear();
    assert!(matches!(
        service.job_history(&principal, None, 24).await,
        Err(InstanceError::Forbidden)
    ));
    principal.roles = vec!["Admin".into()];
    assert!(matches!(
        service.job_history(&principal, None, 169).await,
        Err(InstanceError::Validation(_))
    ));
    assert!(matches!(
        service.job_history(&principal, Some("bad/id"), 24).await,
        Err(InstanceError::Validation(_))
    ));
}

#[tokio::test]
async fn schedule_catalog_and_subscription_require_a_human_administrator() {
    let (service, _, mut principal) = fixture();
    assert!(service.schedule_changes(&principal).is_ok());
    assert!(matches!(
        service.schedules(&principal, None, None).await,
        Err(InstanceError::Conflict(_))
    ));
    for kind in [
        citadel_identity::AuthenticatedPrincipalType::User,
        citadel_identity::AuthenticatedPrincipalType::ServiceAccount,
    ] {
        principal.principal_type = kind;
        principal.roles = if matches!(kind, citadel_identity::AuthenticatedPrincipalType::User) {
            vec![]
        } else {
            vec!["Admin".into()]
        };
        assert!(matches!(
            service.schedules(&principal, None, None).await,
            Err(InstanceError::Forbidden)
        ));
        assert!(matches!(
            service.schedule_changes(&principal),
            Err(InstanceError::Forbidden)
        ));
    }
}
