use super::*;
use citadel_backups::BackupEntitlements;
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Default)]
pub struct Entitlement(AtomicBool);
impl BackupEntitlements for Entitlement {
    fn automated_operations(&self) -> BoxFuture<'_, Result<bool, BackupError>> {
        Box::pin(async { Ok(self.0.load(Ordering::Relaxed)) })
    }
}

pub fn router(
    pool: sqlx::PgPool,
    identity: Arc<IdentityService>,
    backups: Arc<BackupService>,
    builds: Arc<BuildService>,
    stacks: Arc<citadel_stacks::StackService>,
) -> Router {
    use citadel_adapters::{
        automation_store::PostgresAutomationStore,
        automation_token::IdentityAutomationRunTokenIssuer, crypto::AesGcmSecretProtector,
        postgres::git::accounts::PostgresGitAccountRepository,
        postgres::git::repositories::PostgresGitRepositoryExecutionPersistence,
    };
    let git = Arc::new(citadel_git::GitRepositoryExecutionService::new(
        Arc::new(PostgresGitRepositoryExecutionPersistence::new(pool.clone())),
        Arc::new(citadel_git::GitAccountService::new(
            Arc::new(PostgresGitAccountRepository::new(pool.clone())),
            Arc::new(AesGcmSecretProtector::new(&[59; 32]).unwrap()),
        )),
        Arc::new(citadel_git::GitCli::new(std::time::Duration::from_secs(5))),
        std::env::temp_dir().join(format!("webhook-git-{}", Uuid::now_v7())),
        std::time::Duration::from_secs(60),
    ));
    let automation = Arc::new(citadel_automation::AutomationService::new(
        Arc::new(PostgresAutomationStore::new(pool.clone())),
        Arc::new(IdentityAutomationRunTokenIssuer::new(identity)),
        citadel_automation::AutomationRuntimeConfig {
            deno_path: "deno".into(),
            work_root: std::env::temp_dir(),
            internal_base_url: "http://127.0.0.1:8000".into(),
            endpoint_catalog_json: "[]".into(),
            maximum_log_bytes: 1024,
            stale_after: std::time::Duration::from_secs(60),
        },
    ));
    citadel_server::webhooks_http::router(citadel_server::webhooks_http::WebhooksHttpState {
        git,
        automation,
        backups: Some(backups),
        builds: Some(builds),
        stacks: Some(stacks),
        services: None,
        audit: Some(Arc::new(
            citadel_adapters::activity_store::PostgresActivityStore::new(pool),
        )),
        alerts: None,
    })
}

pub async fn verify(
    app: &Router,
    pool: &sqlx::PgPool,
    backups: &BackupService,
    entitlement: &Entitlement,
    id: Uuid,
) {
    let config = json!({"enabled":true,"provider":"Generic","authScheme":"BearerToken","secret":"backup-webhook-test"});
    sqlx::query("UPDATE backuppolicies SET webhook=$2 WHERE id=$1")
        .bind(id)
        .bind(&config)
        .execute(pool)
        .await
        .unwrap();
    let url = format!("/listener/generic/backup-policy/{id}/run");
    for token in [None, Some("Bearer wrong")] {
        assert_eq!(
            send(app, &url, token).await.status(),
            StatusCode::UNAUTHORIZED
        );
    }
    let denied = send(app, &url, Some("Bearer backup-webhook-test")).await;
    assert_eq!(denied.status(), StatusCode::ACCEPTED);
    assert_eq!(response_json(denied).await["status"], "noop");
    assert_eq!(
        backups.store().get_policy(id).await.unwrap().control_state,
        "Idle"
    );

    entitlement.0.store(true, Ordering::Relaxed);
    let accepted = send(app, &url, Some("Bearer backup-webhook-test")).await;
    assert_eq!(accepted.status(), StatusCode::ACCEPTED);
    assert_eq!(response_json(accepted).await["status"], "queued");
    let run_id = backups
        .store()
        .get_policy(id)
        .await
        .unwrap()
        .current_run_id
        .unwrap();
    assert_eq!(
        backups.store().get_run(run_id).await.unwrap().trigger,
        "Webhook"
    );
    assert_eq!(
        response_json(send(app, &url, Some("Bearer backup-webhook-test")).await).await["status"],
        "noop"
    );
    assert_eq!(
        backups.store().get_policy(id).await.unwrap().current_run_id,
        Some(run_id)
    );
    // Expiration after acceptance cannot bypass the execution-time entitlement.
    entitlement.0.store(false, Ordering::Relaxed);
    assert!(
        backups
            .process_backup(&CancellationToken::new())
            .await
            .unwrap()
    );
    let failed = backups.store().get_run(run_id).await.unwrap();
    assert_eq!(failed.status, "Failed");
    assert!(failed.error_message.as_deref().unwrap().contains("license"));

    entitlement.0.store(true, Ordering::Relaxed);
    assert_eq!(
        response_json(send(app, &url, Some("Bearer backup-webhook-test")).await).await["status"],
        "queued"
    );
    let run_id = backups
        .store()
        .get_policy(id)
        .await
        .unwrap()
        .current_run_id
        .unwrap();
    assert!(
        backups
            .process_backup(&CancellationToken::new())
            .await
            .unwrap()
    );
    let completed = backups.store().get_run(run_id).await.unwrap();
    assert_eq!(completed.status, "Succeeded", "{completed:?}");
    sqlx::query("UPDATE backuppolicies SET webhook=NULL WHERE id=$1")
        .bind(id)
        .execute(pool)
        .await
        .unwrap();
    assert!(matches!(
        backups.store().enqueue_webhook(id, &config).await,
        Err(BackupError::Conflict(_))
    ));
    assert_eq!(
        send(app, &url, Some("Bearer backup-webhook-test"))
            .await
            .status(),
        StatusCode::NOT_FOUND
    );
    entitlement.0.store(false, Ordering::Relaxed);
}

async fn send(app: &Router, url: &str, token: Option<&str>) -> axum::response::Response {
    let mut request = Request::builder().method(Method::POST).uri(url);
    if let Some(token) = token {
        request = request.header("authorization", token);
    }
    app.clone()
        .oneshot(request.body(Body::from("{}")).unwrap())
        .await
        .unwrap()
}
