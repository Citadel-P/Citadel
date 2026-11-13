use super::*;
use citadel_alerts::AlertRepository;
use citadel_updates::ReleaseCheckRepository;

#[tokio::test]
#[ignore = "requires a fresh disposable CITADEL_EXECUTION_DATABASE_URL"]
async fn update_notices_require_an_administrator_even_with_alert_permissions() {
    let _fixture = FIXTURE_LOCK.lock().await;
    let url = std::env::var("CITADEL_EXECUTION_DATABASE_URL").unwrap();
    assert!(
        url::Url::parse(&url)
            .unwrap()
            .path()
            .starts_with("/citadel_update_test_")
    );
    MigrationRunner::migrate(&url).await.unwrap();
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .connect(&url)
        .await
        .unwrap();
    let admin = seed_administrator(&pool).await;
    let reader = seed_regular_user(&pool).await;
    let role = Uuid::now_v7();
    sqlx::query("INSERT INTO roles(id,name,roletype) VALUES($1,'Alert manager','Custom')")
        .bind(role)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO actorroles(actorid,roleid) VALUES($1,$2)")
        .bind(reader.actor_id.value())
        .bind(role)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO permissions(id,permissionlevel,resourcetype,roleid,specificpermissions) VALUES($1,2,$2,$3,0)")
        .bind(Uuid::now_v7()).bind(ResourceType::Alert as i32).bind(role).execute(&pool).await.unwrap();
    let identity = Arc::new(IdentityService::new(
        Arc::new(PostgresIdentityStore::new(pool.clone())),
        Arc::new(citadel_identity::CredentialWork::new(Arc::new(
            Argon2PasswordHasher::default(),
        ))),
        Arc::new(
            JwtSessionTokenCodec::new(&[83_u8; 32], "fixture".into(), "fixture".into()).unwrap(),
        ),
        Arc::new(OpaqueServiceAccountTokenCodec),
        Arc::new(StaticEntitlementService::new(false)),
        Arc::new(SystemClock),
        Arc::new(NoopServiceAccountLastUsedTracker),
        Duration::minutes(15),
        Duration::days(30),
    ));
    let store = Arc::new(PostgresAlertRepository::new(pool.clone()));
    let updates = citadel_adapters::persistence::postgres::updates::PostgresUpdateRepository::new(
        pool.clone(),
    );
    updates
        .record_check(Ok(None), chrono::Utc::now())
        .await
        .unwrap();
    updates
        .observe_core_availability("0.1.0", Some("0.1.1"))
        .await
        .unwrap();
    while store
        .process_pending_observation(Uuid::now_v7())
        .await
        .unwrap()
    {}
    let id: Uuid =
        sqlx::query_scalar("SELECT id FROM alertevents WHERE type='CoreUpdateAvailable'")
            .fetch_one(&pool)
            .await
            .unwrap();
    let app = alerts_http::router(AlertsHttpState {
        identity,
        store: store.clone(),
        delivery: Arc::new(FakeAlertDelivery),
    });
    for (method, path, body) in [
        (Method::GET, format!("/api/v1/alertEvents/{id}"), None),
        (
            Method::POST,
            "/api/v1/alertEvents/acknowledge".into(),
            Some(json!({"ids":[id]})),
        ),
        (
            Method::POST,
            "/api/v1/alertEvents/resolve".into(),
            Some(json!({"ids":[id],"resolutionNote":"test"})),
        ),
    ] {
        assert_eq!(
            request(&app, method, &path, Some(reader.clone()), body)
                .await
                .status(),
            StatusCode::FORBIDDEN
        );
    }
    let event = store.get_event(id).await.unwrap();
    assert!(event.acknowledged_at.is_none() && event.resolved_at.is_none());
    for (actor, count) in [(reader.clone(), 0), (admin.clone(), 1)] {
        let page = response_json(
            request(
                &app,
                Method::GET,
                "/api/v1/alertEvents",
                Some(actor.clone()),
                None,
            )
            .await,
        )
        .await;
        assert_eq!(page["pagedResult"]["totalCount"], count);
        let unresolved = response_json(
            request(
                &app,
                Method::GET,
                "/api/v1/alertEvents/unresolved-count",
                Some(actor),
                None,
            )
            .await,
        )
        .await;
        assert_eq!(unresolved["count"], count);
    }
    let event = response_json(
        request(
            &app,
            Method::GET,
            &format!("/api/v1/alertEvents/{id}"),
            Some(admin.clone()),
            None,
        )
        .await,
    )
    .await;
    assert_eq!(event["severity"], "Info");
    assert_eq!(event["resourcePath"], "/updates");
    assert_eq!(
        request(
            &app,
            Method::POST,
            "/api/v1/alertEvents/acknowledge",
            Some(admin),
            Some(json!({"ids":[id]}))
        )
        .await
        .status(),
        StatusCode::NO_CONTENT
    );
    assert!(store.get_event(id).await.unwrap().acknowledged_at.is_some());
    pool.close().await;
}
