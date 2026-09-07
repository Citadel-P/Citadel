use super::*;
use citadel_swarm_services::{ServiceImageDigestPort, SwarmServiceStore};
use std::sync::atomic::{AtomicU8, Ordering};

#[derive(Default)]
pub(super) struct DigestFixture {
    mode: AtomicU8,
    started: tokio::sync::Notify,
}
impl ServiceImageDigestPort for DigestFixture {
    fn digest<'a>(
        &'a self,
        _: Uuid,
        _: Uuid,
        reference: &'a str,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<String, SwarmServiceError>> {
        Box::pin(async move {
            assert_eq!(reference, "redis:7-alpine");
            match self.mode.load(Ordering::Relaxed) {
                0 => Ok(format!("sha256:{}", "a".repeat(64))),
                1 => Err(SwarmServiceError::Conflict(
                    "The platform is unavailable.".into(),
                )),
                2 => Err(SwarmServiceError::Runtime(
                    "upstream echoed credential: must-not-leak".into(),
                )),
                _ => {
                    self.started.notify_one();
                    cancel.cancelled().await;
                    Err(SwarmServiceError::Cancelled)
                }
            }
        })
    }
}

// Ports the three ManagedSwarmServiceEndpointTests CheckUpdates cases and
// UpdateCheckCommandTests cancellation regression, with durable lease fencing
// and lost-request/restart coverage beyond the original in-process lease.
pub(super) async fn exercise_update_checks(
    app: &Router,
    services: &Arc<ManagedSwarmServiceService>,
    pool: &sqlx::PgPool,
    admin: &ActorPrincipal,
    id: Uuid,
    digests: &Arc<DigestFixture>,
) {
    let url = format!("/api/v1/swarmServices/{id}/check-updates");
    assert_eq!(
        request(app, Method::POST, &url, None, None).await.status(),
        StatusCode::UNAUTHORIZED
    );
    let denied_actor = Uuid::now_v7();
    sqlx::query("INSERT INTO actors(id,type,isenabled) VALUES($1,'User',true)")
        .bind(denied_actor)
        .execute(pool)
        .await
        .unwrap();
    let mut denied = admin.clone();
    denied.actor_id = ActorId::new(denied_actor);
    denied.roles.clear();
    assert_eq!(
        request(app, Method::POST, &url, Some(denied), None)
            .await
            .status(),
        StatusCode::FORBIDDEN
    );
    sqlx::query("DELETE FROM actors WHERE id=$1")
        .bind(denied_actor)
        .execute(pool)
        .await
        .unwrap();
    for (mode, expected_status) in [
        (0, StatusCode::OK),
        (1, StatusCode::CONFLICT),
        (2, StatusCode::BAD_GATEWAY),
    ] {
        digests.mode.store(mode, Ordering::Relaxed);
        let response = request(app, Method::POST, &url, Some(admin.clone()), None).await;
        let status = response.status();
        let body = response_json(response).await;
        assert_eq!(status, expected_status, "{body}");
        assert!(!body.to_string().contains("must-not-leak"));
        let persisted = services.get(admin.actor_id, true, id).await.unwrap();
        assert_eq!(persisted.control_state, "Idle");
        assert_eq!(
            persisted.current_operation.as_ref().unwrap().state,
            "Completed"
        );
        assert_eq!(
            persisted.applied_image_digest.as_deref(),
            Some("sha256:fixture")
        );
        assert_eq!(
            persisted.auto_update_state.current_digest.as_deref(),
            Some("sha256:fixture")
        );
        assert_eq!(
            persisted.auto_update_state.status,
            if mode == 2 {
                "Failed"
            } else {
                "UpdateAvailable"
            }
        );
        assert_eq!(persisted.auto_update_state.last_error.is_some(), mode == 2);
    }
    digests.mode.store(3, Ordering::Relaxed);
    let task = {
        let app = app.clone();
        let admin = admin.clone();
        let url = url.clone();
        tokio::spawn(async move { request(&app, Method::POST, &url, Some(admin), None).await })
    };
    tokio::time::timeout(
        std::time::Duration::from_secs(3),
        digests.started.notified(),
    )
    .await
    .unwrap();
    assert_eq!(
        services
            .get(admin.actor_id, true, id)
            .await
            .unwrap()
            .control_state,
        "Processing"
    );
    assert_eq!(
        request(app, Method::POST, &url, Some(admin.clone()), None)
            .await
            .status(),
        StatusCode::CONFLICT
    );
    let busy_apply = response_json(
        request(
            app,
            Method::POST,
            &format!("/api/v1/swarmServices/{id}/apply"),
            Some(admin.clone()),
            None,
        )
        .await,
    )
    .await;
    assert_eq!(busy_apply[0]["stage"], "Failed");
    assert!(
        busy_apply[0]["errorMessage"]
            .as_str()
            .unwrap()
            .contains("operation in progress"),
        "{busy_apply}"
    );
    task.abort();
    let _ = task.await;
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        while services
            .get(admin.actor_id, true, id)
            .await
            .unwrap()
            .control_state
            != "Idle"
        {
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
    })
    .await
    .unwrap();

    let store = PostgresSwarmServiceStore::new(pool.clone());
    let expected = services.get(admin.actor_id, true, id).await.unwrap();
    let first = store
        .begin_update_check(admin.actor_id, true, &expected)
        .await
        .unwrap();
    sqlx::query("UPDATE swarmservices SET controlstartedat=$2 WHERE id=$1")
        .bind(id)
        .bind(chrono::Utc::now().timestamp() - 120)
        .execute(pool)
        .await
        .unwrap();
    let cutoff = chrono::Utc::now().timestamp() - 60;
    assert!(
        store
            .stale_deletion_claims(cutoff, 25)
            .await
            .unwrap()
            .is_empty(),
        "an interrupted read must never delete Docker Services"
    );
    assert!(
        store
            .stale_operation_claims(cutoff, 25)
            .await
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        store.recover_update_checks(cutoff, 25).await.unwrap(),
        vec![id]
    );
    let expected = services.get(admin.actor_id, true, id).await.unwrap();
    let second = store
        .begin_update_check(admin.actor_id, true, &expected)
        .await
        .unwrap();
    assert!(
        store.complete_update_check(&first, None).await.is_err(),
        "old cleanup must not release a replacement check"
    );
    assert_eq!(
        services
            .get(admin.actor_id, true, id)
            .await
            .unwrap()
            .control_state,
        "Processing"
    );
    store.complete_update_check(&second, None).await.unwrap();
    let persisted = services.get(admin.actor_id, true, id).await.unwrap();
    assert_eq!(
        persisted.current_operation.as_ref().unwrap().state,
        "Completed"
    );
    assert_eq!(persisted.control_state, "Idle");
}
