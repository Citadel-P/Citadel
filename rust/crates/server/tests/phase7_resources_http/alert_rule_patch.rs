use super::*;

// Ports AlertRulePatchTests: configuration merge, channel replacement, bad
// cooldown rollback, missing rule/channel, and channel merge-patch behavior.
pub(super) async fn verify(
    app: &Router,
    pool: &sqlx::PgPool,
    admin: &ActorPrincipal,
    rule: &Value,
    channel: &Value,
    hub: &RealtimeHub,
    store: &PostgresAlertStore,
) {
    let mut changes = hub.subscribe();
    let id = rule["id"].as_str().unwrap();
    let path = format!("/api/v1/alertRules/{id}");
    let channel_id = channel["id"].as_str().unwrap();
    let channel_path = format!("/api/v1/alertRules/channels/{channel_id}");
    let before =
        response_json(request(app, Method::GET, &path, Some(admin.clone()), None).await).await;
    let reader = seed_regular_user(pool).await;
    for principal in [None, Some(reader)] {
        let expected = if principal.is_none() {
            StatusCode::UNAUTHORIZED
        } else {
            StatusCode::FORBIDDEN
        };
        for target in [&path, &channel_path] {
            assert_eq!(
                request(
                    app,
                    Method::PATCH,
                    target,
                    principal.clone(),
                    Some(json!({"status":"Disabled"}))
                )
                .await
                .status(),
                expected
            );
        }
    }
    let response = request(
        app,
        Method::PATCH,
        &path,
        Some(admin.clone()),
        Some(json!({"severity":"Critical","cooldownSeconds":600,"status":"Disabled"})),
    )
    .await;
    let status = response.status();
    let value = response_json(response).await;
    assert_eq!(status, StatusCode::OK, "{value}");
    assert_eq!(value["name"], before["name"]);
    assert_eq!(value["description"], before["description"]);
    assert_eq!(value["channelIds"], before["channelIds"]);
    let mut expected = before.clone();
    expected["severity"] = json!("Critical");
    expected["cooldownSeconds"] = json!(600);
    expected["status"] = json!("Disabled");
    alert_assertions::saved_rule(pool, admin, &expected, &value).await;
    alert_assertions::runtime_rule(pool, store, &value).await;
    assert_eq!(drain_resource_changes(&mut changes, "Alert"), 1);
    let persisted: (String, i32, String) =
        sqlx::query_as("SELECT severity,cooldownseconds,status FROM alertrules WHERE id=$1")
            .bind(Uuid::parse_str(id).unwrap())
            .fetch_one(pool)
            .await
            .unwrap();
    assert_eq!(persisted, ("Critical".into(), 600, "Disabled".into()));
    for invalid in [
        json!({"cooldownSeconds":5}),
        json!({"severity":"not-a-severity"}),
        json!({"channelIds":[Uuid::now_v7()]}),
        json!([]),
    ] {
        let before_rules = alert_assertions::rule_set(pool).await;
        let response = request(
            app,
            Method::PATCH,
            &path,
            Some(admin.clone()),
            Some(invalid.clone()),
        )
        .await;
        if invalid.get("cooldownSeconds").is_some() {
            alert_assertions::validation_problem(
                response,
                json!({"0":["Cooldown must be between 10s and 24h. (Parameter 'cooldownSeconds')"]}),
            )
            .await;
        } else {
            assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        }
        assert_eq!(alert_assertions::rule_set(pool).await, before_rules);
        let after =
            response_json(request(app, Method::GET, &path, Some(admin.clone()), None).await).await;
        assert_eq!(
            after, value,
            "rejected partial writes must not change configuration or channels"
        );
        assert_eq!(
            drain_resource_changes(&mut changes, "Alert"),
            0,
            "a rejected patch must not publish a change"
        );
    }
    let second = response_json(request(app, Method::POST, "/api/v1/alertRules/channels", Some(admin.clone()), Some(json!({"name":format!("patch-{}",Uuid::now_v7()),"alertDestination":"Generic","url":"https://alerts.example.test/second","isActive":true}))).await).await;
    // Enable via HTTP so the same store must evaluate the just-replaced links,
    // not merely return them from a separate database reader.
    assert_eq!(
        request(
            app,
            Method::PATCH,
            &path,
            Some(admin.clone()),
            Some(json!({"status":"Enabled"}))
        )
        .await
        .status(),
        StatusCode::OK
    );
    for channels in [json!([second["id"]]), json!([]), json!([channel_id])] {
        let response = request(
            app,
            Method::PATCH,
            &path,
            Some(admin.clone()),
            Some(json!({"channelIds":channels})),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let updated = response_json(response).await;
        assert_eq!(updated["channelIds"], channels);
        alert_assertions::runtime_rule(pool, store, &updated).await;
        let saved: Vec<Uuid> =
            sqlx::query_scalar("SELECT alertchannelid FROM alertrulechannels WHERE alertruleid=$1")
                .bind(Uuid::parse_str(id).unwrap())
                .fetch_all(pool)
                .await
                .unwrap();
        assert_eq!(serde_json::to_value(saved).unwrap(), channels);
    }
    // Disjoint concurrent patches must not overwrite values read before the lock.
    let (left, right) = tokio::join!(
        request(
            app,
            Method::PATCH,
            &path,
            Some(admin.clone()),
            Some(json!({"severity":"Info"}))
        ),
        request(
            app,
            Method::PATCH,
            &path,
            Some(admin.clone()),
            Some(json!({"cooldownSeconds":700}))
        )
    );
    assert_eq!(left.status(), StatusCode::OK);
    assert_eq!(right.status(), StatusCode::OK);
    let after =
        response_json(request(app, Method::GET, &path, Some(admin.clone()), None).await).await;
    assert_eq!(after["severity"], "Info");
    assert_eq!(after["cooldownSeconds"], 700);
    let before_rules = alert_assertions::rule_set(pool).await;
    let response = request(
        app,
        Method::PATCH,
        &channel_path,
        Some(admin.clone()),
        Some(json!({"url":"https://alerts.example.test/patched","isActive":false})),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let updated = response_json(response).await;
    assert_eq!(updated["name"], channel["name"]);
    assert_eq!(updated["alertDestination"], channel["alertDestination"]);
    assert_eq!(updated["isActive"], false);
    let saved: (String, bool) =
        sqlx::query_as("SELECT url,isactive FROM alertchannels WHERE id=$1")
            .bind(Uuid::parse_str(channel_id).unwrap())
            .fetch_one(pool)
            .await
            .unwrap();
    assert_eq!(saved, ("https://alerts.example.test/patched".into(), false));
    assert_eq!(alert_assertions::rule_set(pool).await, before_rules);
    let current_rule =
        response_json(request(app, Method::GET, &path, Some(admin.clone()), None).await).await;
    alert_assertions::runtime_rule(pool, store, &current_rule).await;
    for prefix in ["/api/v1/alertRules", "/api/v1/alertRules/channels"] {
        let before_rules = alert_assertions::rule_set(pool).await;
        let patch = if prefix == "/api/v1/alertRules" {
            json!({"severity":"Critical"})
        } else {
            json!({})
        };
        let response = request(
            app,
            Method::PATCH,
            &format!("{prefix}/{}", Uuid::now_v7()),
            Some(admin.clone()),
            Some(patch),
        )
        .await;
        if prefix == "/api/v1/alertRules" {
            alert_assertions::missing_rule(response).await;
        } else {
            alert_assertions::problem(
                response,
                StatusCode::NOT_FOUND,
                "The requested resource was not found.",
            )
            .await;
        }
        assert_eq!(alert_assertions::rule_set(pool).await, before_rules);
    }
    // Keep the enclosing lifecycle's delivery fixture active.
    assert_eq!(
        request(
            app,
            Method::PATCH,
            &channel_path,
            Some(admin.clone()),
            Some(json!({"isActive":true}))
        )
        .await
        .status(),
        StatusCode::OK
    );
    assert_eq!(request(app, Method::PATCH, &path, Some(admin.clone()), Some(json!({"status":before["status"],"severity":before["severity"],"cooldownSeconds":before["cooldownSeconds"]}))).await.status(),StatusCode::OK);
}
