use super::*;
use citadel_adapters::persistence::postgres::platforms::containers::repository::PostgresContainerRepository;
use citadel_platforms::{RuntimeCapabilityError, RuntimeErrorKind, containers::*};
use citadel_primitives::ResourceType;
use futures_util::future::BoxFuture;
use tokio_util::sync::CancellationToken;

type RuntimeCall = (Uuid, String, Option<String>, ContainerAction);

#[tokio::test]
#[ignore = "requires CITADEL_PHASE4_DATABASE_URL"]
async fn six_container_http_selection_sends_one_edge_mutation_then_verifies_each_target() {
    use citadel_adapters::connectors::edge::EdgeTarget;
    use citadel_contracts::citadel::{
        containers::v1::{ContainerIds, InspectContainerRequest},
        edge::v1::{EdgeCommandKind as Kind, core_envelope},
        shared_models::v1::{ContainerState, ContainerStateType, InspectContainerResponse},
    };
    use prost::Message;
    let f = fixture().await;
    let mut inventory = snapshot(f.platform_id);
    let template = inventory.containers[0].clone();
    inventory.containers = (0..6)
        .map(|i| {
            let mut container = template.clone();
            container.id = format!("batch-{i}");
            container.name = format!("batch-{i}");
            container
        })
        .collect();
    PostgresInventoryProjectionStore::new(f.pool.clone())
        .persist(&inventory)
        .await
        .unwrap();
    let ids: Vec<Uuid> = sqlx::query_scalar(
        "UPDATE containers SET dockernodeid='node-1' WHERE platformid=$1 RETURNING id",
    )
    .bind(f.platform_id)
    .fetch_all(&f.pool)
    .await
    .unwrap();
    let store = PostgresContainerRepository::new(f.pool.clone());
    let references: Vec<_> = ids.iter().map(ToString::to_string).collect();
    assert_eq!(store.resolve_ids(&references).await.unwrap(), ids);
    let mut missing = references.clone();
    missing.push(Uuid::now_v7().to_string());
    assert!(
        matches!(store.resolve_ids(&missing).await, Err(error) if error.kind == RuntimeErrorKind::NotFound)
    );
    let registry = &f.lookup_state.platforms.edge;
    let (session, mut commands) = registry
        .register(
            EdgeTarget::node(f.platform_id, "node-1".into()),
            Uuid::now_v7(),
        )
        .unwrap();
    let (wrong, mut wrong_commands) = registry
        .register(
            EdgeTarget::node(f.platform_id, "other-node".into()),
            Uuid::now_v7(),
        )
        .unwrap();
    let (response, ()) = tokio::join!(
        send_json(
            &f,
            Method::PATCH,
            "/api/v1/containers/stop",
            f.administrator.clone(),
            json!(ids)
        ),
        async {
            let mut inspected = std::collections::BTreeSet::new();
            for step in 0..7 {
                let envelope = tokio::time::timeout(StdDuration::from_secs(5), commands.recv())
                    .await
                    .unwrap()
                    .unwrap();
                let id = Uuid::parse_str(&envelope.command_id).unwrap();
                let Some(core_envelope::Body::Command(command)) = envelope.body else {
                    panic!("expected command")
                };
                if step == 0 {
                    assert_eq!(command.kind, Kind::ContainerStop as i32);
                    let mut targets = ContainerIds::decode(command.payload.as_slice())
                        .unwrap()
                        .ids;
                    targets.sort();
                    assert_eq!(
                        targets,
                        (0..6).map(|i| format!("batch-{i}")).collect::<Vec<_>>()
                    );
                } else {
                    assert_eq!(command.kind, Kind::ContainerInspect as i32);
                    let target = InspectContainerRequest::decode(command.payload.as_slice())
                        .unwrap()
                        .container_id;
                    assert!(inspected.insert(target.clone()));
                    session.output(
                        id,
                        InspectContainerResponse {
                            id: target,
                            state: Some(ContainerState {
                                status: ContainerStateType::Exited as i32,
                                ..Default::default()
                            }),
                            ..Default::default()
                        }
                        .encode_to_vec(),
                    );
                }
                session.complete(id, true);
            }
            assert_eq!(inspected.len(), 6);
        }
    );
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    assert!(commands.try_recv().is_err());
    assert!(wrong_commands.try_recv().is_err());
    let finished: i64 = sqlx::query_scalar("SELECT count(*) FROM containers WHERE id=ANY($1) AND state='Exited' AND controlstate='Idle'").bind(&ids).fetch_one(&f.pool).await.unwrap();
    assert_eq!(finished, 6);
    registry.remove(&session);
    registry.remove(&wrong);
    f.docker_server.abort();
    f.pool.close().await;
    let _ = tokio::fs::remove_file(f.docker_socket).await;
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE4_DATABASE_URL"]
async fn container_mutation_transactions_lock_platform_before_inventory_children() {
    let f = fixture().await;
    let store = PostgresContainerRepository::new(f.pool.clone());
    let actor = f.administrator.actor_id;
    let container: Uuid = sqlx::query_scalar("SELECT id FROM containers WHERE platformid=$1")
        .bind(f.platform_id)
        .fetch_one(&f.pool)
        .await
        .unwrap();
    let deployment = Uuid::now_v7();
    let stack = Uuid::now_v7();
    let release = Uuid::now_v7();
    sqlx::query("UPDATE platforms SET platformdescriptor='{\"$type\":\"Docker\"}' WHERE id=$1")
        .bind(f.platform_id)
        .execute(&f.pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO deployments(id,name,platformid,createdbyactorid,status,spec) VALUES($1,'lock-order-deployment',$2,$3,'Healthy','{}')")
        .bind(deployment).bind(f.platform_id).bind(actor.value()).execute(&f.pool).await.unwrap();
    sqlx::query("INSERT INTO stacks(id,name,createdbyactorid,stacksource,driftpolicy,stackupdatestate) VALUES($1,'lock-order-stack',$2,'WebEditor','{}','{}')")
        .bind(stack).bind(actor.value()).execute(&f.pool).await.unwrap();
    sqlx::query("INSERT INTO stackreleases(id,stackid,platformid,createdbyactorid,status,spec,version) VALUES($1,$2,$3,$4,'Healthy','{}','1')")
        .bind(release).bind(stack).bind(f.platform_id).bind(actor.value()).execute(&f.pool).await.unwrap();
    sqlx::query("UPDATE stacks SET currentstackreleaseid=$2 WHERE id=$1")
        .bind(stack)
        .bind(release)
        .execute(&f.pool)
        .await
        .unwrap();
    sqlx::query("UPDATE containers SET deploymentid=$2,stackid=$3 WHERE id=$1")
        .bind(container)
        .bind(deployment)
        .bind(stack)
        .execute(&f.pool)
        .await
        .unwrap();

    for phase in ["finish", "observe", "claim", "deployment_claim", "abandon"] {
        let claim = if matches!(phase, "claim" | "deployment_claim") {
            None
        } else {
            Some(
                store
                    .claim(actor, true, &[container], ContainerAction::Start)
                    .await
                    .unwrap(),
            )
        };
        // Inventory takes the Platform lock before updating its containers and parents.
        let mut inventory = f.pool.begin().await.unwrap();
        let inventory_pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
            .fetch_one(&mut *inventory)
            .await
            .unwrap();
        sqlx::query("SELECT id FROM platforms WHERE id=$1 FOR UPDATE")
            .bind(f.platform_id)
            .fetch_one(&mut *inventory)
            .await
            .unwrap();
        let worker = store.clone();
        let operation = tokio::spawn(async move {
            match phase {
                "claim" => worker
                    .claim(actor, true, &[container], ContainerAction::Start)
                    .await
                    .map(|_| ()),
                "deployment_claim" => worker
                    .claim_selection(
                        actor,
                        true,
                        &[deployment],
                        ContainerAction::Start,
                        ContainerSelectionKind::Deployments,
                    )
                    .await
                    .map(|_| ()),
                "observe" => {
                    let claim = claim.unwrap();
                    worker
                        .observed(claim.operation_id, &claim.targets[0], Some("running"))
                        .await
                }
                "finish" => worker.finish(claim.unwrap().operation_id).await,
                "abandon" => worker.abandon(claim.unwrap().operation_id).await,
                _ => unreachable!(),
            }
        });
        // Wait for the actual database lock, rather than depending on task timing.
        tokio::time::timeout(StdDuration::from_secs(5), async {
            loop {
                let waiting: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE datname=current_database() AND $1=ANY(pg_blocking_pids(pid)))")
                    .bind(inventory_pid).fetch_one(&f.pool).await.unwrap();
                if waiting { break; }
                assert!(!operation.is_finished(), "{phase} bypassed the Platform lock");
                tokio::time::sleep(StdDuration::from_millis(10)).await;
            }
        }).await.unwrap();
        // A waiting mutation must not hold a child lock that inventory needs.
        for (table, id) in [
            ("deployments", deployment),
            ("stacks", stack),
            ("stackreleases", release),
            ("containers", container),
        ] {
            sqlx::query(sqlx::AssertSqlSafe(format!(
                "SELECT id FROM {table} WHERE id=$1 FOR UPDATE NOWAIT"
            )))
            .bind(id)
            .fetch_one(&mut *inventory)
            .await
            .unwrap_or_else(|error| panic!("{phase} locked {table} before its Platform: {error}"));
        }
        inventory.commit().await.unwrap();
        tokio::time::timeout(StdDuration::from_secs(5), operation)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        let pending: Option<Uuid> =
            sqlx::query_scalar("SELECT containeroperationid FROM containers WHERE id=$1")
                .bind(container)
                .fetch_one(&f.pool)
                .await
                .unwrap();
        if let Some(pending) = pending {
            store.finish(pending).await.unwrap();
        }
        for (table, id) in [
            ("deployments", deployment),
            ("stacks", stack),
            ("containers", container),
        ] {
            let idle: bool = sqlx::query_scalar(sqlx::AssertSqlSafe(format!("SELECT controlstate='Idle' AND containeroperationid IS NULL FROM {table} WHERE id=$1")))
                .bind(id).fetch_one(&f.pool).await.unwrap();
            assert!(idle, "{phase} left {table} processing");
        }
    }
    f.docker_server.abort();
}

#[derive(Default)]
struct Runtime {
    calls: Mutex<Vec<RuntimeCall>>,
    state: Mutex<Option<String>>,
    fail: std::sync::atomic::AtomicBool,
}
impl ContainerMutationRuntime for Runtime {
    fn mutate<'a>(
        &'a self,
        target: &'a ContainerTarget,
        action: ContainerAction,
        _: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<(), RuntimeCapabilityError>> {
        Box::pin(async move {
            self.calls.lock().await.push((
                target.platform_id,
                target.docker_id.clone(),
                target.node_id.clone(),
                action,
            ));
            if self.fail.load(std::sync::atomic::Ordering::SeqCst) {
                return Err(RuntimeCapabilityError::new(
                    RuntimeErrorKind::Remote,
                    "Rejected by Docker.",
                    false,
                ));
            }
            *self.state.lock().await = match action {
                ContainerAction::Stop => Some("exited".into()),
                ContainerAction::Pause => Some("paused".into()),
                ContainerAction::Delete(_) => None,
                _ => Some("running".into()),
            };
            Ok(())
        })
    }
    fn observe<'a>(
        &'a self,
        _: &'a ContainerTarget,
        _: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Option<String>, RuntimeCapabilityError>> {
        Box::pin(async move { Ok(self.state.lock().await.clone()) })
    }
}

// Ports ContainerCommandEndpointTests: the existing public IDs, five actions,
// persisted observed state, and direct Swarm Task protection.
#[tokio::test]
#[ignore = "requires CITADEL_PHASE4_DATABASE_URL"]
async fn container_actions_preserve_ids_permissions_and_persist_observed_state() {
    let mut f = fixture().await;
    let runtime = Arc::new(Runtime::default());
    let service = Arc::new(ContainerMutationService::new(
        Arc::new(PostgresContainerRepository::new(f.pool.clone())),
        runtime.clone(),
        std::sync::Arc::new(
            citadel_server::tasks::platforms::TrackedContainerTasks::new(
                citadel_runtime::DynamicTasks::new(tokio_util::sync::CancellationToken::new()),
            ),
        ),
    ));
    let mut state = f.lookup_state.platforms.clone();
    state.containers = service;
    f.app = platforms_http::router(state);
    let id: Uuid = sqlx::query_scalar("SELECT id FROM containers WHERE platformid=$1")
        .bind(f.platform_id)
        .fetch_one(&f.pool)
        .await
        .unwrap();
    let docker_id = "a".repeat(64);
    sqlx::query("UPDATE containers SET dockercontainerid=$2,dockernodeid='node-1' WHERE id=$1")
        .bind(id)
        .bind(&docker_id)
        .execute(&f.pool)
        .await
        .unwrap();
    let denied = ActorPrincipal {
        actor_id: ActorId::new(f.actor_id),
        roles: vec![],
        ..f.administrator.clone()
    };
    for (route, action, expected) in [
        ("start", ContainerAction::Start, "Running"),
        ("stop", ContainerAction::Stop, "Exited"),
        ("pause", ContainerAction::Pause, "Paused"),
        ("unpause", ContainerAction::Unpause, "Running"),
        ("restart", ContainerAction::Restart, "Running"),
    ] {
        let url = format!("/api/v1/containers/{route}");
        let unauthenticated = Request::builder()
            .method(Method::PATCH)
            .uri(&url)
            .header("content-type", "application/json")
            .body(Body::from(json!([id]).to_string()))
            .unwrap();
        assert_eq!(
            f.app
                .clone()
                .oneshot(unauthenticated)
                .await
                .unwrap()
                .status(),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            send_json(&f, Method::PATCH, &url, denied.clone(), json!([id]))
                .await
                .status(),
            StatusCode::FORBIDDEN
        );
        // The endpoint accepts both its persisted UUID and the old short Docker ID.
        let response = send_json(
            &f,
            Method::PATCH,
            &url,
            f.administrator.clone(),
            json!([id, &docker_id[..12]]),
        )
        .await;
        assert_eq!(
            response.status(),
            StatusCode::NO_CONTENT,
            "{route}: {:?}",
            to_bytes(response.into_body(), 4096).await.unwrap()
        );
        let stored: (String, String, Option<Uuid>) = sqlx::query_as(
            "SELECT state,controlstate,controltriggeredby FROM containers WHERE id=$1",
        )
        .bind(id)
        .fetch_one(&f.pool)
        .await
        .unwrap();
        assert_eq!(stored, (expected.into(), "Idle".into(), None));
        assert_eq!(
            runtime.calls.lock().await.last().unwrap(),
            &(
                f.platform_id,
                docker_id.clone(),
                Some("node-1".into()),
                action
            )
        );
    }
    assert_eq!(
        runtime.calls.lock().await.len(),
        5,
        "duplicate selectors must dispatch only once"
    );
    for invalid in [
        json!([]),
        json!(["invalid"]),
        json!([Uuid::nil()]),
        json!(vec![id; 101]),
    ] {
        assert_eq!(
            send_json(
                &f,
                Method::PATCH,
                "/api/v1/containers/start",
                f.administrator.clone(),
                invalid
            )
            .await
            .status(),
            StatusCode::BAD_REQUEST
        );
    }
    for column in ["isswarmtask", "issystem"] {
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "UPDATE containers SET {column}=true WHERE id=$1"
        )))
        .bind(id)
        .execute(&f.pool)
        .await
        .unwrap();
        assert_eq!(
            send_json(
                &f,
                Method::PATCH,
                "/api/v1/containers/restart",
                f.administrator.clone(),
                json!([id])
            )
            .await
            .status(),
            StatusCode::CONFLICT
        );
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "UPDATE containers SET {column}=false WHERE id=$1"
        )))
        .bind(id)
        .execute(&f.pool)
        .await
        .unwrap();
    }
    assert_eq!(runtime.calls.lock().await.len(), 5);
    // Execute implies Write, but Read alone did not grant it.
    sqlx::query("UPDATE resourceaccesses SET permissionlevel=4 WHERE actorid=$1 AND resourceid=$2")
        .bind(f.actor_id)
        .bind(f.platform_id)
        .execute(&f.pool)
        .await
        .unwrap();
    assert_eq!(
        send_json(
            &f,
            Method::PATCH,
            "/api/v1/containers/start",
            denied.clone(),
            json!([id])
        )
        .await
        .status(),
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        send_json(
            &f,
            Method::DELETE,
            "/api/v1/containers",
            denied,
            json!({"containerIds":[id],"force":true})
        )
        .await
        .status(),
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        runtime.calls.lock().await.last().unwrap().3,
        ContainerAction::Delete(DeleteContainerOptions {
            force: true,
            v: false,
            link: false
        })
    );
    assert!(
        !sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM containers WHERE id=$1)")
            .bind(id)
            .fetch_one(&f.pool)
            .await
            .unwrap()
    );
    f.docker_server.abort();
    let _ = tokio::fs::remove_file(f.docker_socket).await;
}

// Ports ContainerCommandCompletionTests and ContainerProcessingConsistencyTests:
#[tokio::test]
#[ignore = "requires CITADEL_PHASE4_DATABASE_URL"]
async fn deployment_state_routes_use_deployment_grants_and_preserve_atomic_claims() {
    let mut f = fixture().await;
    let runtime = Arc::new(Runtime::default());
    let mut state = f.lookup_state.platforms.clone();
    state.containers = Arc::new(ContainerMutationService::new(
        Arc::new(PostgresContainerRepository::new(f.pool.clone())),
        runtime.clone(),
        std::sync::Arc::new(
            citadel_server::tasks::platforms::TrackedContainerTasks::new(
                citadel_runtime::DynamicTasks::new(tokio_util::sync::CancellationToken::new()),
            ),
        ),
    ));
    f.app = platforms_http::router(state);
    let container: Uuid = sqlx::query_scalar("SELECT id FROM containers WHERE platformid=$1")
        .bind(f.platform_id)
        .fetch_one(&f.pool)
        .await
        .unwrap();
    let deployment = Uuid::now_v7();
    sqlx::query("UPDATE platforms SET platformdescriptor='{\"$type\":\"Docker\"}' WHERE id=$1")
        .bind(f.platform_id)
        .execute(&f.pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO deployments(id,name,platformid,createdbyactorid,status,spec) VALUES($1,'state-actions',$2,$3,'Healthy','{}')")
        .bind(deployment).bind(f.platform_id).bind(f.administrator.actor_id.value()).execute(&f.pool).await.unwrap();
    sqlx::query("UPDATE containers SET deploymentid=$2,isswarmtask=false WHERE id=$1")
        .bind(container)
        .bind(deployment)
        .execute(&f.pool)
        .await
        .unwrap();
    let actor_id = Uuid::now_v7();
    sqlx::query("INSERT INTO actors(id,isenabled,type) VALUES($1,true,'User')")
        .bind(actor_id)
        .execute(&f.pool)
        .await
        .unwrap();
    let mut writer = f.administrator.clone();
    writer.actor_id = ActorId::new(actor_id);
    writer.roles.clear();
    assert_eq!(
        send_json(
            &f,
            Method::POST,
            "/api/v1/deployments/start",
            writer.clone(),
            json!([deployment])
        )
        .await
        .status(),
        StatusCode::FORBIDDEN
    );
    sqlx::query("INSERT INTO resourceaccesses(id,actorid,resourcetype,resourceid,permissionlevel,specificpermissions) VALUES($1,$2,$3,$4,2,0)")
        .bind(Uuid::now_v7()).bind(actor_id).bind(ResourceType::Deployment as i32).bind(deployment).execute(&f.pool).await.unwrap();
    for (name, action, status) in [
        ("start", ContainerAction::Start, "Healthy"),
        ("pause", ContainerAction::Pause, "Paused"),
        ("resume", ContainerAction::Unpause, "Healthy"),
        ("restart", ContainerAction::Restart, "Healthy"),
        ("stop", ContainerAction::Stop, "Stopped"),
    ] {
        let response = send_json(
            &f,
            Method::POST,
            &format!("/api/v1/deployments/{name}"),
            writer.clone(),
            json!([deployment]),
        )
        .await;
        assert_eq!(
            response.status(),
            StatusCode::NO_CONTENT,
            "{name}: {}",
            json_body(response).await
        );
        assert_eq!(runtime.calls.lock().await.last().unwrap().3, action);
        let saved: (String, String) =
            sqlx::query_as("SELECT status,controlstate FROM deployments WHERE id=$1")
                .bind(deployment)
                .fetch_one(&f.pool)
                .await
                .unwrap();
        assert_eq!(saved, (status.into(), "Idle".into()));
    }
    let calls = runtime.calls.lock().await.len();
    for (ids, expected) in [
        (json!([]), StatusCode::BAD_REQUEST),
        (json!([Uuid::nil()]), StatusCode::BAD_REQUEST),
        (json!([deployment, Uuid::now_v7()]), StatusCode::NOT_FOUND),
    ] {
        assert_eq!(
            send_json(
                &f,
                Method::POST,
                "/api/v1/deployments/start",
                writer.clone(),
                ids
            )
            .await
            .status(),
            expected
        );
    }
    sqlx::query(
        "UPDATE platforms SET platformdescriptor='{\"$type\":\"DockerSwarm\"}' WHERE id=$1",
    )
    .bind(f.platform_id)
    .execute(&f.pool)
    .await
    .unwrap();
    assert_eq!(
        send_json(
            &f,
            Method::POST,
            "/api/v1/deployments/start",
            writer,
            json!([deployment])
        )
        .await
        .status(),
        StatusCode::BAD_REQUEST
    );
    assert_eq!(runtime.calls.lock().await.len(), calls);
    assert_eq!(
        sqlx::query_scalar::<_, String>("SELECT controlstate FROM deployments WHERE id=$1")
            .bind(deployment)
            .fetch_one(&f.pool)
            .await
            .unwrap(),
        "Idle"
    );
    f.docker_server.abort();
    let _ = tokio::fs::remove_file(f.docker_socket).await;
}

// Ports ContainerCommandCompletionTests and ContainerProcessingConsistencyTests:
// all-or-nothing parent claims, inventory writes during commands, and a late
// completion from the same actor must not release a newer operation.
#[tokio::test]
#[ignore = "requires CITADEL_PHASE4_DATABASE_URL"]
async fn container_claims_are_atomic_parent_aware_and_recovered_without_replaying_commands() {
    let f = fixture().await;
    let store = Arc::new(PostgresContainerRepository::new(f.pool.clone()));
    let actor = f.administrator.actor_id;
    let container: Uuid = sqlx::query_scalar("SELECT id FROM containers WHERE platformid=$1")
        .bind(f.platform_id)
        .fetch_one(&f.pool)
        .await
        .unwrap();
    let deployment = Uuid::now_v7();
    let stack = Uuid::now_v7();
    let release = Uuid::now_v7();
    sqlx::query("INSERT INTO deployments(id,name,platformid,createdbyactorid,status,spec) VALUES($1,'mutation-parent',$2,$3,'Healthy','{}')").bind(deployment).bind(f.platform_id).bind(actor.value()).execute(&f.pool).await.unwrap();
    sqlx::query("INSERT INTO stacks(id,name,createdbyactorid,stacksource,driftpolicy,stackupdatestate) VALUES($1,'mutation-stack',$2,'WebEditor','{}','{}')").bind(stack).bind(actor.value()).execute(&f.pool).await.unwrap();
    sqlx::query("INSERT INTO stackreleases(id,stackid,platformid,createdbyactorid,status,spec,version) VALUES($1,$2,$3,$4,'Healthy','{}','1')").bind(release).bind(stack).bind(f.platform_id).bind(actor.value()).execute(&f.pool).await.unwrap();
    sqlx::query("UPDATE stacks SET currentstackreleaseid=$2 WHERE id=$1")
        .bind(stack)
        .bind(release)
        .execute(&f.pool)
        .await
        .unwrap();
    sqlx::query("UPDATE containers SET deploymentid=$2,stackid=$3 WHERE id=$1")
        .bind(container)
        .bind(deployment)
        .bind(stack)
        .execute(&f.pool)
        .await
        .unwrap();
    assert!(
        store
            .claim(
                actor,
                true,
                &[container, Uuid::now_v7()],
                ContainerAction::Start
            )
            .await
            .is_err()
    );
    assert_eq!(
        sqlx::query_scalar::<_, String>("SELECT controlstate FROM deployments WHERE id=$1")
            .bind(deployment)
            .fetch_one(&f.pool)
            .await
            .unwrap(),
        "Idle"
    );
    let old = store
        .claim(actor, true, &[container], ContainerAction::Start)
        .await
        .unwrap();
    assert!(
        matches!(store.claim(actor, true, &[container], ContainerAction::Stop).await, Err(e) if e.kind == RuntimeErrorKind::Conflict)
    );
    assert_eq!(old.deployment_ids, [deployment]);
    assert_eq!(old.stack_ids, [stack]);
    sqlx::query("UPDATE containers SET rowversion=rowversion+1 WHERE id=$1")
        .bind(container)
        .execute(&f.pool)
        .await
        .unwrap();
    store
        .observed(old.operation_id, &old.targets[0], Some("running"))
        .await
        .unwrap();
    store.finish(old.operation_id).await.unwrap();
    let current = store
        .claim(actor, true, &[container], ContainerAction::Stop)
        .await
        .unwrap();
    store
        .observed(old.operation_id, &old.targets[0], Some("dead"))
        .await
        .unwrap();
    store.finish(old.operation_id).await.unwrap();
    assert_eq!(
        sqlx::query_as::<_, (String, Option<Uuid>)>(
            "SELECT state,containeroperationid FROM containers WHERE id=$1"
        )
        .bind(container)
        .fetch_one(&f.pool)
        .await
        .unwrap(),
        ("Running".into(), Some(current.operation_id))
    );
    for table in ["containers", "deployments", "stacks"] {
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "UPDATE {table} SET controlstartedat=0 WHERE containeroperationid=$1"
        )))
        .bind(current.operation_id)
        .execute(&f.pool)
        .await
        .unwrap();
    }
    let runtime = Arc::new(Runtime::default());
    *runtime.state.lock().await = Some("exited".into());
    let service = ContainerMutationService::new(
        store.clone(),
        runtime.clone(),
        std::sync::Arc::new(
            citadel_server::tasks::platforms::TrackedContainerTasks::new(
                citadel_runtime::DynamicTasks::new(tokio_util::sync::CancellationToken::new()),
            ),
        ),
    );
    service.reconcile(&CancellationToken::new()).await.unwrap();
    assert!(
        runtime.calls.lock().await.is_empty(),
        "recovery must inspect, never replay stop/restart/delete"
    );
    assert_eq!(
        sqlx::query_as::<_, (String, String)>(
            "SELECT status,controlstate FROM deployments WHERE id=$1"
        )
        .bind(deployment)
        .fetch_one(&f.pool)
        .await
        .unwrap(),
        ("Stopped".into(), "Idle".into())
    );
    assert_eq!(
        sqlx::query_scalar::<_, String>("SELECT status FROM stackreleases WHERE id=$1")
            .bind(release)
            .fetch_one(&f.pool)
            .await
            .unwrap(),
        "Stopped"
    );
    // A rejected mutation retains a fenced claim for recovery instead of pretending the old state is current.
    runtime
        .fail
        .store(true, std::sync::atomic::Ordering::SeqCst);
    assert!(
        service
            .execute(
                actor,
                true,
                vec![container.to_string()],
                ContainerAction::Start
            )
            .await
            .is_err()
    );
    assert_eq!(
        sqlx::query_scalar::<_, String>("SELECT controlstate FROM containers WHERE id=$1")
            .bind(container)
            .fetch_one(&f.pool)
            .await
            .unwrap(),
        "Processing"
    );
    f.docker_server.abort();
    let _ = tokio::fs::remove_file(f.docker_socket).await;
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE4_DATABASE_URL"]
async fn container_commands_use_the_owning_edge_node_and_confirm_deletion_without_a_manager_fallback()
 {
    use citadel_adapters::connectors::edge::EdgeTarget;
    use citadel_contracts::citadel::{
        containers::v1::{
            ContainerIds, DeleteContainerRequest, InspectContainerRequest, ListContainersResponse,
        },
        edge::v1::{EdgeCommandKind as Kind, core_envelope},
        shared_models::v1::{ContainerState, ContainerStateType, InspectContainerResponse},
    };
    use prost::Message;
    let f = fixture().await;
    let id: Uuid = sqlx::query_scalar("SELECT id FROM containers WHERE platformid=$1")
        .bind(f.platform_id)
        .fetch_one(&f.pool)
        .await
        .unwrap();
    sqlx::query("UPDATE containers SET dockernodeid='node-1' WHERE id=$1")
        .bind(id)
        .execute(&f.pool)
        .await
        .unwrap();
    let registry = &f.lookup_state.platforms.edge;
    let (session, mut commands) = registry
        .register(
            EdgeTarget::node(f.platform_id, "node-1".into()),
            Uuid::now_v7(),
        )
        .unwrap();
    let (_, mut wrong_commands) = registry
        .register(
            EdgeTarget::node(f.platform_id, "another-node".into()),
            Uuid::now_v7(),
        )
        .unwrap();
    for (route, kind, result) in [
        ("start", Kind::ContainerStart, ContainerStateType::Running),
        ("stop", Kind::ContainerStop, ContainerStateType::Exited),
        (
            "restart",
            Kind::ContainerRestart,
            ContainerStateType::Running,
        ),
        ("pause", Kind::ContainerPause, ContainerStateType::Paused),
        (
            "unpause",
            Kind::ContainerUnpause,
            ContainerStateType::Running,
        ),
        ("delete", Kind::ContainerDelete, ContainerStateType::Unknown),
    ] {
        let (method, url, body) = if route == "delete" {
            (
                Method::DELETE,
                "/api/v1/containers".into(),
                json!({"containerIds":[id],"force":true}),
            )
        } else {
            (
                Method::PATCH,
                format!("/api/v1/containers/{route}"),
                json!([id]),
            )
        };
        let (response, ()) = tokio::join!(
            send_json(&f, method, &url, f.administrator.clone(), body),
            async {
                for step in 0..if route == "delete" { 3 } else { 2 } {
                    let envelope = tokio::time::timeout(StdDuration::from_secs(3), commands.recv())
                        .await
                        .unwrap()
                        .unwrap();
                    let command_id = Uuid::parse_str(&envelope.command_id).unwrap();
                    let Some(core_envelope::Body::Command(command)) = envelope.body else {
                        panic!("expected command");
                    };
                    if step == 0 {
                        assert_eq!(command.kind, kind as i32);
                        if kind == Kind::ContainerDelete {
                            let request =
                                DeleteContainerRequest::decode(command.payload.as_slice()).unwrap();
                            assert_eq!(request.ids, ["container-1"]);
                            assert_eq!(
                                (request.v, request.force, request.link),
                                (Some(false), Some(true), Some(false))
                            );
                        } else {
                            assert_eq!(
                                ContainerIds::decode(command.payload.as_slice())
                                    .unwrap()
                                    .ids,
                                ["container-1"]
                            );
                        }
                        session.complete(command_id, true);
                    } else if step == 1 {
                        assert_eq!(command.kind, Kind::ContainerInspect as i32);
                        assert_eq!(
                            InspectContainerRequest::decode(command.payload.as_slice())
                                .unwrap()
                                .container_id,
                            "container-1"
                        );
                        if route == "delete" {
                            session.complete(command_id, false);
                        } else {
                            session.output(
                                command_id,
                                InspectContainerResponse {
                                    id: "container-1".into(),
                                    state: Some(ContainerState {
                                        status: result as i32,
                                        ..Default::default()
                                    }),
                                    ..Default::default()
                                }
                                .encode_to_vec(),
                            );
                            session.complete(command_id, true);
                        }
                    } else {
                        assert_eq!(command.kind, Kind::ContainerList as i32);
                        session.output(
                            command_id,
                            ListContainersResponse::default().encode_to_vec(),
                        );
                        session.complete(command_id, true);
                    }
                }
            }
        );
        assert_eq!(
            response.status(),
            StatusCode::NO_CONTENT,
            "{route}: {:?}",
            to_bytes(response.into_body(), 4096).await.unwrap()
        );
        assert!(wrong_commands.try_recv().is_err());
    }
    assert!(
        !sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM containers WHERE id=$1)")
            .bind(id)
            .fetch_one(&f.pool)
            .await
            .unwrap()
    );
    registry.remove(&session);
    f.docker_server.abort();
    let _ = tokio::fs::remove_file(f.docker_socket).await;
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE4_DATABASE_URL"]
async fn twenty_local_stops_are_bounded_and_partial_failure_recovers_without_replay_or_sync() {
    use citadel_adapters::connectors::routing::containers::ContainerRuntimeRouter;
    use std::sync::atomic::{AtomicUsize, Ordering};
    for (fail, cancel) in [(false, false), (true, false), (false, true)] {
        let mut f = fixture().await;
        let mut inventory = snapshot(f.platform_id);
        let template = inventory.containers[0].clone();
        inventory.containers = (0..20)
            .map(|i| RuntimeContainerSummary {
                id: format!("bulk-{i}"),
                name: format!("bulk-{i}"),
                ..template.clone()
            })
            .collect();
        PostgresInventoryProjectionStore::new(f.pool.clone())
            .persist(&inventory)
            .await
            .unwrap();
        sqlx::query("UPDATE platforms SET platformdescriptor='{\"$type\":\"Docker\"}',connectortype='Local' WHERE id=$1").bind(f.platform_id).execute(&f.pool).await.unwrap();
        let ids: Vec<Uuid> =
            sqlx::query_scalar("SELECT id FROM containers WHERE platformid=$1 ORDER BY id")
                .bind(f.platform_id)
                .fetch_all(&f.pool)
                .await
                .unwrap();
        assert_eq!(ids.len(), 20);
        let calls = Arc::new(AtomicUsize::new(0));
        let inspections = Arc::new(AtomicUsize::new(0));
        let active = Arc::new(AtomicUsize::new(0));
        let peak = Arc::new(AtomicUsize::new(0));
        let stopped = Arc::new(Mutex::new(std::collections::BTreeSet::<String>::new()));
        let path = std::env::temp_dir().join(format!("bulk-local-{}.sock", Uuid::now_v7()));
        let listener = UnixListener::bind(&path).unwrap();
        let server = tokio::spawn({
            let calls = calls.clone();
            let inspections = inspections.clone();
            let active = active.clone();
            let peak = peak.clone();
            let stopped = stopped.clone();
            async move {
                let mut tasks = tokio::task::JoinSet::new();
                loop {
                    tokio::select! {
                        accepted=listener.accept()=> {
                            let (mut socket,_)=accepted.unwrap();
                            let calls=calls.clone(); let inspections=inspections.clone(); let active=active.clone(); let peak=peak.clone(); let stopped=stopped.clone();
                            tasks.spawn(async move {
                                let mut data=Vec::new(); let mut buf=[0;4096];
                                while !data.windows(4).any(|w|w==b"\r\n\r\n") { let n=socket.read(&mut buf).await.unwrap(); if n==0 { return; } data.extend_from_slice(&buf[..n]); }
                                let request=String::from_utf8_lossy(&data); let line=request.lines().next().unwrap();
                                let (status,body)=if line.starts_with("GET /version ") { (200,json!({"ApiVersion":"1.49","MinAPIVersion":"1.41"})) }
                                else {
                                    let url=line.split_whitespace().nth(1).unwrap();
                                    let parts:Vec<_>=url.split('/').collect();
                                    assert_eq!(parts[2],"containers","no unrelated enumeration: {line}");
                                    let id=parts[3].to_owned();
                                    if line.starts_with("POST ") {
                                        assert!(url.contains("/stop"));
                                        calls.fetch_add(1,Ordering::SeqCst);
                                        let count=active.fetch_add(1,Ordering::SeqCst)+1; peak.fetch_max(count,Ordering::SeqCst);
                                        tokio::time::sleep(StdDuration::from_millis(if cancel {500} else {80})).await;
                                        active.fetch_sub(1,Ordering::SeqCst);
                                        if fail && id=="bulk-0" { (500,json!({"message":"deliberate stop failure"})) }
                                        else { stopped.lock().await.insert(id); (204,json!(null)) }
                                    } else {
                                        assert!(url.ends_with("/json"),"no resource list: {line}");
                                        inspections.fetch_add(1,Ordering::SeqCst);
                                        let state=if stopped.lock().await.contains(&id) {"exited"} else {"running"};
                                        (200,json!({"Id":id,"State":{"Status":state}}))
                                    }
                                };
                                let body=if status==204 {String::new()} else {body.to_string()};
                                let _ = socket.write_all(format!("HTTP/1.1 {status} Response\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).as_bytes()).await;
                            });
                        },
                        result=tasks.join_next(), if !tasks.is_empty()=> { result.unwrap().unwrap(); }
                    }
                }
            }
        });
        let runtime = ContainerRuntimeRouter::new(
            f.pool.clone(),
            DockerClient::new(&path, StdDuration::from_secs(2)).unwrap(),
            None,
            f.lookup_state.platforms.edge.clone(),
        );
        let service = Arc::new(runtime.into_service(Arc::new(
            citadel_server::tasks::platforms::TrackedContainerTasks::new(
                citadel_runtime::DynamicTasks::new(CancellationToken::new()),
            ),
        )));
        let mut state = f.lookup_state.platforms.clone();
        state.containers = service.clone();
        f.app = platforms_http::router(state);
        if cancel {
            let token = CancellationToken::new();
            let (result, ()) = tokio::join!(
                service.execute_background(
                    f.administrator.actor_id,
                    ids.iter().map(ToString::to_string).collect(),
                    ContainerAction::Stop,
                    token.clone()
                ),
                async {
                    tokio::time::timeout(StdDuration::from_secs(3), async {
                        while calls.load(Ordering::SeqCst) < CONTAINER_IO_CONCURRENCY {
                            tokio::task::yield_now().await;
                        }
                    })
                    .await
                    .unwrap();
                    token.cancel();
                }
            );
            assert!(matches!(result,Err(error) if error.kind==RuntimeErrorKind::Cancelled));
            tokio::time::sleep(StdDuration::from_millis(600)).await;
            assert_eq!(
                calls.load(Ordering::SeqCst),
                CONTAINER_IO_CONCURRENCY,
                "cancelled queued mutations must not reach Docker"
            );
            assert_eq!(inspections.load(Ordering::SeqCst), 0);
            sqlx::query("UPDATE containers SET controlstartedat=1 WHERE platformid=$1")
                .bind(f.platform_id)
                .execute(&f.pool)
                .await
                .unwrap();
            service.reconcile(&CancellationToken::new()).await.unwrap();
            assert_eq!(calls.load(Ordering::SeqCst), CONTAINER_IO_CONCURRENCY);
            assert_eq!(inspections.load(Ordering::SeqCst), 20);
            assert_eq!(
                sqlx::query_scalar::<_, i64>(
                    "SELECT count(*) FROM containers WHERE platformid=$1 AND controlstate='Idle'"
                )
                .bind(f.platform_id)
                .fetch_one(&f.pool)
                .await
                .unwrap(),
                20
            );
            server.abort();
            f.docker_server.abort();
            let _ = tokio::fs::remove_file(path).await;
            f.pool.close().await;
            continue;
        }
        let response = send_json(
            &f,
            Method::PATCH,
            "/api/v1/containers/stop",
            f.administrator.clone(),
            json!(ids),
        )
        .await;
        assert_eq!(response.status().is_success(), !fail);
        assert_eq!(calls.load(Ordering::SeqCst), 20);
        assert_eq!(inspections.load(Ordering::SeqCst), 20);
        assert_eq!(peak.load(Ordering::SeqCst), CONTAINER_IO_CONCURRENCY);
        let (exited,processing):(i64,i64)=sqlx::query_as("SELECT count(*) FILTER(WHERE state='Exited'),count(*) FILTER(WHERE controlstate='Processing') FROM containers WHERE platformid=$1").bind(f.platform_id).fetch_one(&f.pool).await.unwrap();
        assert_eq!(exited, if fail { 19 } else { 20 });
        assert_eq!(processing, if fail { 20 } else { 0 });
        if fail {
            sqlx::query("UPDATE containers SET controlstartedat=1 WHERE platformid=$1")
                .bind(f.platform_id)
                .execute(&f.pool)
                .await
                .unwrap();
            service.reconcile(&CancellationToken::new()).await.unwrap();
            assert_eq!(
                calls.load(Ordering::SeqCst),
                20,
                "recovery must never replay stop"
            );
            assert_eq!(inspections.load(Ordering::SeqCst), 40);
            let pending: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM containers WHERE platformid=$1 AND controlstate='Processing'",
            )
            .bind(f.platform_id)
            .fetch_one(&f.pool)
            .await
            .unwrap();
            assert_eq!(pending, 0);
        }
        server.abort();
        f.docker_server.abort();
        let _ = tokio::fs::remove_file(path).await;
        f.pool.close().await;
    }
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE4_DATABASE_URL"]
async fn observation_batch_rolls_back_all_targets_and_does_not_advance_the_generation() {
    use citadel_platforms::jobs::{ProjectionKind, ProjectionWrite, SnapshotGeneration};
    let f = fixture().await;
    let mut inventory = snapshot(f.platform_id);
    let template = inventory.containers[0].clone();
    inventory.containers = vec![
        RuntimeContainerSummary {
            id: "batch-good".into(),
            name: "batch-good".into(),
            ..template.clone()
        },
        RuntimeContainerSummary {
            id: "batch-bad".into(),
            name: "reject-batch".into(),
            ..template
        },
    ];
    PostgresInventoryProjectionStore::new(f.pool.clone())
        .persist(&inventory)
        .await
        .unwrap();
    let ids: Vec<Uuid> =
        sqlx::query_scalar("SELECT id FROM containers WHERE platformid=$1 ORDER BY id")
            .bind(f.platform_id)
            .fetch_all(&f.pool)
            .await
            .unwrap();
    let store = PostgresContainerRepository::new(f.pool.clone());
    let claim = store
        .claim(f.administrator.actor_id, true, &ids, ContainerAction::Stop)
        .await
        .unwrap();
    let stamp = SnapshotGeneration::capture(f.platform_id, None, ProjectionKind::Containers).await;
    sqlx::query("CREATE FUNCTION phase7_reject_observation() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.name='reject-batch' AND NEW.state='Exited' THEN RAISE EXCEPTION 'deliberate batch failure'; END IF; RETURN NEW; END $$").execute(&f.pool).await.unwrap();
    sqlx::query("CREATE TRIGGER phase7_reject_observation BEFORE UPDATE ON containers FOR EACH ROW EXECUTE FUNCTION phase7_reject_observation()").execute(&f.pool).await.unwrap();
    let observations: Vec<_> = claim
        .targets
        .iter()
        .map(|target| ContainerObservation {
            target: target.clone(),
            state: Some("exited".into()),
        })
        .collect();
    assert!(
        store
            .observed_batch(claim.operation_id, &observations)
            .await
            .is_err()
    );
    assert!(
        stamp.matches(
            &ProjectionWrite::begin(f.platform_id, None, ProjectionKind::Containers).await
        )
    );
    let running: i64 =
        sqlx::query_scalar("SELECT count(*) FROM containers WHERE id=ANY($1) AND state='Running'")
            .bind(&ids)
            .fetch_one(&f.pool)
            .await
            .unwrap();
    assert_eq!(running, 2);
    sqlx::query("DROP TRIGGER phase7_reject_observation ON containers")
        .execute(&f.pool)
        .await
        .unwrap();
    sqlx::query("DROP FUNCTION phase7_reject_observation()")
        .execute(&f.pool)
        .await
        .unwrap();
    store
        .observed_batch(claim.operation_id, &observations)
        .await
        .unwrap();
    let before: Vec<(Uuid, i64)> =
        sqlx::query_as("SELECT id,rowversion FROM containers WHERE id=ANY($1) ORDER BY id")
            .bind(&ids)
            .fetch_all(&f.pool)
            .await
            .unwrap();
    store
        .observed_batch(claim.operation_id, &observations)
        .await
        .unwrap();
    let after: Vec<(Uuid, i64)> =
        sqlx::query_as("SELECT id,rowversion FROM containers WHERE id=ANY($1) ORDER BY id")
            .bind(&ids)
            .fetch_all(&f.pool)
            .await
            .unwrap();
    assert_eq!(
        before, after,
        "semantic duplicates must not increment revisions"
    );
    let mut wrong: Vec<_> = claim
        .targets
        .iter()
        .map(|target| ContainerObservation {
            target: target.clone(),
            state: None,
        })
        .collect();
    for value in &mut wrong {
        value.target.node_id = Some("another-node".into());
    }
    store
        .observed_batch(claim.operation_id, &wrong)
        .await
        .unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM containers WHERE id=ANY($1)")
            .bind(&ids)
            .fetch_one(&f.pool)
            .await
            .unwrap(),
        2,
        "wrong node must not delete a claimed UUID"
    );
    store.finish(claim.operation_id).await.unwrap();
    store
        .observed_batch(
            claim.operation_id,
            &claim
                .targets
                .iter()
                .map(|target| ContainerObservation {
                    target: target.clone(),
                    state: None,
                })
                .collect::<Vec<_>>(),
        )
        .await
        .unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM containers WHERE id=ANY($1)")
            .bind(&ids)
            .fetch_one(&f.pool)
            .await
            .unwrap(),
        2,
        "finished claims cannot delete observations"
    );
    f.docker_server.abort();
    f.pool.close().await;
}
