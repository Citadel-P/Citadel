use super::*;
use citadel_adapters::edge::{EdgeRegistry, EdgeTarget};
use citadel_contracts::citadel::{
    edge::v1::{EdgeCommandKind, core_envelope},
    images::v1::CheckBuildHostResponse,
};
use prost::Message;

// Ports BuildAgentPoolCommandTests: ready, missing BuildKit, unreachable,
// unavailable Docker, Edge dispatch and unsupported-provider rejection.
pub(super) async fn verify(
    app: &Router,
    db: &sqlx::PgPool,
    edge: &EdgeRegistry,
    builds: &Arc<BuildService>,
    principal: &ActorPrincipal,
    id: Uuid,
) {
    verify_capabilities(app, db, principal, id).await;
    verify_edits(app, db, builds, principal, id).await;
    let uri = format!("/api/v1/buildAgentPools/{id}/test");
    assert_eq!(
        request(app, Method::POST, &uri, None, None).await.status(),
        StatusCode::UNAUTHORIZED
    );
    let mut denied = principal.clone();
    denied.roles.clear();
    denied.actor_id = ActorId::new(Uuid::now_v7());
    assert_eq!(
        request(app, Method::POST, &uri, Some(denied), None)
            .await
            .status(),
        StatusCode::FORBIDDEN
    );
    let target = EdgeTarget::build_pool(id);
    for (available, buildkit, abort_caller) in [
        (true, "v0.20.2", false),
        (true, "", true),
        (false, "", false),
    ] {
        let (session, mut outbound) = edge.register(target.clone(), Uuid::now_v7()).unwrap();
        let client = {
            let app = app.clone();
            let uri = uri.clone();
            let actor = principal.clone();
            tokio::spawn(async move { request(&app, Method::POST, &uri, Some(actor), None).await })
        };
        let envelope = tokio::time::timeout(std::time::Duration::from_secs(5), outbound.recv())
            .await
            .unwrap()
            .unwrap();
        let command_id = Uuid::parse_str(&envelope.command_id).unwrap();
        let Some(core_envelope::Body::Command(command)) = envelope.body else {
            panic!("expected command");
        };
        assert_eq!(command.kind, EdgeCommandKind::ImageCheckBuildHost as i32);
        assert_eq!(command.resource_id, id.to_string());
        assert_eq!(
            builds.store().get_pool(id).await.unwrap().control_state,
            "Processing"
        );
        assert_eq!(
            request(app, Method::POST, &uri, Some(principal.clone()), None)
                .await
                .status(),
            StatusCode::CONFLICT
        );
        if abort_caller {
            client.abort();
        }
        session.output(
            command_id,
            CheckBuildHostResponse {
                available,
                docker_version: "28.0.0".into(),
                api_version: "1.49".into(),
                operating_system: "linux".into(),
                architecture: "amd64".into(),
                build_kit_version: buildkit.into(),
            }
            .encode_to_vec(),
        );
        session.complete(command_id, true);
        if !abort_caller {
            assert_eq!(client.await.unwrap().status(), StatusCode::OK);
        }
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            loop {
                if builds.store().get_pool(id).await.unwrap().control_state == "Idle" {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap();
        let stored = builds.store().get_pool(id).await.unwrap();
        assert_eq!(
            stored.last_validation_status,
            if available { "Ready" } else { "Invalid" }
        );
        assert!(stored.last_validated_at.is_some());
        assert!(stored.control_triggered_by.is_none());
        if available {
            assert_eq!(
                stored.last_validation_message.as_deref(),
                Some(if buildkit.is_empty() {
                    "Docker 28.0.0 - API 1.49 - linux/amd64"
                } else {
                    "Docker 28.0.0 - API 1.49 - linux/amd64 - BuildKit v0.20.2"
                })
            );
        }
    }
    edge.disconnect(&target);
    let unavailable =
        response_json(request(app, Method::POST, &uri, Some(principal.clone()), None).await).await;
    assert_eq!(unavailable["lastValidationStatus"], "Invalid");
    let activities: i64 = sqlx::query_scalar("SELECT count(*) FROM activityevents WHERE resourceid=$1 AND eventtype='BuildAgentPoolTested'").bind(id).fetch_one(db).await.unwrap();
    assert_eq!(activities, 5);
    // A late result cannot finish a replacement claim after crash recovery.
    let stale = builds
        .store()
        .claim_pool_test(id, principal.actor_id)
        .await
        .unwrap();
    sqlx::query("UPDATE buildagentpools SET controlstartedat=0 WHERE id=$1")
        .bind(id)
        .execute(db)
        .await
        .unwrap();
    let current = builds
        .store()
        .claim_pool_test(id, principal.actor_id)
        .await
        .unwrap();
    let result = citadel_builds::BuildPoolCheck {
        ready: false,
        message: "Recovered test".into(),
    };
    assert!(
        builds
            .store()
            .finish_pool_test(&stale, &result)
            .await
            .is_err()
    );
    builds
        .store()
        .finish_pool_test(&current, &result)
        .await
        .unwrap();
    sqlx::query("UPDATE buildagentpools SET provider='AwsEc2',providerspec='{\"$type\":\"AwsEc2\"}' WHERE id=$1").bind(id).execute(db).await.unwrap();
    assert_eq!(
        request(app, Method::POST, &uri, Some(principal.clone()), None)
            .await
            .status(),
        StatusCode::CONFLICT
    );
    sqlx::query("UPDATE buildagentpools SET provider='SelfManagedVm',providerspec='{\"$type\":\"SelfManagedVm\",\"connectionMode\":\"EdgeAgent\"}' WHERE id=$1").bind(id).execute(db).await.unwrap();
}

async fn verify_capabilities(
    app: &Router,
    db: &sqlx::PgPool,
    principal: &ActorPrincipal,
    id: Uuid,
) {
    let mut reader = principal.clone();
    reader.actor_id = ActorId::new(Uuid::now_v7());
    reader.roles.clear();
    sqlx::query("INSERT INTO actors(id,isenabled,type) VALUES($1,true,'System')")
        .bind(reader.actor_id.value())
        .execute(db)
        .await
        .unwrap();
    sqlx::query("INSERT INTO resourceaccesses(id,actorid,permissionlevel,resourceid,resourcetype,specificpermissions) VALUES($1,$2,1,$3,$4,0)")
        .bind(Uuid::now_v7()).bind(reader.actor_id.value()).bind(id).bind(citadel_domain::ResourceType::BuildAgentPool as i32).execute(db).await.unwrap();
    let response = request(
        app,
        Method::GET,
        "/api/v1/buildAgentPools",
        Some(reader.clone()),
        None,
    )
    .await;
    let status = response.status();
    let value = response_json(response).await;
    assert_eq!(status, StatusCode::OK, "{value}");
    assert_eq!(value["pools"].as_array().unwrap().len(), 1);
    assert_eq!(value["pools"][0]["id"], id.to_string());
    assert_eq!(
        value["pools"][0]["capabilities"],
        json!({"canRead":true,"canWrite":false,"canExecute":false})
    );
    assert_eq!(value["capabilities"]["canWrite"], false);
    let detail = response_json(
        request(
            app,
            Method::GET,
            &format!("/api/v1/buildAgentPools/{id}"),
            Some(reader.clone()),
            None,
        )
        .await,
    )
    .await;
    assert_eq!(detail["capabilities"], value["pools"][0]["capabilities"]);
    // Write may configure/enroll a pool, but only Execute may revoke its Agent.
    sqlx::query("UPDATE resourceaccesses SET permissionlevel=2 WHERE actorid=$1")
        .bind(reader.actor_id.value())
        .execute(db)
        .await
        .unwrap();
    assert_eq!(
        request(
            app,
            Method::POST,
            &format!("/api/v1/buildAgentPools/{id}/edge/revoke"),
            Some(reader),
            None
        )
        .await
        .status(),
        StatusCode::FORBIDDEN
    );
}

// BuildEndpointTests.BuildAgentPoolEndpoints_ShouldPersistLifecycleAndEdgeEnrollment:
// partial configuration, separate rename and metadata contracts, and atomic activity.
async fn verify_edits(
    app: &Router,
    db: &sqlx::PgPool,
    builds: &Arc<BuildService>,
    principal: &ActorPrincipal,
    id: Uuid,
) {
    let uri = format!("/api/v1/buildAgentPools/{id}");
    let old = builds.store().get_pool(id).await.unwrap();
    let mut denied = principal.clone();
    denied.roles.clear();
    denied.actor_id = ActorId::new(Uuid::now_v7());
    for (method, path, body) in [
        (
            Method::PATCH,
            uri.clone(),
            json!({"description":"updated pool","maxActiveBuilders":3}),
        ),
        (
            Method::POST,
            "/api/v1/buildAgentPools/rename".into(),
            json!({"id":id,"name":format!("renamed-{id}")}),
        ),
        (
            Method::PATCH,
            format!("{uri}/_metadata"),
            json!({"description":"metadata pool"}),
        ),
    ] {
        assert_eq!(
            request(app, method.clone(), &path, None, Some(body.clone()))
                .await
                .status(),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            request(
                app,
                method.clone(),
                &path,
                Some(denied.clone()),
                Some(body.clone())
            )
            .await
            .status(),
            StatusCode::FORBIDDEN
        );
        let response = request(app, method, &path, Some(principal.clone()), Some(body)).await;
        let status = response.status();
        let value = response_json(response).await;
        assert_eq!(status, StatusCode::OK, "{value}");
    }
    let current = builds.store().get_pool(id).await.unwrap();
    assert_eq!(current.name, format!("renamed-{id}"));
    assert_eq!(current.description.as_deref(), Some("metadata pool"));
    assert_eq!(current.max_active_builders, 3);
    assert_eq!(current.provider_spec, old.provider_spec);
    assert_eq!(current.queue_timeout_seconds, old.queue_timeout_seconds);
    assert_eq!(current.last_validation_status, old.last_validation_status);
    let snapshot: Value = sqlx::query_scalar("SELECT info::jsonb FROM activityevents WHERE resourceid=$1 AND eventtype='BuildAgentPoolUpdated' ORDER BY createdat DESC LIMIT 1")
        .bind(id).fetch_one(db).await.unwrap();
    assert_eq!(snapshot["NewPool"]["Description"], "metadata pool");
    assert_eq!(snapshot["NewPool"]["MaxActiveBuilders"], 3);
    // A stale editor must not overwrite a newer save.
    let input = old
        .apply_patch(json!({"description":"lost update"}), false)
        .unwrap();
    assert!(matches!(
        builds
            .store()
            .update_pool(&old, &input, principal.actor_id)
            .await,
        Err(citadel_builds::BuildError::Conflict(_))
    ));
    // Reject unsupported/read-only fields, malformed values, and busy mutation.
    for patch in [
        json!({"maxActiveBuilders":0}),
        json!({"providerSpec":{"$type":"GenericEdge"}}),
        json!({"name":"bypass rename"}),
        json!({"description":"x".repeat(601)}),
    ] {
        assert_eq!(
            request(
                app,
                Method::PATCH,
                &uri,
                Some(principal.clone()),
                Some(patch)
            )
            .await
            .status(),
            StatusCode::BAD_REQUEST
        );
    }
    let claim = builds
        .store()
        .claim_pool_test(id, principal.actor_id)
        .await
        .unwrap();
    assert_eq!(
        request(
            app,
            Method::PATCH,
            &uri,
            Some(principal.clone()),
            Some(json!({"enabled":false}))
        )
        .await
        .status(),
        StatusCode::CONFLICT
    );
    builds
        .store()
        .finish_pool_test(
            &claim,
            &citadel_builds::BuildPoolCheck {
                ready: true,
                message: "Ready".into(),
            },
        )
        .await
        .unwrap();
    assert_eq!(
        request(
            app,
            Method::PATCH,
            &format!("{uri}/_metadata"),
            Some(principal.clone()),
            Some(json!({"description":null}))
        )
        .await
        .status(),
        StatusCode::OK
    );
    assert!(
        builds
            .store()
            .get_pool(id)
            .await
            .unwrap()
            .description
            .is_none()
    );
}
