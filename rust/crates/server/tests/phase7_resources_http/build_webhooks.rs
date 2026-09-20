use super::*;
use citadel_builds::{BuildEntitlements, BuildError};
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Default)]
pub struct Entitlement(AtomicBool);
impl BuildEntitlements for Entitlement {
    fn enabled(
        &self,
        _: citadel_licensing::LicenseCapability,
    ) -> BoxFuture<'_, Result<bool, BuildError>> {
        Box::pin(async { Ok(self.0.load(Ordering::Relaxed)) })
    }
}

// Ports ReceiveWebhookTests' Build path filtering, busy/lost claim and committed
// queue cases through the real HTTP route and PostgreSQL transaction boundary.
pub async fn verify(
    app: &Router,
    pool: &sqlx::PgPool,
    builds: &BuildService,
    entitlement: &Entitlement,
    principal: &ActorPrincipal,
    fixture: &FixtureIds,
) {
    let project = response_json(request(app, Method::POST, "/api/v1/buildProjects", Some(principal.clone()), Some(json!({
        "name":format!("webhook-{}",Uuid::now_v7()),"enabled":true,
        "gitRepositoryId":fixture.git_repository,"branch":"main","contextPath":"src",
        "dockerfilePath":"Dockerfile","buildArgs":[],"buildSecrets":[],
        "builderKind":"Platform","platformId":fixture.platform,"registryId":fixture.registry,
        "imageRepository":"citadel/webhook-test","tagTemplates":["{branch}-{shortSha}"],"timeoutSeconds":60,"retentionRunCount":5
    }))).await).await;
    let id = Uuid::parse_str(project["id"].as_str().unwrap()).unwrap();
    let consumers = super::build_completion::create(pool, fixture, id).await;
    let config = json!({"enabled":true,"provider":"Generic","authScheme":"BearerToken","secret":"build-hook-test"});
    sqlx::query("UPDATE buildprojects SET webhook=$2 WHERE id=$1")
        .bind(id)
        .bind(&config)
        .execute(pool)
        .await
        .unwrap();
    let url = format!("/listener/generic/build/{id}/run");
    for token in [None, Some("Bearer wrong")] {
        assert_eq!(
            send(app, &url, token, json!({})).await.status(),
            StatusCode::UNAUTHORIZED
        );
    }
    assert_eq!(
        response_json(send(app, &url, Some("Bearer build-hook-test"), json!({})).await).await["status"],
        "noop"
    );
    assert!(builds.store().get(id).await.unwrap().latest_run.is_none());
    entitlement.0.store(true, Ordering::Relaxed);
    for payload in [
        json!({"changedPaths":["docs/readme.md"]}),
        json!({"branch":"other","changedPaths":["src/main.rs"]}),
        json!({"repository":"https://example.test/unrelated.git","changedPaths":["src/main.rs"]}),
    ] {
        assert_eq!(
            response_json(send(app, &url, Some("Bearer build-hook-test"), payload).await).await["status"],
            "noop"
        );
        assert!(builds.store().get(id).await.unwrap().latest_run.is_none());
    }
    let payload =
        json!({"branch":"main","changedPaths":["src/main.rs"],"commitSha":"a".repeat(40)});
    assert_eq!(
        send(
            app,
            &url,
            Some("Bearer build-hook-test"),
            json!({"commitSha":"--help"})
        )
        .await
        .status(),
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        response_json(send(app, &url, Some("Bearer build-hook-test"), payload.clone()).await).await
            ["status"],
        "queued"
    );
    let queued = builds.store().get(id).await.unwrap();
    let run = queued.latest_run.unwrap();
    assert_eq!(run.trigger, "Webhook");
    assert_eq!(run.branch, "main");
    assert_eq!(run.resolved_commit_sha, Some("a".repeat(40)));
    let audit: Value = sqlx::query_scalar("SELECT info::jsonb FROM activityevents WHERE resourceid=$1 AND eventtype='BuildWebhookReceived' AND info::jsonb->>'Status'='queued' ORDER BY createdat DESC LIMIT 1")
        .bind(id).fetch_one(pool).await.unwrap();
    assert_eq!(audit["Branch"], "main");
    assert_eq!(audit["DispatchedBranch"], run.branch);
    assert_eq!(
        audit["DispatchedCommitSha"],
        run.resolved_commit_sha.clone().unwrap()
    );
    let triggered_by: Uuid =
        sqlx::query_scalar("SELECT triggeredbyactorid FROM buildruns WHERE id=$1")
            .bind(run.id)
            .fetch_one(pool)
            .await
            .unwrap();
    assert_eq!(triggered_by, SYSTEM_ACTOR_ID);
    assert_eq!(
        response_json(send(app, &url, Some("Bearer build-hook-test"), payload.clone()).await).await
            ["status"],
        "noop"
    );
    assert_eq!(
        builds.store().get(id).await.unwrap().latest_run.unwrap().id,
        run.id
    );
    assert!(builds.process_one(&CancellationToken::new()).await.unwrap());
    assert_eq!(
        builds.store().get_run(run.id).await.unwrap().status,
        "Succeeded"
    );
    super::build_completion::verify(pool, run.id, &consumers).await;
    let events: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM activityevents WHERE resourceid=$1 AND eventtype='BuildRunQueued'",
    )
    .bind(id)
    .fetch_one(pool)
    .await
    .unwrap();
    assert_eq!(events, 1);
    let old = builds.store().get(id).await.unwrap();
    sqlx::query("UPDATE buildprojects SET webhook=NULL,rowversion=rowversion+1 WHERE id=$1")
        .bind(id)
        .execute(pool)
        .await
        .unwrap();
    assert!(matches!(
        builds.store().enqueue_webhook(&old, "main", None).await,
        Err(BuildError::Conflict(_))
    ));
    assert_eq!(
        send(app, &url, Some("Bearer build-hook-test"), payload)
            .await
            .status(),
        StatusCode::NOT_FOUND
    );
    super::build_completion::verify_lifecycle(pool, builds, fixture, id, run.id, &consumers).await;
    entitlement.0.store(false, Ordering::Relaxed);
}

async fn send(
    app: &Router,
    url: &str,
    token: Option<&str>,
    body: Value,
) -> axum::response::Response {
    let mut request = Request::builder().method(Method::POST).uri(url);
    if let Some(token) = token {
        request = request.header("authorization", token);
    }
    app.clone()
        .oneshot(
            request
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap()
}
