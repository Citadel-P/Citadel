//! Ports the shared-secret and body-limit cases from .NET WebhookListenerTests,
//! then exercises durable Automation dispatch through the real Deno worker.
use super::*;
use citadel_automation::{AutomationEntitlements, AutomationError};
use futures_util::future::BoxFuture;
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Default)]
pub struct Entitlement(AtomicBool);

impl AutomationEntitlements for Entitlement {
    fn automated_operations(&self) -> BoxFuture<'_, Result<bool, AutomationError>> {
        Box::pin(async { Ok(self.0.load(Ordering::Relaxed)) })
    }
}

pub fn router(pool: PgPool, automation: Arc<AutomationService>) -> Router {
    use citadel_adapters::{
        crypto::AesGcmSecretProtector, postgres::git::accounts::PostgresGitAccountRepository,
        postgres::git::repositories::PostgresGitRepositoryExecutionPersistence,
    };
    use citadel_git::{GitAccountService, GitCli, GitRepositoryExecutionService};
    let git = Arc::new(GitRepositoryExecutionService::new(
        Arc::new(PostgresGitRepositoryExecutionPersistence::new(pool.clone())),
        Arc::new(GitAccountService::new(
            Arc::new(PostgresGitAccountRepository::new(pool)),
            Arc::new(AesGcmSecretProtector::new(&[33; 32]).unwrap()),
        )),
        Arc::new(GitCli::new(Duration::from_secs(5))),
        std::env::temp_dir().join(format!("citadel-webhook-test-{}", Uuid::now_v7())),
        Duration::from_secs(60),
    ));
    citadel_server::webhooks_http::router(citadel_server::webhooks_http::WebhooksHttpState {
        git,
        automation,
        backups: None,
        builds: None,
        stacks: None,
        services: None,
        audit: None,
        alerts: None,
    })
}

pub async fn verify(
    app: &Router,
    admin: &ActorPrincipal,
    db: &PgPool,
    store: &PostgresAutomationStore,
    service: &AutomationService,
    license: &Entitlement,
) {
    let action = create(
        app,
        admin,
        "console.log('webhook received', args.message);",
        true,
        10,
    )
    .await;
    let id = Uuid::parse_str(action["id"].as_str().unwrap()).unwrap();
    // Older rows may contain a JSON null rather than a SQL NULL.
    set_config(db, id, &Value::Null).await;
    assert!(store.get(id).await.unwrap().webhook.is_none());
    let config = json!({"enabled":true,"provider":"Generic","authScheme":"BearerToken","secret":"fixture-webhook-secret"});
    set_config(db, id, &config).await;
    let url = format!("/listener/generic/automation-action/{id}/run");

    for token in [None, Some("Bearer wrong"), Some("Bearer ")] {
        assert_eq!(
            send(app, &url, token, Body::from("{}"), &[]).await.status(),
            StatusCode::UNAUTHORIZED
        );
    }
    assert!(store.list_runs(id, 10).await.unwrap().is_empty());
    assert_eq!(
        send(
            app,
            &url,
            Some("Bearer fixture-webhook-secret"),
            Body::from("{}"),
            &[("authorization", "Bearer second")]
        )
        .await
        .status(),
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        send(
            app,
            &format!("/listener/github/action/{id}/run"),
            Some("Bearer fixture-webhook-secret"),
            Body::from("{}"),
            &[]
        )
        .await
        .status(),
        StatusCode::BAD_REQUEST
    );

    // A valid credential does not bypass licensing or enqueue a denied job.
    let response = send(
        app,
        &url,
        Some("Bearer fixture-webhook-secret"),
        Body::from("{}"),
        &[],
    )
    .await;
    assert_eq!(response.status(), StatusCode::ACCEPTED);
    assert_eq!(body(response).await["status"], "noop");
    assert!(store.list_runs(id, 10).await.unwrap().is_empty());
    license.0.store(true, Ordering::Relaxed);

    // Covers both a fixed body and chunked input, without trusting Content-Length.
    for chunked in [false, true] {
        let large = vec![b'x'; 1024 * 1024 + 1];
        let payload = if chunked {
            Body::from_stream(futures_util::stream::iter([
                Ok::<_, std::convert::Infallible>(axum::body::Bytes::copy_from_slice(
                    &large[..512 * 1024],
                )),
                Ok(axum::body::Bytes::copy_from_slice(&large[512 * 1024..])),
            ]))
        } else {
            Body::from(large)
        };
        assert_eq!(
            send(
                app,
                &url,
                Some("Bearer fixture-webhook-secret"),
                payload,
                &[]
            )
            .await
            .status(),
            StatusCode::BAD_REQUEST
        );
    }
    assert!(store.list_runs(id, 10).await.unwrap().is_empty());

    let response = send(
        app,
        &url,
        Some("Bearer fixture-webhook-secret"),
        Body::from(r#"{"message":"from-ci"}"#),
        &[],
    )
    .await;
    assert_eq!(response.status(), StatusCode::ACCEPTED);
    assert_eq!(body(response).await["status"], "queued");
    let run = store.list_runs(id, 1).await.unwrap().remove(0);
    assert_eq!(run.status, "Queued");
    assert_eq!(run.trigger, "Webhook");
    assert_eq!(run.run_as_actor_id, admin.actor_id.value());
    assert_eq!(run.triggered_by_actor_id, None);
    assert_eq!(
        serde_json::from_str::<Value>(&run.args_json).unwrap(),
        json!({"message":"from-ci"})
    );
    // Repeated delivery cannot execute concurrently or replace the live claim.
    assert_eq!(
        body(
            send(
                app,
                &url,
                Some("Bearer fixture-webhook-secret"),
                Body::from("{}"),
                &[]
            )
            .await
        )
        .await["status"],
        "noop"
    );
    assert_eq!(store.get(id).await.unwrap().current_run_id, Some(run.id));
    assert!(
        service
            .process_one(&tokio_util::sync::CancellationToken::new())
            .await
            .unwrap()
    );
    let completed = store.get_run(id, run.id).await.unwrap();
    assert_eq!(completed.status, "Succeeded");
    assert!(completed.logs.unwrap().contains("webhook received from-ci"));
    let activity: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM activityevents WHERE resourceid=$1 AND eventtype='ActionRunQueued'",
    )
    .bind(id)
    .fetch_one(db)
    .await
    .unwrap();
    assert_eq!(activity, 1);

    let github = json!({"enabled":true,"provider":"GitHub","authScheme":"GitHubHmacSha256","branchFilter":"main"});
    set_config(db, id, &github).await;
    let github_url = format!("/listener/github/action/{id}/run");
    for (event, payload) in [
        ("push", r#"{"ref":"refs/heads/other"}"#),
        ("push", "{}"),
        ("ping", "{}"),
    ] {
        let response = send(
            app,
            &github_url,
            None,
            Body::from(payload),
            &[("x-github-event", event)],
        )
        .await;
        assert_eq!(response.status(), StatusCode::ACCEPTED);
        assert_eq!(body(response).await["status"], "noop");
    }
    // The authenticated snapshot is rechecked under the enqueue row lock.
    assert!(matches!(
        store
            .enqueue_webhook(
                id,
                &serde_json::from_value(config.clone()).unwrap(),
                &json!({})
            )
            .await,
        Err(AutomationError::Conflict(_))
    ));
    set_config(db, id, &json!({"enabled":false})).await;
    assert_eq!(
        send(app, &github_url, None, Body::from("{}"), &[])
            .await
            .status(),
        StatusCode::NOT_FOUND
    );
    // Worker and HTTP execution share one bounded semaphore, including across
    // different Actions. Excess work remains durably queued for the next pass.
    let mut queued = Vec::new();
    for _ in 0..=service.options().max_parallel_runs {
        let action = create(
            app,
            admin,
            "await new Promise(r=>setTimeout(r,500));",
            true,
            10,
        )
        .await;
        let action_id = Uuid::parse_str(action["id"].as_str().unwrap()).unwrap();
        queued.push(
            store
                .enqueue(admin.actor_id, action_id, "Manual", &json!({}), None)
                .await
                .unwrap(),
        );
    }
    let cancel = tokio_util::sync::CancellationToken::new();
    let results =
        futures_util::future::join_all((0..queued.len()).map(|_| service.process_one(&cancel)))
            .await;
    assert_eq!(
        results
            .into_iter()
            .filter(|result| matches!(result, Ok(true)))
            .count(),
        service.options().max_parallel_runs
    );
    let pending: i64 =
        sqlx::query_scalar("SELECT count(*) FROM actionruns WHERE id=ANY($1) AND status='Queued'")
            .bind(queued.iter().map(|run| run.id).collect::<Vec<_>>())
            .fetch_one(db)
            .await
            .unwrap();
    assert_eq!(pending, 1);
    assert!(service.process_one(&cancel).await.unwrap());
    license.0.store(false, Ordering::Relaxed);
}

async fn set_config(db: &PgPool, id: Uuid, config: &Value) {
    sqlx::query("UPDATE actions SET webhook=$2 WHERE id=$1")
        .bind(id)
        .bind(config)
        .execute(db)
        .await
        .unwrap();
}

async fn send(
    app: &Router,
    url: &str,
    token: Option<&str>,
    body: Body,
    headers: &[(&str, &str)],
) -> axum::response::Response {
    let mut request = Request::builder().method(Method::POST).uri(url);
    if let Some(token) = token {
        request = request.header("authorization", token);
    }
    for (key, value) in headers {
        request = request.header(*key, *value);
    }
    app.clone()
        .oneshot(request.body(body).unwrap())
        .await
        .unwrap()
}
