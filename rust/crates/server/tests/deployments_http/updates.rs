//! Ports CheckDeploymentUpdates/DeploymentAutoUpdateJob tests with real HTTP
//! authorization and PostgreSQL leases, plus cancellation and late-result races.
use super::*;
use citadel_deployments::{DeploymentEntitlementPort, DeploymentStore};
use std::sync::atomic::{AtomicU8, AtomicUsize};

pub(super) async fn verify_automatic_apply(
    service: &DeploymentService,
    pool: &sqlx::PgPool,
    admin: &ActorPrincipal,
    id: Uuid,
    fail: &AtomicBool,
) {
    let store = PostgresDeploymentStore::new(pool.clone());
    for (should_fail, expected) in [(true, "Failed"), (false, "UpToDate")] {
        sqlx::query("UPDATE deployments SET spec=jsonb_set(jsonb_set(spec::jsonb,'{UpdateBehavior}','\"AutoDeploy\"'::jsonb),'{Image,ResolvedDigest}',to_jsonb($2::text))::json WHERE id=$1")
            .bind(id).bind(format!("sha256:{}", "a".repeat(64))).execute(pool).await.unwrap();
        fail.store(should_fail, Ordering::Release);
        service
            .run_scheduled_update_checks(&CancellationToken::new())
            .await
            .unwrap();
        let current = store
            .get_authorized(admin.actor_id, true, id)
            .await
            .unwrap();
        assert_eq!(current.control_state, "Idle");
        let update = current.auto_update_state.unwrap();
        assert_eq!(update.status, expected);
        assert_eq!(update.last_error.is_some(), should_fail);
        assert_eq!(
            current.status,
            if should_fail { "Failed" } else { "Healthy" }
        );
        if !should_fail {
            assert_eq!(update.current_digest, update.remote_digest);
        }
    }
    sqlx::query("DELETE FROM containers WHERE deploymentid=$1")
        .bind(id)
        .execute(pool)
        .await
        .unwrap();
}

#[derive(Default)]
pub(super) struct Digests {
    mode: AtomicU8,
    calls: AtomicUsize,
    started: tokio::sync::Notify,
}
impl Digests {
    pub(super) fn read<'a>(
        &'a self,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<String, DeploymentError>> {
        Box::pin(async move {
            self.calls.fetch_add(1, Ordering::Relaxed);
            match self.mode.load(Ordering::Relaxed) {
                0 => Ok(format!("sha256:{}", "a".repeat(64))),
                1 => Ok(format!("sha256:{}", "b".repeat(64))),
                2 => Err(DeploymentError::Runtime(
                    "upstream-secret-must-not-leak".into(),
                )),
                _ => {
                    self.started.notify_one();
                    cancel.cancelled().await;
                    Err(DeploymentError::Cancelled)
                }
            }
        })
    }
}
pub(super) struct Entitlements(AtomicBool);
impl Entitlements {
    pub(super) fn new() -> Self {
        Self(AtomicBool::new(true))
    }
}
impl DeploymentEntitlementPort for Entitlements {
    fn enabled(
        &self,
        _: citadel_domain::LicenseCapability,
    ) -> BoxFuture<'_, Result<bool, DeploymentError>> {
        Box::pin(async { Ok(self.0.load(Ordering::Relaxed)) })
    }
}

pub(super) async fn verify(
    app: &Router,
    service: &Arc<DeploymentService>,
    pool: &sqlx::PgPool,
    admin: &ActorPrincipal,
    id: Uuid,
    digests: &Arc<Digests>,
    entitlement: &Entitlements,
) {
    let url = format!("/api/v1/deployments/{id}/check-updates");
    assert_eq!(
        request(app, Method::POST, &url, None, None).await.status(),
        StatusCode::UNAUTHORIZED
    );
    let mut denied = admin.clone();
    denied.roles.clear();
    denied.actor_id = ActorId::new(Uuid::now_v7());
    assert_eq!(
        request(app, Method::POST, &url, Some(denied), None)
            .await
            .status(),
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        request(app, Method::POST, &url, Some(admin.clone()), None)
            .await
            .status(),
        StatusCode::CONFLICT
    );
    assert_eq!(digests.calls.load(Ordering::Relaxed), 0);
    sqlx::query("UPDATE deployments SET spec=jsonb_set(spec::jsonb,'{Image,ResolvedDigest}',to_jsonb($2::text))::json WHERE id=$1")
        .bind(id).bind(format!("sha256:{}", "a".repeat(64))).execute(pool).await.unwrap();
    for (mode, expected) in [(0, "UpToDate"), (1, "UpdateAvailable"), (2, "Failed")] {
        digests.mode.store(mode, Ordering::Relaxed);
        let response = request(app, Method::POST, &url, Some(admin.clone()), None).await;
        assert_eq!(
            response.status(),
            if mode == 2 {
                StatusCode::BAD_GATEWAY
            } else {
                StatusCode::OK
            }
        );
        assert!(
            !response_json(response)
                .await
                .to_string()
                .contains("upstream-secret")
        );
        let stored: (String, String, Option<String>, Option<Uuid>) = sqlx::query_as("SELECT controlstate,autoupdatestate_status,autoupdatestate_lasterror,updatecheckid FROM deployments WHERE id=$1").bind(id).fetch_one(pool).await.unwrap();
        assert_eq!(stored.0, "Idle");
        assert_eq!(stored.1, expected);
        assert_eq!(stored.2.is_some(), mode == 2);
        assert_eq!(stored.3, None);
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
        request(app, Method::POST, &url, Some(admin.clone()), None)
            .await
            .status(),
        StatusCode::CONFLICT
    );
    task.abort();
    let _ = task.await;
    tokio::time::timeout(std::time::Duration::from_secs(3), async {
        loop {
            if sqlx::query_scalar::<_, Option<Uuid>>(
                "SELECT updatecheckid FROM deployments WHERE id=$1",
            )
            .bind(id)
            .fetch_one(pool)
            .await
            .unwrap()
            .is_none()
            {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    let store = PostgresDeploymentStore::new(pool.clone());
    let snapshot = store
        .get_authorized(admin.actor_id, true, id)
        .await
        .unwrap();
    let lease = store
        .begin_update_check(admin.actor_id, true, &snapshot)
        .await
        .unwrap();
    assert!(store.claim_apply(admin.actor_id, true, id).await.is_err());
    sqlx::query("UPDATE deployments SET controlstartedat=1 WHERE id=$1")
        .bind(id)
        .execute(pool)
        .await
        .unwrap();
    assert_eq!(service.recover_update_checks().await.unwrap(), 1);
    assert!(
        store.complete_update_check(&lease, None).await.is_err(),
        "late completion cannot take a recovered lease"
    );
    assert!(
        store
            .claim_apply_versioned(admin.actor_id, true, id, Some(snapshot.row_version))
            .await
            .is_err(),
        "automatic Apply fences its checked configuration"
    );
    sqlx::query("UPDATE deployments SET spec=jsonb_set(spec::jsonb,'{UpdateBehavior}', '\"Notify\"'::jsonb)::json WHERE id=$1").bind(id).execute(pool).await.unwrap();
    digests.mode.store(1, Ordering::Relaxed);
    let calls = digests.calls.load(Ordering::Relaxed);
    entitlement.0.store(false, Ordering::Relaxed);
    service
        .run_scheduled_update_checks(&CancellationToken::new())
        .await
        .unwrap();
    assert_eq!(
        digests.calls.load(Ordering::Relaxed),
        calls + 1,
        "Community still detects updates"
    );
    entitlement.0.store(true, Ordering::Relaxed);
    service
        .run_scheduled_update_checks(&CancellationToken::new())
        .await
        .unwrap();
    assert_eq!(digests.calls.load(Ordering::Relaxed), calls + 2);
    let checked = store
        .get_authorized(admin.actor_id, true, id)
        .await
        .unwrap();
    assert_eq!(checked.auto_update_state.unwrap().status, "UpdateAvailable");
    assert_eq!(checked.status, "Created", "Notify must not Apply");
    assert!(
        store
            .scheduled_update_candidates(id, 25)
            .await
            .unwrap()
            .iter()
            .all(|next| *next > id)
    );
    sqlx::query("UPDATE deployments SET spec=jsonb_set(spec::jsonb,'{UpdateBehavior}', '\"Disabled\"'::jsonb)::json WHERE id=$1").bind(id).execute(pool).await.unwrap();
}
