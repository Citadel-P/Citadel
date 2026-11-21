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
        _: Option<&'a CoverageSelection>,
    ) -> BoxFuture<'a, Result<IndexedCoverage, BackupError>> {
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
        _operation_id: Uuid,
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
    let volume_group = Group::parse(&format!("volumes:{}", f.platform_id)).unwrap();
    let joined_volumes = reader
        .read(&f.administrator, &volume_group, None)
        .await
        .unwrap();
    assert_eq!(joined_volumes.events.len(), 1);
    let joined_volume = &joined_volumes.events[0];
    assert_eq!(joined_volume.target, "VolumesUpdated");
    assert_eq!(joined_volume.arguments[0]["scope"], "all");
    assert_eq!(
        joined_volume.arguments[0]["volumes"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert!(
        reader
            .read(&unauthorized, &volume_group, None)
            .await
            .is_err()
    );
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
    hub.publish_runtime_change(f.platform_id, "containerStats", "update", "");
    assert!(!volume_group.affected_by(&events.recv().await.unwrap()));
    hub.publish_runtime_change(f.platform_id, "network", "create", "bridge");
    assert!(!volume_group.affected_by(&events.recv().await.unwrap()));
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
    assert!(volume_group.affected_by(&delta));
    let scoped = reader
        .read(&actor, &volume_group, Some(&delta))
        .await
        .unwrap();
    assert_eq!(scoped.events[0].arguments[0]["scope"], "one");
    assert_eq!(scoped.events[0].arguments[0]["id"], "data");
    assert_eq!(
        scoped.events[0].arguments[0]["volumes"][0]["backupCoverage"]["policyCount"],
        0
    );
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
    let scoped = reader
        .read(&actor, &volume_group, Some(&change))
        .await
        .unwrap();
    assert!(
        scoped
            .events
            .iter()
            .all(|e| e.target == "VolumesUpdated" && e.arguments[0]["scope"] == "nodes")
    );
    assert_eq!(scoped.events.len(), 1);
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

struct ScheduledBackups;
impl BackupEntitlements for ScheduledBackups {
    fn automated_operations(&self) -> BoxFuture<'_, Result<bool, BackupError>> {
        Box::pin(async { Ok(true) })
    }
}

#[tokio::test]
#[ignore = "requires disposable CITADEL_PLATFORM_DATABASE_URL"]
async fn committed_container_deletion_recomputes_authorized_surviving_stack_coverage() {
    use citadel_adapters::persistence::postgres::platforms::status::container_event_committed;
    let f = fixture().await;
    let mut reader = super::realtime_groups::reader(&f);
    reader.backup_coverage = Some(Arc::new(BackupCoverageService {
        backups: Arc::new(BackupService::new(reader.backups.clone(), Arc::new(NoExecution),
            Arc::new(citadel_adapters::persistence::postgres::backups::source_planner::PostgresBackupSourcePlanner::new(f.pool.clone())),
            Duration::minutes(5),
            Arc::new(citadel_adapters::security::identity::backup_authorization::IdentityBackupRunAuthorizer::new(reader.identity.clone())),
        ).with_entitlements(Arc::new(ScheduledBackups))),
        identity: reader.identity.clone(),
        volume_coverage: f.lookup_state.platforms.volume_coverage.clone(),
    }));
    sqlx::query("UPDATE platforms SET platformdescriptor=$2 WHERE id=$1")
        .bind(f.platform_id)
        .bind(json!({"$type":"DockerStandalone"}))
        .execute(&f.pool)
        .await
        .unwrap();
    let [stack, release, secret, repo, policy, run] = std::array::from_fn(|_| Uuid::now_v7());
    sqlx::query("INSERT INTO stacks(id,name,createdbyactorid,stacksource,driftpolicy,stackupdatestate,controlstate) VALUES($1,'coverage-stack',$2,'WebEditor','{}','{}','Idle')")
        .bind(stack).bind(SYSTEM_ACTOR_ID).execute(&f.pool).await.unwrap();
    sqlx::query("INSERT INTO stackreleases(id,stackid,platformid,createdbyactorid,spec,status,version) VALUES($1,$2,$3,$4,'{}','Degraded','1')")
        .bind(release).bind(stack).bind(f.platform_id).bind(SYSTEM_ACTOR_ID).execute(&f.pool).await.unwrap();
    sqlx::query("UPDATE stacks SET currentstackreleaseid=$1 WHERE id=$2")
        .bind(release)
        .bind(stack)
        .execute(&f.pool)
        .await
        .unwrap();
    for name in ["a", "b"] {
        sqlx::query("INSERT INTO containers(id,stackid,platformid,dockercontainerid,name,stack,dockerimageid,created,updated,state,ports,mounts) VALUES($1,$2,$3,$4,$4,'coverage-stack','image',1,1,'exited','[]',$5)")
            .bind(Uuid::now_v7()).bind(stack).bind(f.platform_id).bind(name).bind(json!([{"type":"volume","name":name}])).execute(&f.pool).await.unwrap();
    }
    sqlx::query("INSERT INTO secretdefinitions(id,name,providertype) VALUES($1,'coverage-password','InternalEncrypted')")
        .bind(secret).execute(&f.pool).await.unwrap();
    sqlx::query("INSERT INTO backuprepositories(id,name,normalizedname,passwordsecretid,spec,status,type,createdbyactorid) VALUES($1,'coverage','coverage',$2,'{}','Ready','FileSystem',$3)")
        .bind(repo).bind(secret).bind(SYSTEM_ACTOR_ID).execute(&f.pool).await.unwrap();
    sqlx::query("INSERT INTO backuppolicies(id,name,normalizedname,backuprepositoryid,createdbyactorid,runasactorid,source,enabled,cron) VALUES($1,'daily','daily',$2,$3,$3,$4,true,'0 0 * * *')")
        .bind(policy).bind(repo).bind(SYSTEM_ACTOR_ID).bind(json!({"$type":"DockerVolume","platformId":f.platform_id,"volumeName":"a"})).execute(&f.pool).await.unwrap();
    sqlx::query("INSERT INTO backupruns(id,backuppolicyid,backuprepositoryid,policynamesnapshot,repositorytypesnapshot,snapshotavailability,sourcesnapshot,status,trigger,triggeredbyactorid,completedat) SELECT $1,id,backuprepositoryid,name,'FileSystem','Available',source,'Succeeded','Schedule',createdbyactorid,CURRENT_TIMESTAMP FROM backuppolicies WHERE id=$2")
        .bind(run).bind(policy).execute(&f.pool).await.unwrap();
    sqlx::query("INSERT INTO backuprunitems(id,backuprunid,platformid,volumename,status,resticsnapshotid,completedat) VALUES($1,$2,$3,'a','Succeeded','snapshot',CURRENT_TIMESTAMP)")
        .bind(Uuid::now_v7()).bind(run).bind(f.platform_id).execute(&f.pool).await.unwrap();
    let group = Group::parse(&format!("containers:{}", f.platform_id)).unwrap();
    let key = stack.to_string();
    let joined = reader.read(&f.administrator, &group, None).await.unwrap();
    let full = &joined.events[0].arguments[0];
    assert_eq!(
        full["stackBackupCoverage"][&key]["status"], "Warning",
        "{full}"
    );
    assert_eq!(
        full["containers"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|c| c["stackId"] == key)
            .count(),
        2
    );

    let limited = super::lookup::subject(&f).await;
    super::lookup::grant(
        &f,
        limited.actor_id.value(),
        citadel_primitives::ResourceType::Platform,
        f.platform_id,
        0,
    )
    .await;
    let limited_join = reader.read(&limited, &group, None).await.unwrap();
    assert!(
        !serde_json::to_string(&limited_join.events)
            .unwrap()
            .contains(&key)
    );
    assert!(
        !serde_json::to_string(&limited_join.events)
            .unwrap()
            .contains("coverage-stack")
    );
    let hub = citadel_server::realtime::RealtimeHub::new(
        8,
        Arc::new(citadel_server::metrics::Metrics::default()),
    );
    let mut events = hub.subscribe();
    for name in ["b", "a"] {
        let committed = container_event_committed(
            &f.pool,
            f.platform_id,
            None,
            name,
            None,
            None,
            chrono::Utc::now().timestamp(),
        )
        .await
        .unwrap();
        assert_eq!(committed.affected_stack(), Some(stack));
        hub.publish_container_observation(
            f.platform_id,
            "destroy",
            name.into(),
            committed.affected_stack(),
        );
        let event = events.recv().await.unwrap();
        let snapshot = reader
            .read(&f.administrator, &group, Some(&event))
            .await
            .unwrap();
        assert_eq!(snapshot.events.len(), 1);
        assert_eq!(snapshot.events[0].target, "ContainersChanged");
        let patch = &snapshot.events[0].arguments[0];
        assert_eq!(patch["containerId"], name);
        assert_eq!(patch["containers"], json!([]));
        assert_eq!(patch["stackIds"], json!([stack]));
        if name == "b" {
            assert_eq!(
                patch["stackBackupCoverage"][&key]["status"], "Protected",
                "{patch}"
            );
        }
        let status: String = sqlx::query_scalar("SELECT status FROM stackreleases WHERE id=$1")
            .bind(release)
            .fetch_one(&f.pool)
            .await
            .unwrap();
        assert_eq!(status, "Degraded");
        let limited_patch = reader.read(&limited, &group, Some(&event)).await.unwrap();
        assert!(
            !serde_json::to_string(&limited_patch.events)
                .unwrap()
                .contains(&key)
        );
    }
    super::realtime_groups::cleanup(f).await;
}
