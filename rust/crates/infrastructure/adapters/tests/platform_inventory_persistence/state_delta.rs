use super::*;
use citadel_adapters::persistence::postgres::platforms::{
    runtime_index::RuntimeIdentityIndex, status::container_state_deltas_committed,
};
use citadel_platforms::jobs::{
    ContainerLifecycleState as State, ContainerStateDelta, ProjectionKind, ProjectionWrite,
    SnapshotGeneration,
};
use sqlx::Row;

#[tokio::test]
#[ignore = "requires CITADEL_PHASE4_DATABASE_URL"]
async fn lifecycle_deltas_preserve_metadata_ordering_scope_and_skip_parent_tables() {
    let url = std::env::var("CITADEL_PHASE4_DATABASE_URL").unwrap();
    MigrationRunner::migrate(&url).await.unwrap();
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .connect(&url)
        .await
        .unwrap();
    let platform = Uuid::now_v7();
    sqlx::query("INSERT INTO platforms(id,name,address,connectortype,cpucount,imagecount,memtotal,networkcount,volumecount,platformdescriptor,status) VALUES($1,$1::text,$1::text,'Local',1,0,1024,0,0,'{\"$type\":\"Docker\"}','Online')")
        .bind(platform).execute(&pool).await.unwrap();
    for (docker, node) in [("first", None), ("second", None), ("first", Some("worker"))] {
        sqlx::query("INSERT INTO containers(id,platformid,dockernodeid,dockercontainerid,dockerimageid,name,created,updated,state,ports,projectionobservedat) VALUES($1,$2,$3,$4,'image','preserved',1,10,'Running','[{\"privatePort\":80}]',10)")
            .bind(Uuid::now_v7()).bind(platform).bind(node).bind(docker).execute(&pool).await.unwrap();
    }
    let index = RuntimeIdentityIndex::attach(pool.clone());
    index.rebuild().await.unwrap();
    // Force a stale UUID hint; full runtime identity must still recover the row.
    let replacement = Uuid::now_v7();
    sqlx::query("UPDATE containers SET id=$1 WHERE platformid=$2 AND dockercontainerid='first' AND dockernodeid IS NULL")
        .bind(replacement).bind(platform).execute(&pool).await.unwrap();
    let generation = SnapshotGeneration::capture(platform, None, ProjectionKind::Containers).await;
    let mut blocked_parents = pool.begin().await.unwrap();
    sqlx::query("LOCK TABLE images,stacks,stackreleases,deployments IN ACCESS EXCLUSIVE MODE")
        .execute(&mut *blocked_parents)
        .await
        .unwrap();
    let deltas = [
        ContainerStateDelta {
            docker_id: "first".into(),
            state: State::Exited,
            observed_at: 20,
            observed_at_millis: 20000,
        },
        ContainerStateDelta {
            docker_id: "second".into(),
            state: State::Paused,
            observed_at: 20,
            observed_at_millis: 20000,
        },
        ContainerStateDelta {
            docker_id: "unknown".into(),
            state: State::Running,
            observed_at: 20,
            observed_at_millis: 20000,
        },
    ];
    let results = tokio::time::timeout(
        std::time::Duration::from_secs(3),
        container_state_deltas_committed(&pool, platform, None, &deltas),
    )
    .await
    .expect("state-only unmanaged writes must not read any parent/image table")
    .unwrap();
    assert!(results[0].changed && results[1].changed);
    assert_eq!(results[0].patch.as_ref().unwrap().id, replacement);
    assert_eq!(
        results[0].patch.as_ref().unwrap().state.as_deref(),
        Some("Exited")
    );
    assert_eq!(
        results[0].patch.as_ref().unwrap().control_state.as_deref(),
        Some("Idle")
    );
    assert!(results[2].patch.is_none());
    assert_eq!(results[0].container_id, Some(replacement));
    assert_eq!(index.lookup(platform, None, "first"), Some(replacement));
    assert!(!results[2].accepted);
    assert!(
        results
            .iter()
            .all(|r| r.stack_id.is_none() && r.deployment_id.is_none() && r.operation_id.is_none())
    );
    blocked_parents.rollback().await.unwrap();
    let write = ProjectionWrite::begin(platform, None, ProjectionKind::Containers).await;
    assert!(
        !generation.matches(&write),
        "a committed state delta invalidates an older inventory read"
    );
    drop(write);

    let row = sqlx::query(
        "SELECT name,dockerimageid,ports,state,rowversion,updated FROM containers WHERE id=$1",
    )
    .bind(replacement)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(row.get::<String, _>("name"), "preserved");
    assert_eq!(row.get::<String, _>("dockerimageid"), "image");
    assert_eq!(
        row.get::<serde_json::Value, _>("ports"),
        json!([{"privatePort":80}])
    );
    assert_eq!(row.get::<String, _>("state"), "Exited");
    assert_eq!(row.get::<i64, _>("updated"), 20);
    let version = row.get::<i64, _>("rowversion");
    // A stale event and a newer semantic no-op preserve the revision and metadata.
    for (state, observed_at) in [
        (State::Running, 19),
        (State::Exited, 21),
        (State::Exited, 21),
    ] {
        let result = container_state_deltas_committed(
            &pool,
            platform,
            None,
            &[ContainerStateDelta {
                docker_id: "first".into(),
                state,
                observed_at,
                observed_at_millis: observed_at.saturating_mul(1000),
            }],
        )
        .await
        .unwrap();
        assert!(result[0].accepted && !result[0].changed);
    }
    let stored: (String, i64, i64, i64) = sqlx::query_as(
        "SELECT state,rowversion,updated,projectionobservedat FROM containers WHERE id=$1",
    )
    .bind(replacement)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(stored, ("Exited".into(), version, 20, 21));
    assert_eq!(
        sqlx::query_scalar::<_, String>(
            "SELECT state FROM containers WHERE platformid=$1 AND dockernodeid='worker'"
        )
        .bind(platform)
        .fetch_one(&pool)
        .await
        .unwrap(),
        "Running"
    );
    sqlx::query(
        "UPDATE containers SET projectionstalesince=22,projectionstalereason='gap' WHERE id=$1",
    )
    .bind(replacement)
    .execute(&pool)
    .await
    .unwrap();
    assert!(
        container_state_deltas_committed(
            &pool,
            platform,
            None,
            &[ContainerStateDelta {
                docker_id: "first".into(),
                state: State::Exited,
                observed_at: 22,
                observed_at_millis: 22000,
            }]
        )
        .await
        .unwrap()[0]
            .changed
    );
    assert!(
        container_state_deltas_committed(&pool, Uuid::now_v7(), None, &deltas[..1])
            .await
            .unwrap()
            .iter()
            .all(|r| !r.accepted)
    );
    sqlx::query("DELETE FROM platforms WHERE id=$1")
        .bind(platform)
        .execute(&pool)
        .await
        .unwrap();
    pool.close().await;
}

fn iterations(family: &str) -> u64 {
    let mut metrics = String::new();
    citadel_runtime::runtime_metrics::render_runtime_metrics(&mut metrics);
    let prefix = format!("citadel_runtime_iterations_total{{family=\"{family}\"}} ");
    metrics
        .lines()
        .find_map(|line| line.strip_prefix(&prefix))
        .unwrap()
        .parse()
        .unwrap()
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE4_DATABASE_URL; run with --test-threads=1 for process counters"]
async fn six_deltas_reconcile_unique_external_parents_and_defer_owned_effects() {
    let url = std::env::var("CITADEL_PHASE4_DATABASE_URL").unwrap();
    MigrationRunner::migrate(&url).await.unwrap();
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&url)
        .await
        .unwrap();
    for kind in ["stack", "deployment", "unmanaged", "owned"] {
        let platform = Uuid::now_v7();
        let stack = Uuid::now_v7();
        let deployment = Uuid::now_v7();
        let operation = Uuid::now_v7();
        let owned = kind == "owned";
        let has_stack = kind == "stack" || owned;
        let has_deployment = kind == "deployment" || owned;
        sqlx::query("INSERT INTO platforms(id,name,address,connectortype,cpucount,imagecount,memtotal,networkcount,volumecount,platformdescriptor,status) VALUES($1,$1::text,$1::text,'Local',1,0,1024,0,0,'{\"$type\":\"Docker\"}','Online')")
            .bind(platform).execute(&pool).await.unwrap();
        if has_stack {
            sqlx::query("INSERT INTO stacks(id,name,createdbyactorid,stacksource,driftpolicy,stackupdatestate) VALUES($1,$1::text,$2,'WebEditor','{}','{}')")
                .bind(stack).bind(citadel_identity::SYSTEM_ACTOR_ID).execute(&pool).await.unwrap();
            sqlx::query("INSERT INTO stackreleases(id,createdbyactorid,platformid,spec,stackid,status,version) VALUES($1,$2,$3,'{}',$1,'Healthy','1')")
                .bind(stack).bind(citadel_identity::SYSTEM_ACTOR_ID).bind(platform).execute(&pool).await.unwrap();
            sqlx::query("UPDATE stacks SET currentstackreleaseid=$1 WHERE id=$1")
                .bind(stack)
                .execute(&pool)
                .await
                .unwrap();
        }
        if has_deployment {
            sqlx::query("INSERT INTO deployments(id,name,platformid,spec,status,createdbyactorid) VALUES($1,$1::text,$2,'{}','Healthy',$3)")
                .bind(deployment).bind(platform).bind(citadel_identity::SYSTEM_ACTOR_ID).execute(&pool).await.unwrap();
        }
        if owned {
            for table in ["stacks", "deployments"] {
                sqlx::query(if table == "stacks" {"UPDATE stacks SET controlstate='Processing',containeroperationid=$2,controltriggeredby=$3 WHERE id=$1"} else {"UPDATE deployments SET controlstate='Processing',containeroperationid=$2,controltriggeredby=$3 WHERE id=$1"})
                    .bind(if table == "stacks" {stack} else {deployment}).bind(operation).bind(citadel_identity::SYSTEM_ACTOR_ID).execute(&pool).await.unwrap();
            }
        }
        for i in 0..6 {
            sqlx::query("INSERT INTO containers(id,platformid,dockercontainerid,dockerimageid,name,created,updated,state,ports,projectionobservedat,stackid,deploymentid,controlstate,containeroperationid) VALUES($1,$2,$3,'image',$3,1,10,'Running','[]',10,$4,$5,$6,$7)")
                .bind(Uuid::now_v7()).bind(platform).bind(format!("batch-{i}"))
                .bind(has_stack.then_some(stack)).bind(has_deployment.then_some(deployment))
                .bind(if owned {"Processing"} else {"Idle"}).bind(owned.then_some(operation)).execute(&pool).await.unwrap();
        }
        let mut listener = sqlx::postgres::PgListener::connect(&url).await.unwrap();
        listener.listen("citadel_stack_drift").await.unwrap();
        let mut lock = pool.begin().await.unwrap();
        if owned || kind == "unmanaged" {
            sqlx::query("LOCK TABLE stacks,stackreleases,deployments IN ACCESS EXCLUSIVE MODE")
                .execute(&mut *lock)
                .await
                .unwrap();
        }
        let before = [
            iterations("ContainerStateDeltaBatch"),
            iterations("ContainerParentReconcileStack"),
            iterations("ContainerParentReconcileDeployment"),
        ];
        let deltas: Vec<_> = (0..6)
            .map(|i| ContainerStateDelta {
                docker_id: format!("batch-{i}"),
                state: State::Exited,
                observed_at: 20,
                observed_at_millis: 20000,
            })
            .collect();
        let results = tokio::time::timeout(
            std::time::Duration::from_secs(3),
            container_state_deltas_committed(&pool, platform, None, &deltas),
        )
        .await
        .expect("unmanaged/operation-owned deltas must never access parent tables")
        .unwrap();
        lock.rollback().await.unwrap();
        assert!(
            results
                .iter()
                .all(|r| r.changed && r.accepted && r.defer_parent_effects == owned)
        );
        assert_eq!(iterations("ContainerStateDeltaBatch") - before[0], 1);
        assert_eq!(
            iterations("ContainerParentReconcileStack") - before[1],
            u64::from(kind == "stack")
        );
        assert_eq!(
            iterations("ContainerParentReconcileDeployment") - before[2],
            u64::from(kind == "deployment")
        );
        if kind == "stack" {
            let notification =
                tokio::time::timeout(std::time::Duration::from_secs(1), listener.recv())
                    .await
                    .unwrap()
                    .unwrap();
            assert_eq!(notification.payload(), stack.to_string());
        }
        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(30), listener.recv())
                .await
                .is_err(),
            "unexpected/duplicate drift for {kind}"
        );
        for (parent, table) in [(stack, "stacks"), (deployment, "deployments")] {
            if (table == "stacks" && !has_stack) || (table == "deployments" && !has_deployment) {
                continue;
            }
            let status: String = if table == "stacks" {
                sqlx::query_scalar("SELECT status FROM stackreleases WHERE id=$1")
                    .bind(parent)
                    .fetch_one(&pool)
                    .await
                    .unwrap()
            } else {
                sqlx::query_scalar("SELECT status FROM deployments WHERE id=$1")
                    .bind(parent)
                    .fetch_one(&pool)
                    .await
                    .unwrap()
            };
            assert_eq!(status, if owned { "Healthy" } else { "Stopped" });
            let activities: i64 =
                sqlx::query_scalar("SELECT count(*) FROM activityevents WHERE resourceid=$1")
                    .bind(parent)
                    .fetch_one(&pool)
                    .await
                    .unwrap();
            assert_eq!(activities, if owned { 0 } else { 1 });
        }
        let rows: Vec<(String, String, Option<Uuid>)> = sqlx::query_as(
            "SELECT state,controlstate,containeroperationid FROM containers WHERE platformid=$1",
        )
        .bind(platform)
        .fetch_all(&pool)
        .await
        .unwrap();
        assert_eq!(rows.len(), 6);
        assert!(rows.iter().all(|(state, control, op)| state == "Exited"
            && control == if owned { "Processing" } else { "Idle" }
            && *op == owned.then_some(operation)));
        // Replays neither reconcile parents nor insert duplicate activities/drift.
        let before = [
            iterations("ContainerParentReconcileStack"),
            iterations("ContainerParentReconcileDeployment"),
        ];
        assert!(
            container_state_deltas_committed(&pool, platform, None, &deltas)
                .await
                .unwrap()
                .iter()
                .all(|r| !r.changed)
        );
        assert_eq!(
            before,
            [
                iterations("ContainerParentReconcileStack"),
                iterations("ContainerParentReconcileDeployment")
            ]
        );
        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(30), listener.recv())
                .await
                .is_err()
        );
    }
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE4_DATABASE_URL; run with --test-threads=1"]
async fn operation_finish_and_stale_recovery_create_parent_activities_once() {
    use citadel_adapters::persistence::postgres::platforms::containers::repository::PostgresContainerRepository;
    use citadel_platforms::containers::{
        ContainerAction, ContainerMutationRuntime, ContainerRepository, ContainerTaskSpawner,
    };
    use futures_util::future::BoxFuture;
    use std::sync::Arc;
    use tokio_util::sync::CancellationToken;

    struct ExitedRuntime;
    impl ContainerMutationRuntime for ExitedRuntime {
        fn mutate<'a>(
            &'a self,
            _: &'a citadel_platforms::containers::ContainerTarget,
            _: ContainerAction,
            _: &'a CancellationToken,
        ) -> BoxFuture<'a, Result<(), citadel_platforms::RuntimeCapabilityError>> {
            Box::pin(async { unreachable!("recovery is read-only") })
        }
        fn observe<'a>(
            &'a self,
            _: &'a citadel_platforms::containers::ContainerTarget,
            _: &'a CancellationToken,
        ) -> BoxFuture<'a, Result<Option<String>, citadel_platforms::RuntimeCapabilityError>>
        {
            Box::pin(async { Ok(Some("exited".into())) })
        }
    }
    struct Tasks;
    impl ContainerTaskSpawner for Tasks {
        fn spawn(
            &self,
            _: BoxFuture<'static, Result<(), citadel_platforms::RuntimeCapabilityError>>,
        ) -> bool {
            false
        }
        fn shutdown_token(&self) -> CancellationToken {
            CancellationToken::new()
        }
    }

    let url = std::env::var("CITADEL_PHASE4_DATABASE_URL").unwrap();
    MigrationRunner::migrate(&url).await.unwrap();
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&url)
        .await
        .unwrap();
    let platform = Uuid::now_v7();
    sqlx::query("INSERT INTO platforms(id,name,address,connectortype,cpucount,imagecount,memtotal,networkcount,volumecount,platformdescriptor,status) VALUES($1,$1::text,$1::text,'Local',1,0,1024,0,0,'{\"$type\":\"Docker\"}','Online')")
        .bind(platform).execute(&pool).await.unwrap();
    let actor = citadel_identity::SYSTEM_ACTOR_ID;
    let repository = PostgresContainerRepository::new(pool.clone());

    async fn seed_claim(
        pool: &sqlx::PgPool,
        platform: Uuid,
        actor: Uuid,
        stale: bool,
    ) -> (Uuid, Uuid, Uuid) {
        let operation = Uuid::now_v7();
        let stack = Uuid::now_v7();
        let deployment = Uuid::now_v7();
        sqlx::query("INSERT INTO stacks(id,name,createdbyactorid,stacksource,driftpolicy,stackupdatestate,controlstate,controlstartedat,controltriggeredby,containeroperationid) VALUES($1,$1::text,$2,'WebEditor','{}','{}','Processing',$3,$2,$4)")
            .bind(stack).bind(actor).bind(if stale {-600_i64} else {chrono::Utc::now().timestamp()}).bind(operation).execute(pool).await.unwrap();
        sqlx::query("INSERT INTO stackreleases(id,createdbyactorid,platformid,spec,stackid,status,version) VALUES($1,$2,$3,'{}',$1,'Healthy','1')")
            .bind(stack).bind(actor).bind(platform).execute(pool).await.unwrap();
        sqlx::query("UPDATE stacks SET currentstackreleaseid=$1 WHERE id=$1")
            .bind(stack)
            .execute(pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO deployments(id,name,platformid,spec,status,createdbyactorid,controlstate,controlstartedat,controltriggeredby,containeroperationid) VALUES($1,$1::text,$2,'{}','Healthy',$3,'Processing',$4,$3,$5)")
            .bind(deployment).bind(platform).bind(actor).bind(if stale {-600_i64} else {chrono::Utc::now().timestamp()}).bind(operation).execute(pool).await.unwrap();
        for i in 0..3 {
            sqlx::query("INSERT INTO containers(id,platformid,dockercontainerid,dockerimageid,name,created,updated,state,ports,stackid,deploymentid,controlstate,controlstartedat,controltriggeredby,containeroperationid) VALUES($1,$2,$3,'image',$3,1,1,'Running','[]',$4,$5,'Processing',$6,$7,$8)")
                .bind(Uuid::now_v7()).bind(platform).bind(format!("{operation}-{i}"))
                .bind(stack).bind(deployment).bind(if stale {-600_i64} else {chrono::Utc::now().timestamp()})
                .bind(actor).bind(operation).execute(pool).await.unwrap();
        }
        (operation, stack, deployment)
    }

    let (normal, stack, deployment) = seed_claim(&pool, platform, actor, false).await;
    sqlx::query("UPDATE containers SET state='Exited' WHERE containeroperationid=$1")
        .bind(normal)
        .execute(&pool)
        .await
        .unwrap();
    let completed = repository.finish_committed(normal).await.unwrap();
    assert_eq!(completed.stack_ids, [stack]);
    assert_eq!(completed.deployment_ids, [deployment]);
    assert_eq!(completed.container_patches.len(), 3);
    assert!(completed.container_patches.iter().all(|patch| {
        patch.platform_id == platform
            && patch.state.as_deref() == Some("Exited")
            && patch.control_state.as_deref() == Some("Idle")
    }));
    assert!(
        repository
            .finish_committed(normal)
            .await
            .unwrap()
            .stack_ids
            .is_empty()
    );
    let activities: i64 =
        sqlx::query_scalar("SELECT count(*) FROM activityevents WHERE resourceid=ANY($1)")
            .bind(vec![stack, deployment])
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(activities, 2);

    let (stale_operation, stale_stack, stale_deployment) =
        seed_claim(&pool, platform, actor, true).await;
    let service = citadel_platforms::containers::ContainerMutationService::new(
        Arc::new(repository.clone()),
        Arc::new(ExitedRuntime),
        Arc::new(Tasks),
    );
    service.reconcile(&CancellationToken::new()).await.unwrap();
    let first: Vec<(Uuid, String)> = sqlx::query_as("SELECT resourceid,eventtype FROM activityevents WHERE resourceid=ANY($1) ORDER BY resourceid,eventtype")
        .bind(vec![stale_stack, stale_deployment]).fetch_all(&pool).await.unwrap();
    assert_eq!(first.len(), 2);
    assert!(first.iter().any(|(_, kind)| kind == "StackStopped"));
    assert!(first.iter().any(|(_, kind)| kind == "DeploymentStopped"));
    service.reconcile(&CancellationToken::new()).await.unwrap();
    let after: i64 =
        sqlx::query_scalar("SELECT count(*) FROM activityevents WHERE resourceid=ANY($1)")
            .bind(vec![stale_stack, stale_deployment])
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(after, 2);
    let claims: i64 =
        sqlx::query_scalar("SELECT count(*) FROM containers WHERE containeroperationid=ANY($1)")
            .bind(vec![normal, stale_operation])
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(claims, 0);
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE4_DATABASE_URL"]
async fn container_command_admission_uses_uuid_index_and_cached_authorization() {
    use citadel_adapters::persistence::postgres::platforms::{
        containers::repository::PostgresContainerRepository, runtime_index::RuntimeIdentityIndex,
    };
    use citadel_platforms::containers::{
        ContainerAction, ContainerRepository, ContainerSelectionKind,
    };
    use citadel_primitives::{ActorId, PermissionLevel, ResourceType};

    let url = std::env::var("CITADEL_PHASE4_DATABASE_URL").unwrap();
    MigrationRunner::migrate(&url).await.unwrap();
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .connect(&url)
        .await
        .unwrap();
    let platform = Uuid::now_v7();
    let actor = Uuid::now_v7();
    sqlx::query("INSERT INTO actors(id,isenabled,type) VALUES($1,true,'User')")
        .bind(actor)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO platforms(id,name,address,connectortype,cpucount,imagecount,memtotal,networkcount,volumecount,platformdescriptor,status) VALUES($1,$1::text,$1::text,'Local',1,0,1024,0,0,'{\"$type\":\"Docker\"}','Online')")
        .bind(platform)
        .execute(&pool)
        .await
        .unwrap();
    let ids = [Uuid::now_v7(), Uuid::now_v7(), Uuid::now_v7()];
    let prefix = Uuid::now_v7().simple().to_string()[..8].to_owned();
    let docker_ids = [
        format!("{prefix}def456"),
        format!("{prefix}abc12345"),
        format!("{prefix}abc12399"),
    ];
    for (id, docker) in ids.into_iter().zip(&docker_ids) {
        sqlx::query("INSERT INTO containers(id,platformid,dockercontainerid,dockerimageid,name,created,updated,state,ports,projectionobservedat) VALUES($1,$2,$3,'image','fixture',1,1,'Exited','[]',1)")
            .bind(id)
            .bind(platform)
            .bind(docker)
            .execute(&pool)
            .await
            .unwrap();
    }
    sqlx::query("INSERT INTO resourceaccesses(id,actorid,permissionlevel,resourceid,resourcetype,specificpermissions) VALUES($1,$2,$3,$4,$5,0)")
        .bind(Uuid::now_v7())
        .bind(actor)
        .bind(PermissionLevel::Write as i32)
        .bind(platform)
        .bind(ResourceType::Platform as i32)
        .execute(&pool)
        .await
        .unwrap();

    let index = RuntimeIdentityIndex::attach(pool.clone());
    index.rebuild().await.unwrap();
    let repository = PostgresContainerRepository::new(pool.clone());
    let sql_before = iterations("ContainerIdResolveQuery");
    assert_eq!(
        repository.resolve_ids(&[ids[0].to_string()]).await.unwrap(),
        vec![ids[0]],
        "UUID parsing must leave existence validation to the claim transaction"
    );
    assert_eq!(iterations("ContainerIdResolveQuery"), sql_before);
    assert_eq!(
        repository
            .resolve_ids(&[prefix.clone() + "def"])
            .await
            .unwrap(),
        vec![ids[0]],
        "a unique Docker prefix should resolve from the committed runtime index"
    );
    assert!(matches!(
        repository.resolve_ids(&[prefix.clone() + "abc"]).await,
        Err(error) if error.kind == citadel_platforms::RuntimeErrorKind::Conflict
    ));
    assert!(matches!(
        repository.resolve_ids(&[prefix.clone() + "ffff"]).await,
        Err(error) if error.kind == citadel_platforms::RuntimeErrorKind::NotFound
    ));
    assert_eq!(iterations("ContainerIdResolveQuery") - sql_before, 1);

    let claim = repository
        .claim_selection(
            ActorId::new(actor),
            false,
            &[ids[0]],
            ContainerAction::Start,
            ContainerSelectionKind::Containers,
        )
        .await
        .unwrap();
    repository
        .finish_committed(claim.operation_id)
        .await
        .unwrap();
    let scopes_before = iterations("AuthorizationScopeQuery");
    let resources_before = iterations("AuthorizationResourceQuery");
    let claim = repository
        .claim_selection(
            ActorId::new(actor),
            false,
            &[ids[0]],
            ContainerAction::Start,
            ContainerSelectionKind::Containers,
        )
        .await
        .unwrap();
    repository
        .finish_committed(claim.operation_id)
        .await
        .unwrap();
    assert_eq!(iterations("AuthorizationScopeQuery"), scopes_before);
    assert_eq!(iterations("AuthorizationResourceQuery"), resources_before);

    let scopes_before_admin = iterations("AuthorizationScopeQuery");
    let claim = repository
        .claim_selection(
            ActorId::new(actor),
            true,
            &[ids[0]],
            ContainerAction::Start,
            ContainerSelectionKind::Containers,
        )
        .await
        .unwrap();
    repository
        .finish_committed(claim.operation_id)
        .await
        .unwrap();
    assert_eq!(iterations("AuthorizationScopeQuery"), scopes_before_admin);
    sqlx::query("DELETE FROM resourceaccesses WHERE resourceid=$1 AND actorid=$2")
        .bind(platform)
        .bind(actor)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM platforms WHERE id=$1")
        .bind(platform)
        .execute(&pool)
        .await
        .unwrap();
    pool.close().await;
}
