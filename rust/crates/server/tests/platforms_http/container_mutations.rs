use super::*;
use citadel_adapters::container_mutation_store::PostgresContainerRepository;
use citadel_platforms::{RuntimeCapabilityError, RuntimeErrorKind, containers::*};
use citadel_primitives::ResourceType;
use futures_util::future::BoxFuture;
use tokio_util::sync::CancellationToken;

type RuntimeCall = (Uuid, String, Option<String>, ContainerAction);

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
    let service = ContainerMutationService::new(store.clone(), runtime.clone());
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
    use citadel_adapters::edge::EdgeTarget;
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
