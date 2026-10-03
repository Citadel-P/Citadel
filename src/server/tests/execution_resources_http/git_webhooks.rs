use super::*;

pub async fn verify(app: &Router, pool: &sqlx::PgPool, fixture: &FixtureIds) {
    let id = fixture.git_repository;
    sqlx::query("UPDATE gitrepositories SET webhook=$2 WHERE id=$1")
        .bind(id).bind(json!({"enabled":true,"provider":"Generic","authScheme":"BearerToken","secret":"git-hook-fixture"}))
        .execute(pool).await.unwrap();
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!("/listener/generic/repo/{id}/pull"))
                .header("authorization", "Bearer git-hook-fixture")
                .header("content-type", "application/json")
                .body(Body::from(json!({"branch":"main"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::ACCEPTED);
    let branch: String = sqlx::query_scalar(
        "SELECT branch FROM gitrepositoryrefs WHERE gitrepositoryid=$1 AND status='Pending'",
    )
    .bind(id)
    .fetch_one(pool)
    .await
    .unwrap();
    let audit: Value = sqlx::query_scalar("SELECT info::jsonb FROM activityevents WHERE resourceid=$1 AND eventtype='GitRepoWebhookReceived' AND info::jsonb->>'Status'='queued' ORDER BY createdat DESC LIMIT 1")
        .bind(id).fetch_one(pool).await.unwrap();
    assert_eq!(audit["DispatchedBranch"], branch);
    assert!(
        audit["DispatchedCommitSha"].is_null(),
        "The commit is unknown until Git synchronization completes"
    );
    assert!(!audit.to_string().contains("git-hook-fixture"));
}
