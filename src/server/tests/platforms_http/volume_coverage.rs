use super::*;
use citadel_backups::{coverage::*, runs::read_models::*, *};
use citadel_server::{
    api::resources::backups::coverage::BackupCoverageService,
    realtime_groups::{Group, GroupReadPort},
};
use futures_util::future::BoxFuture;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio_util::sync::CancellationToken;

struct CoverageFixture {
    platform: Uuid,
    reader: ActorId,
    failed: AtomicBool,
}
impl CoverageReader for CoverageFixture {
    fn snapshot<'a>(
        &'a self,
        _: ActorId,
        _: bool,
        _: &'a [Uuid],
    ) -> BoxFuture<'a, Result<CoverageSnapshot, BackupError>> {
        Box::pin(async { panic!("volume reads must use their coverage projection") })
    }
}
impl VolumeCoverageReader for CoverageFixture {
    fn volume_coverage<'a>(
        &'a self,
        actor: ActorId,
        administrator: bool,
        platform: Uuid,
        volumes: &'a [(String, Option<String>)],
        _: bool,
    ) -> BoxFuture<'a, Result<Vec<VolumeBackupCoverage>, BackupError>> {
        Box::pin(async move {
            assert_eq!(platform, self.platform);
            assert!(administrator || actor == self.reader);
            Ok(volumes
                .iter()
                .map(|(name, node)| VolumeBackupCoverage {
                    platform_id: platform,
                    volume_name: name.clone(),
                    docker_node_id: node.clone(),
                    status: if !administrator {
                        BackupCoverageStatus::Unprotected
                    } else if self.failed.load(Ordering::SeqCst) {
                        BackupCoverageStatus::Failed
                    } else if node.is_some() {
                        BackupCoverageStatus::Protected
                    } else {
                        BackupCoverageStatus::Warning
                    },
                    policy_count: i32::from(administrator),
                    policies: if administrator {
                        vec![VolumeCoveragePolicy {
                            id: Uuid::from_u128(42),
                            name: "Daily data".into(),
                            enabled: true,
                            schedule_configured: node.is_some(),
                        }]
                    } else {
                        vec![]
                    },
                    reasons: vec![],
                    last_run_id: None,
                    last_run_status: None,
                    last_run_at: None,
                    last_successful_run_at: None,
                })
                .collect())
        })
    }
}
pub(super) struct NoExecution;
impl BackupExecutor for NoExecution {
    fn repository<'a>(
        &'a self,
        _: &'a BackupRepository,
        _: &'a str,
        _: &'a str,
        _: Option<Uuid>,
        _: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Vec<BackupLog>, BackupError>> {
        panic!("read-only fixture")
    }
    fn backup<'a>(
        &'a self,
        _: &'a BackupClaim,
        _: &'a BackupSourcePlan,
        _: &'a CancellationToken,
    ) -> BoxFuture<'a, BackupExecutionResult> {
        panic!("read-only fixture")
    }
    fn restore<'a>(
        &'a self,
        _: &'a RestoreClaim,
        _: &'a CancellationToken,
    ) -> BoxFuture<'a, RestoreExecutionResult> {
        panic!("read-only fixture")
    }
}

#[tokio::test]
#[ignore = "requires disposable CITADEL_PLATFORM_DATABASE_URL"]
async fn volume_realtime_preserves_actor_scoped_coverage_on_join_deltas_and_backup_changes() {
    let f = fixture().await;
    let actor = super::lookup::subject(&f).await;
    super::lookup::grant(
        &f,
        actor.actor_id.value(),
        citadel_primitives::ResourceType::Platform,
        f.platform_id,
        0,
    )
    .await;
    let unauthorized = super::lookup::subject(&f).await;
    let coverage = Arc::new(CoverageFixture {
        platform: f.platform_id,
        reader: actor.actor_id,
        failed: AtomicBool::new(false),
    });
    let mut reader = super::realtime_groups::reader(&f);
    let backups = Arc::new(BackupService::new(
        reader.backups.clone(), Arc::new(NoExecution),
        Arc::new(citadel_adapters::persistence::postgres::backups::source_planner::PostgresBackupSourcePlanner::new(f.pool.clone())),
        Duration::minutes(5),
        Arc::new(citadel_adapters::security::identity::backup_authorization::IdentityBackupRunAuthorizer::new(reader.identity.clone())),
    ));
    reader.backup_coverage = Some(Arc::new(BackupCoverageService {
        backups,
        identity: reader.identity.clone(),
        volume_coverage: coverage.clone(),
    }));
    let node_volume = citadel_platforms::RuntimeVolumeSummary {
        name: "data".into(),
        driver: "local".into(),
        ..Default::default()
    };
    sqlx::query("INSERT INTO swarmnodevolumeprojections(platformid,dockernodeid,volumename,observedat,resource) VALUES($1,'node-1','data',NOW(),$2)")
        .bind(f.platform_id).bind(serde_json::to_value(&node_volume).unwrap()).execute(&f.pool).await.unwrap();
    let group = Group::parse(&format!("docker-daemon:{}", f.platform_id)).unwrap();
    let joined = reader.read(&f.administrator, &group, None).await.unwrap();
    let manager = joined
        .rows
        .iter()
        .find(|r| r.target == "VolumeEventReceived")
        .unwrap();
    assert_eq!(manager.rows[0]["backupCoverage"]["status"], "Warning");
    assert_eq!(
        manager.rows[0]["backupCoverage"]["policies"][0]["name"],
        "Daily data"
    );
    assert_eq!(
        manager.rows[0]["backupCoverage"]["policies"][0]["scheduleConfigured"],
        false
    );
    assert_eq!(
        manager.rows[0]["backupCoverage"]["policies"][0]["canWrite"],
        true
    );
    assert_eq!(manager.rows[0]["backupCoverage"]["canConfigure"], true);
    let payload = serde_json::to_value(&joined.events).unwrap();
    let volumes = &payload[0]["arguments"][0]["volumes"];
    assert_eq!(volumes.as_array().unwrap().len(), 2);
    assert!(
        volumes
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v["dockerNodeId"] == "node-1" && v["backupCoverage"]["status"] == "Protected")
    );

    let hub = citadel_server::realtime::RealtimeHub::new(
        16,
        Arc::new(citadel_server::metrics::Metrics::default()),
    );
    let mut events = hub.subscribe();
    hub.publish_resource_observation(
        f.platform_id,
        None,
        "create",
        &citadel_platforms::jobs::ResourceDelta::Volume {
            id: "data".into(),
            value: Some(node_volume.clone()),
        },
    );
    let delta = events.recv().await.unwrap();
    let admin = reader
        .read(&f.administrator, &group, Some(&delta))
        .await
        .unwrap();
    assert_eq!(admin.rows[0].rows[0]["backupCoverage"]["status"], "Warning");
    let limited = reader.read(&actor, &group, Some(&delta)).await.unwrap();
    assert_eq!(
        limited.rows[0].rows[0]["backupCoverage"]["status"],
        "Unprotected"
    );
    assert_eq!(limited.rows[0].rows[0]["backupCoverage"]["policyCount"], 0);
    assert_eq!(
        limited.rows[0].rows[0]["backupCoverage"]["policies"],
        serde_json::json!([])
    );
    assert_eq!(
        limited.rows[0].rows[0]["backupCoverage"]["canConfigure"],
        false
    );
    assert!(
        reader
            .read(&unauthorized, &group, Some(&delta))
            .await
            .is_err()
    );

    hub.publish_resource_observation(
        f.platform_id,
        Some("node-1"),
        "create",
        &citadel_platforms::jobs::ResourceDelta::Volume {
            id: "data".into(),
            value: Some(node_volume),
        },
    );
    let node_delta = events.recv().await.unwrap();
    let snapshot = reader
        .read(&f.administrator, &group, Some(&node_delta))
        .await
        .unwrap();
    let events_json = serde_json::to_value(&snapshot.events).unwrap();
    assert_eq!(
        events_json[0]["arguments"][0]["volumes"][0]["backupCoverage"]["status"],
        "Protected"
    );

    coverage.failed.store(true, Ordering::SeqCst);
    for change in [
        Some(CoverageInvalidation {
            platform_id: f.platform_id,
            ..Default::default()
        }),
        None,
    ] {
        hub.publish_backup_coverage(change.as_ref());
        let change = events.recv().await.unwrap();
        assert!(group.affected_by(&change));
        let updated = reader
            .read(&f.administrator, &group, Some(&change))
            .await
            .unwrap();
        assert_eq!(
            updated.rows.len(),
            1,
            "backup changes must not refresh unrelated daemon resources"
        );
        assert_eq!(updated.rows[0].target, "VolumeEventReceived");
        assert_eq!(
            updated.rows[0].rows[0]["backupCoverage"]["status"],
            "Failed"
        );
        let events_json = serde_json::to_value(&updated.events).unwrap();
        assert_eq!(
            events_json[0]["arguments"][0]["volumes"][0]["backupCoverage"]["status"],
            "Failed"
        );
    }
    // A global resync must resolve each platform independently. Reusing the
    // first platform's cached volume list would leak it to an offline sibling.
    let other = Uuid::now_v7();
    sqlx::query("INSERT INTO platforms(id,address,connectortype,cpucount,imagecount,memtotal,name,networkcount,platformdescriptor,status,volumecount) VALUES($1,$2,'EdgeAgent',0,0,0,$2,0,'{\"$type\":\"DockerStandalone\"}','Offline',0)")
        .bind(other).bind(format!("offline-{other}")).execute(&f.pool).await.unwrap();
    hub.publish_backup_coverage(None);
    let global = events.recv().await.unwrap();
    reader
        .read(&f.administrator, &group, Some(&global))
        .await
        .unwrap();
    let mut offline_reader = reader.clone();
    offline_reader.backup_coverage = None;
    let offline_group = Group::parse(&format!("docker-daemon:{other}")).unwrap();
    let offline = offline_reader
        .read(&f.administrator, &offline_group, Some(&global))
        .await
        .unwrap();
    assert!(
        offline.rows.is_empty(),
        "unavailable inventory must not delete existing manager rows"
    );
    let payload = serde_json::to_value(&offline.events).unwrap();
    assert!(
        payload[0]["arguments"][0]["volumes"]
            .as_array()
            .unwrap()
            .is_empty(),
        "global events must not leak another platform's volumes"
    );

    // Inventory can fail before the connection monitor marks a platform offline.
    sqlx::query("UPDATE platforms SET status='Online' WHERE id=$1")
        .bind(other)
        .execute(&f.pool)
        .await
        .unwrap();
    hub.publish_backup_coverage(None);
    let change = events.recv().await.unwrap();
    let unavailable = offline_reader
        .read(&f.administrator, &offline_group, Some(&change))
        .await
        .unwrap();
    assert!(unavailable.rows.is_empty());

    // A disconnected manager must not block persisted worker coverage or close the socket.
    sqlx::query("UPDATE platforms SET status='Offline' WHERE id=$1")
        .bind(f.platform_id)
        .execute(&f.pool)
        .await
        .unwrap();
    hub.publish_backup_coverage(None);
    let change = events.recv().await.unwrap();
    let offline = reader
        .read(&f.administrator, &group, Some(&change))
        .await
        .unwrap();
    assert!(offline.rows.is_empty());
    let payload = serde_json::to_value(&offline.events).unwrap();
    assert_eq!(
        payload[0]["arguments"][0]["volumes"][0]["backupCoverage"]["status"],
        "Failed"
    );
    assert!(
        reader
            .read(&unauthorized, &group, Some(&change))
            .await
            .is_err()
    );
    super::realtime_groups::cleanup(f).await;
}
