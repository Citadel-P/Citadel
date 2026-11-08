use super::*;

#[tokio::test]
#[ignore = "requires disposable CITADEL_IDENTITY_DATABASE_URL"]
async fn instance_configuration_patches_are_atomic_audited_and_deployment_restricted() {
    let base = std::env::var("CITADEL_IDENTITY_DATABASE_URL").unwrap();
    let admin = PgPool::connect(&base).await.unwrap();
    let name = format!("citadel_instance_settings_{}", Uuid::now_v7().simple());
    sqlx::query(sqlx::AssertSqlSafe(format!("CREATE DATABASE {name}")))
        .execute(&admin)
        .await
        .unwrap();
    let mut url = url::Url::parse(&base).unwrap();
    url.set_path(&name);
    let fixture = fixture_at(MfaPolicy::Optional, url.as_str()).await;
    sqlx::query("UPDATE instancesetupstates SET initializedat=now(), initialadministratoractorid=$1 WHERE id=1")
        .bind(fixture.principal.actor_id.value())
        .execute(&fixture.pool)
        .await
        .unwrap();
    let mut notification = fixture.instance.installation_changes();
    let defaults =
        serde_json::to_value(citadel_instance::InstanceConfiguration::default()).unwrap();
    let mut configuration = defaults.clone();
    for (key, value) in configuration.as_object_mut().unwrap() {
        *value = match &*value {
            Value::Bool(v) => json!(!v),
            Value::Number(v) => json!(v.as_u64().unwrap() + 1),
            Value::String(_) if key == "logFormat" => json!("Json"),
            Value::String(_) if key == "logVerbosity" => json!("debug"),
            _ => json!("registry.example.test/citadel/agent:test"),
        };
    }
    let path = "/api/v1/installation/settings";
    let partial = send_json(
        &fixture.app,
        Method::PATCH,
        path,
        json!({"configuration":{"passwordMinimumLength":20}}),
        Some(fixture.principal.clone()),
        None,
    )
    .await;
    assert_eq!(partial.status(), StatusCode::OK);
    let stored_choices: Value =
        sqlx::query_scalar("SELECT instanceconfiguration FROM instancesetupstates WHERE id=1")
            .fetch_one(&fixture.pool)
            .await
            .unwrap();
    assert_eq!(
        stored_choices,
        json!({"passwordMinimumLength":20}),
        "unselected defaults must remain fallbacks"
    );
    let mut previous = defaults.clone();
    previous["passwordMinimumLength"] = json!(20);
    notification.borrow_and_update();
    let response = send_json(
        &fixture.app,
        Method::PATCH,
        path,
        json!({"configuration": configuration}),
        Some(fixture.principal.clone()),
        None,
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let saved = json_body(response).await;
    assert_eq!(saved["configuration"], configuration);
    assert_eq!(saved["runningConfiguration"], defaults);
    assert_eq!(saved["restartRequired"], true);
    assert!(notification.has_changed().unwrap());
    notification.borrow_and_update();
    let audit: Value = sqlx::query_scalar("SELECT info::jsonb FROM activityevents WHERE eventtype='InstallationSettingsUpdated' ORDER BY createdat DESC LIMIT 1").fetch_one(&fixture.pool).await.unwrap();
    assert_eq!(audit["PreviousConfiguration"], previous);
    assert_eq!(audit["Configuration"], configuration);
    for invalid in [
        json!({"configuration":null}),
        json!({"configuration":{}}),
        json!({"configuration":{"unknown":true}}),
        json!({"configuration":{"scriptsEnabled":null}}),
        json!({"configuration":{"scriptsMaximumTimeoutSeconds":1}}),
        json!({"configuration":{"serviceAccountMaximumTokenLifetimeDays":1}}),
        json!({"configuration":{"passwordMinimumLength":7}}),
    ] {
        let response = send_json(
            &fixture.app,
            Method::PATCH,
            path,
            invalid,
            Some(fixture.principal.clone()),
            None,
        )
        .await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert!(!notification.has_changed().unwrap());
    }
    let persisted = fixture
        .instance
        .installation_settings()
        .await
        .unwrap()
        .configuration;
    assert_eq!(serde_json::to_value(&persisted).unwrap(), configuration);
    let overrides = citadel_instance::InstanceOverrides {
        scripts_enabled: Some(false),
        ..Default::default()
    };
    let restricted_instance = instance(
        &fixture.pool,
        citadel_instance::InstallationPolicy {
            running: persisted.effective(&overrides),
            overrides,
            ..Default::default()
        },
    );
    let app = router_with_instance(
        fixture.identity.clone(),
        fixture.pool.clone(),
        MfaPolicy::Optional,
        restricted_instance,
    );
    let response = send_json(
        &app,
        Method::PATCH,
        path,
        json!({"configuration":{"scriptsEnabled":true,"passwordMinimumLength":30}}),
        Some(fixture.principal.clone()),
        None,
    )
    .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let response = send_json(
        &app,
        Method::GET,
        path,
        json!(null),
        Some(fixture.principal.clone()),
        None,
    )
    .await;
    let current = json_body(response).await;
    assert_eq!(current["configuration"], configuration);
    assert_eq!(current["restartRequired"], false);
    assert_eq!(current["deploymentOverrides"]["scriptsEnabled"], false);
    // Two independent fields merge against the locked current row.
    let (left, right) = tokio::join!(
        send_json(
            &app,
            Method::PATCH,
            path,
            json!({"configuration":{"scriptsDefaultTimeoutSeconds":40}}),
            Some(fixture.principal.clone()),
            None
        ),
        send_json(
            &app,
            Method::PATCH,
            path,
            json!({"configuration":{"scriptsMaximumTimeoutSeconds":50}}),
            Some(fixture.principal.clone()),
            None
        )
    );
    assert_eq!(left.status(), StatusCode::OK);
    assert!([StatusCode::OK, StatusCode::BAD_REQUEST].contains(&right.status()));
    let reloaded = fixture
        .instance
        .installation_settings()
        .await
        .unwrap()
        .configuration;
    reloaded.validate().unwrap();
    assert_eq!(reloaded.scripts_default_timeout_seconds, 40);
    // Stale principal claims do not bypass the enabled-human recheck inside the transaction.
    sqlx::query("UPDATE actors SET isenabled=false WHERE id=$1")
        .bind(fixture.principal.actor_id.value())
        .execute(&fixture.pool)
        .await
        .unwrap();
    let response = send_json(
        &app,
        Method::PATCH,
        path,
        json!({"configuration":{"scriptsEnabled":false}}),
        Some(fixture.principal.clone()),
        None,
    )
    .await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    fixture.pool.close().await;
    drop(app);
    drop(fixture);
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "DROP DATABASE {name} WITH (FORCE)"
    )))
    .execute(&admin)
    .await
    .unwrap();
    admin.close().await;
}

#[tokio::test]
#[ignore = "requires disposable CITADEL_IDENTITY_DATABASE_URL"]
async fn deployment_http_agent_rejects_combined_settings_atomically() {
    let base = std::env::var("CITADEL_IDENTITY_DATABASE_URL").unwrap();
    let admin = PgPool::connect(&base).await.unwrap();
    let name = format!("citadel_instance_https_{}", Uuid::now_v7().simple());
    sqlx::query(sqlx::AssertSqlSafe(format!("CREATE DATABASE {name}")))
        .execute(&admin)
        .await
        .unwrap();
    let mut url = url::Url::parse(&base).unwrap();
    url.set_path(&name);
    let fixture = fixture_at(MfaPolicy::Optional, url.as_str()).await;
    sqlx::query("UPDATE instancesetupstates SET initializedat=now(), initialadministratoractorid=$1 WHERE id=1")
        .bind(fixture.principal.actor_id.value()).execute(&fixture.pool).await.unwrap();
    let restricted = instance(
        &fixture.pool,
        citadel_instance::InstallationPolicy {
            deployment_core_agent_requires_http: true,
            ..Default::default()
        },
    );
    let notification = restricted.installation_changes();
    let app = router_with_instance(
        fixture.identity.clone(),
        fixture.pool.clone(),
        MfaPolicy::Optional,
        restricted.clone(),
    );
    let before = restricted.installation_settings().await.unwrap();
    let audits: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM activityevents WHERE eventtype='InstallationSettingsUpdated'",
    )
    .fetch_one(&fixture.pool)
    .await
    .unwrap();
    let response = send_json(&app, Method::PATCH, "/api/v1/installation/settings",
        json!({"configuration":{"requireDirectAgentHttps":true,"passwordMinimumLength":20},"mfaPolicy":"RequiredForAllUsers","updateCheckIntervalHours":24}),
        Some(fixture.principal.clone()), None).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert!(
        json_body(response)
            .await
            .to_string()
            .contains("CITADEL_CORE_AGENT_ADDRESS")
    );
    assert_eq!(restricted.installation_settings().await.unwrap(), before);
    assert!(!notification.has_changed().unwrap());
    let after_audits: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM activityevents WHERE eventtype='InstallationSettingsUpdated'",
    )
    .fetch_one(&fixture.pool)
    .await
    .unwrap();
    assert_eq!(after_audits, audits);
    // The hosting fact is not an HTTP setting and cannot be supplied by a client.
    let forged = send_json(&app, Method::PATCH, "/api/v1/installation/settings",
        json!({"deploymentCoreAgentRequiresHttp":false,"configuration":{"requireDirectAgentHttps":true}}),
        Some(fixture.principal.clone()), None).await;
    assert_eq!(forged.status(), StatusCode::BAD_REQUEST);
    assert!(!notification.has_changed().unwrap());
    // HTTPS/absent deployment connector uses the same false fact; normal HTTP
    // Direct Agents are not inspected or mutated by this save.
    let compatible = instance(&fixture.pool, Default::default());
    let compatible_app = router_with_instance(
        fixture.identity.clone(),
        fixture.pool.clone(),
        MfaPolicy::Optional,
        compatible,
    );
    let response = send_json(
        &compatible_app,
        Method::PATCH,
        "/api/v1/installation/settings",
        json!({"configuration":{"requireDirectAgentHttps":true}}),
        Some(fixture.principal.clone()),
        None,
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let body = json_body(response).await;
    assert_eq!(
        body["effectiveConfiguration"]["requireDirectAgentHttps"],
        true
    );
    assert!(!body.to_string().contains("deploymentCoreAgent"));
    fixture.pool.close().await;
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "DROP DATABASE {name} WITH (FORCE)"
    )))
    .execute(&admin)
    .await
    .unwrap();
    admin.close().await;
}
