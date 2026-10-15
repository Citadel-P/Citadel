use super::*;
use citadel_adapters::persistence::postgres::{
    deployments::PostgresDeploymentRepository,
    identity::authentication::store::StaticEntitlementService, stacks::PostgresStackRepository,
};
use citadel_deployments::DeploymentRepository;
use citadel_stacks::{
    ImageUpdateState, RecreateStackOnNewImageState, StackRepository, StackUpdateState,
};

struct Fixture {
    pool: sqlx::PgPool,
    alerts: PostgresAlertRepository,
    actor: ActorId,
    platform: Uuid,
    notifications: Arc<AtomicUsize>,
}
impl Fixture {
    async fn new() -> Self {
        let url = std::env::var("CITADEL_EXECUTION_DATABASE_URL").unwrap();
        MigrationRunner::migrate(&url).await.unwrap();
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(&url)
            .await
            .unwrap();
        let actor = ActorId::new(Uuid::from_u128(1));
        let platform = Uuid::now_v7();
        sqlx::query("INSERT INTO platforms(id,address,connectortype,cpucount,imagecount,memtotal,name,networkcount,platformdescriptor,status,volumecount) VALUES($1,$2,'Local',0,0,0,$2,0,'{\"$type\":\"Docker\"}','Online',0)")
            .bind(platform).bind(format!("http://alert-updates-{platform}.test")).execute(&pool).await.unwrap();
        let notifications = Arc::new(AtomicUsize::new(0));
        let counter = notifications.clone();
        let alerts = PostgresAlertRepository::new(pool.clone())
            .with_entitlements(Arc::new(StaticEntitlementService::new(true)))
            .with_change_notifier(Arc::new(move || {
                counter.fetch_add(1, Ordering::SeqCst);
            }));
        Self {
            pool,
            alerts,
            actor,
            platform,
            notifications,
        }
    }
    async fn rule(&self, id: Uuid, kind: &str) -> Uuid {
        self.alerts
            .create_rule(
                self.actor,
                &AlertRuleConfiguration {
                    name: format!("{kind}-{}", Uuid::now_v7()),
                    description: None,
                    alert_type: kind.into(),
                    severity: citadel_alerts::AlertSeverity::Critical,
                    cooldown_seconds: None,
                    required_matches: None,
                    threshold: None,
                    status: citadel_alerts::AlertRuleStatus::Enabled,
                    channel_ids: vec![],
                    quiet_hours: vec![],
                    limited_to: vec![json!({"resourceId":id})],
                },
            )
            .await
            .unwrap()
            .id
    }
    async fn event(
        &self,
        id: Uuid,
        kind: &str,
        component: &str,
        info: serde_json::Value,
    ) -> (citadel_alerts::AlertEvent, AlertObservation) {
        let observation = AlertObservation {
            alert_type: kind.into(),
            resource_id: id,
            resource_name: "fixture".into(),
            resource_type: if kind.starts_with("Stack") {
                "Stack"
            } else {
                "Deployment"
            }
            .into(),
            deduplication_component: component.into(),
            observed_at: Utc::now(),
            value: None,
            matched: true,
            info,
        };
        let event = self
            .alerts
            .process_event(&observation)
            .await
            .unwrap()
            .expect("pending update should raise an alert");
        (event, observation)
    }
    async fn drain(&self) {
        // The queue is durable; evaluation does not need the original repository instance.
        let resumed = PostgresAlertRepository::new(self.pool.clone()).with_change_notifier({
            let counter = self.notifications.clone();
            Arc::new(move || {
                counter.fetch_add(1, Ordering::SeqCst);
            })
        });
        for _ in 0..100 {
            if !resumed
                .process_pending_observation(Uuid::now_v7())
                .await
                .unwrap()
            {
                return;
            }
        }
        panic!("observation queue did not drain");
    }
    async fn resolved(&self, id: Uuid, expected: bool) {
        let event = self.alerts.get_event(id).await.unwrap();
        assert_eq!(event.resolved_at.is_some(), expected);
        if expected {
            assert_eq!(event.resolved_by_actor_id, Some(Uuid::from_u128(1)));
            assert_eq!(
                event.resolution_note.as_deref(),
                Some("Update applied successfully.")
            );
        }
    }
}
fn state(web: &str, db: &str) -> StackUpdateState {
    StackUpdateState::WebEditor {
        recreate_stack_on_new_image_state: RecreateStackOnNewImageState {
            auto_update_states: [("web", web), ("db", db)]
                .into_iter()
                .map(|(service, digest)| ImageUpdateState {
                    service_name: service.into(),
                    image_name: format!("{service}:latest"),
                    current_digest: digest.into(),
                    remote_digest: Some("sha256:new".into()),
                    last_checked_at: Utc::now(),
                    update_available: digest != "sha256:new",
                })
                .collect(),
        },
    }
}
fn target(web: &str, db: &str) -> serde_json::Value {
    json!({"Updates":[{"ServiceName":"web","ImageName":"web:latest","LatestDigest":web},{"ServiceName":"db","ImageName":"db:latest","LatestDigest":db}]})
}

#[tokio::test]
#[ignore = "requires CITADEL_EXECUTION_DATABASE_URL"]
async fn stack_update_resolution_is_durable_exact_and_does_not_reopen() {
    let f = Fixture::new().await;
    let stacks = PostgresStackRepository::new(f.pool.clone());
    let stack=stacks.create(f.actor,true,&citadel_stacks::CreateStack {
        name:format!("updates-{}",Uuid::now_v7()),platform_id:f.platform,description:None,
        stack_source:citadel_stacks::StackSource::WebEditor,
        spec:serde_json::from_value(json!({"$type":"WebEditor","composeFile":"services:\n  web:\n    image: web:latest\n  db:\n    image: db:latest\n"})).unwrap(),
        drift_policy:None,tag_ids:vec![],duplicate_source:None,
    }).await.unwrap();
    let rule = f.rule(stack.id, "StackImageUpdateAvailable").await;
    f.rule(stack.id, "StackAutoDeployFailed").await;
    let (old, delayed) = f
        .event(
            stack.id,
            "StackImageUpdateAvailable",
            "target-old",
            target("sha256:new", "sha256:new"),
        )
        .await;
    let (newer, newer_observation) = f
        .event(
            stack.id,
            "StackImageUpdateAvailable",
            "target-newer",
            target("sha256:newer", "sha256:newer"),
        )
        .await;
    let (failed, _) = f
        .event(stack.id, "StackAutoDeployFailed", "failed", json!({}))
        .await;
    f.alerts.acknowledge(f.actor, &[old.id]).await.unwrap();
    stacks
        .save_update_check(f.actor, true, &stack, &state("sha256:new", "sha256:old"))
        .await
        .unwrap();
    f.drain().await;
    f.resolved(old.id, false).await; // A partial update cannot close an aggregate alert.
    assert!(
        stacks
            .save_update_check(f.actor, true, &stack, &state("sha256:new", "sha256:new"))
            .await
            .is_err()
    );
    f.drain().await;
    f.resolved(old.id, false).await; // Rejected CAS must not enqueue a clearing observation.
    let current = stacks
        .get_authorized(f.actor, true, stack.id)
        .await
        .unwrap();
    stacks
        .save_update_check(f.actor, true, &current, &state("sha256:new", "sha256:new"))
        .await
        .unwrap();
    f.resolved(old.id, false).await; // Only the committed queue worker resolves it.
    f.alerts
        .update_rule(f.actor, rule, &json!({"status":"Disabled"}))
        .await
        .unwrap();
    let before = f.notifications.load(Ordering::SeqCst);
    f.drain().await;
    assert!(f.notifications.load(Ordering::SeqCst) > before);
    f.resolved(old.id, true).await;
    assert_eq!(
        f.alerts
            .get_event(old.id)
            .await
            .unwrap()
            .acknowledged_by_actor_id,
        Some(f.actor.value())
    );
    f.resolved(newer.id, false).await;
    f.resolved(failed.id, false).await;
    f.alerts
        .update_rule(f.actor, rule, &json!({"status":"Enabled"}))
        .await
        .unwrap();
    assert!(f.alerts.process_event(&delayed).await.unwrap().is_none());
    f.resolved(old.id, true).await;

    // A clearing observation can be delayed while the same target becomes
    // pending again (for example after a rollback). Preserve the new incident.
    let current = stacks
        .get_authorized(f.actor, true, stack.id)
        .await
        .unwrap();
    stacks
        .save_update_check(
            f.actor,
            true,
            &current,
            &state("sha256:newer", "sha256:newer"),
        )
        .await
        .unwrap();
    f.alerts
        .resolve(f.actor, &[newer.id], Some("Reviewed"))
        .await
        .unwrap();
    let current = stacks
        .get_authorized(f.actor, true, stack.id)
        .await
        .unwrap();
    stacks
        .save_update_check(f.actor, true, &current, &state("sha256:new", "sha256:new"))
        .await
        .unwrap();
    let reopened = f
        .alerts
        .process_event(&newer_observation)
        .await
        .unwrap()
        .unwrap();
    assert_ne!(reopened.id, newer.id);
    // PostgreSQL timestamps an event at transaction start. A transaction that
    // waited on a workload lock can predate the clearing observation even
    // though it inserts the new occurrence afterwards.
    sqlx::query("UPDATE alertevents SET createdat=CURRENT_TIMESTAMP-INTERVAL '1 hour' WHERE id=$1")
        .bind(reopened.id)
        .execute(&f.pool)
        .await
        .unwrap();
    f.drain().await;
    f.resolved(reopened.id, false).await;
    assert_eq!(
        f.alerts
            .get_event(newer.id)
            .await
            .unwrap()
            .resolution_note
            .as_deref(),
        Some("Reviewed")
    );
    f.pool.close().await;
}

#[tokio::test]
#[ignore = "requires CITADEL_EXECUTION_DATABASE_URL"]
async fn deployment_apply_resolves_only_its_installed_digest() {
    let f = Fixture::new().await;
    let deployments = PostgresDeploymentRepository::new(f.pool.clone());
    let deployment=deployments.create(f.actor,true,&citadel_deployments::CreateDeployment {
        name:format!("update-deployment-{}",Uuid::now_v7()),platform_id:f.platform,description:None,
        spec:serde_json::from_value(json!({"image":{"$type":"External","registryId":"00000000-0000-0000-0000-000000000100","imageTag":"nginx:alpine"}})).unwrap(),tag_ids:vec![],duplicate_source:None,
    }).await.unwrap();
    f.rule(deployment.id, "DeploymentImageUpdateAvailable")
        .await;
    let (old, delayed) = f
        .event(
            deployment.id,
            "DeploymentImageUpdateAvailable",
            "sha256:new",
            json!({"LatestImage":"sha256:new"}),
        )
        .await;
    let (newer, _) = f
        .event(
            deployment.id,
            "DeploymentImageUpdateAvailable",
            "sha256:newer",
            json!({"LatestImage":"sha256:newer"}),
        )
        .await;
    let claim = deployments
        .claim_apply(f.actor, true, deployment.id)
        .await
        .unwrap();
    deployments
        .fail_apply(f.actor, &claim, "Docker failed", None, &[])
        .await
        .unwrap();
    f.drain().await;
    f.resolved(old.id, false).await;
    let claim = deployments
        .claim_apply(f.actor, true, deployment.id)
        .await
        .unwrap();
    let result = citadel_deployments::RuntimeDeploymentResult {
        docker_container_id: format!("container-{}", Uuid::now_v7()),
        docker_image_id: "image".into(),
        state: citadel_deployments::RuntimeContainerState::Running,
    };
    deployments
        .complete_apply(f.actor, &claim, &result, Some("sha256:new"), &[])
        .await
        .unwrap();
    f.drain().await;
    f.resolved(old.id, true).await;
    f.resolved(newer.id, false).await;
    assert!(f.alerts.process_event(&delayed).await.unwrap().is_none());
    // Reconciliation can also verify an applied target. Failed and cancelled
    // checks must leave the pending alert untouched.
    for status in [
        None,
        Some(citadel_primitives::AutoUpdateStatus::Failed),
        Some(citadel_primitives::AutoUpdateStatus::UpToDate),
    ] {
        let current = deployments
            .get_authorized(f.actor, true, deployment.id)
            .await
            .unwrap();
        let check = deployments
            .begin_update_check(f.actor, true, &current)
            .await
            .unwrap();
        let state = status.map(|status| citadel_primitives::AutoUpdateState {
            status,
            ..citadel_primitives::AutoUpdateState::checked(
                "sha256:newer",
                "sha256:newer",
                Utc::now(),
            )
        });
        deployments
            .complete_update_check(&check, state.as_ref())
            .await
            .unwrap();
        f.drain().await;
        f.resolved(
            newer.id,
            status == Some(citadel_primitives::AutoUpdateStatus::UpToDate),
        )
        .await;
    }
    f.pool.close().await;
}

#[tokio::test]
#[ignore = "requires CITADEL_EXECUTION_DATABASE_URL"]
async fn git_apply_resolves_matching_commit_but_not_newer_or_failed_update() {
    let f = Fixture::new().await;
    let repo = Uuid::now_v7();
    sqlx::query("INSERT INTO gitrepositories(id,createdbyactorid,defaultbranch,name,status,syncmode,url,controlstate) VALUES($1,$2,'main',$3,'Healthy','Manual','https://example.test/repo.git','Idle')")
        .bind(repo).bind(f.actor.value()).bind(repo.to_string()).execute(&f.pool).await.unwrap();
    let stacks = PostgresStackRepository::new(f.pool.clone());
    let stack=stacks.create(f.actor,true,&citadel_stacks::CreateStack {
        name:format!("git-updates-{}",Uuid::now_v7()),platform_id:f.platform,description:None,
        stack_source:citadel_stacks::StackSource::Git,
        spec:serde_json::from_value(json!({"$type":"Git","gitRepoId":repo,"branch":"main","composePaths":["compose.yml"]})).unwrap(),
        drift_policy:None,tag_ids:vec![],duplicate_source:None,
    }).await.unwrap();
    f.rule(stack.id, "StackGitUpdateAvailable").await;
    let (old, delayed) = f
        .event(
            stack.id,
            "StackGitUpdateAvailable",
            "commit-a",
            json!({"RemoteCommitSha":"a".repeat(40)}),
        )
        .await;
    let (newer, _) = f
        .event(
            stack.id,
            "StackGitUpdateAvailable",
            "commit-b",
            json!({"RemoteCommitSha":"b".repeat(40)}),
        )
        .await;
    let claim = stacks
        .claim_apply(f.actor, true, stack.id, None, None)
        .await
        .unwrap();
    stacks
        .fail_apply(f.actor, &claim, "Git failed", false)
        .await
        .unwrap();
    f.drain().await;
    f.resolved(old.id, false).await;
    let claim = stacks
        .claim_apply(f.actor, true, stack.id, None, None)
        .await
        .unwrap();
    let source=serde_json::from_value(json!({"sourceType":"Git","gitRepositoryId":repo,"branch":"main","resolvedCommitSha":"a".repeat(40),"composePaths":["compose.yml"]})).unwrap();
    stacks
        .complete_apply(
            f.actor,
            &claim,
            &citadel_stacks::StackRuntimeResult {
                status: citadel_stacks::StackReleaseStatus::Healthy,
                messages: vec![],
            },
            &[],
            Some(&source),
            None,
        )
        .await
        .unwrap();
    f.drain().await;
    f.resolved(old.id, true).await;
    f.resolved(newer.id, false).await;
    assert!(f.alerts.process_event(&delayed).await.unwrap().is_none());
    f.pool.close().await;
}
