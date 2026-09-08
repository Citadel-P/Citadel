use super::*;

pub(super) async fn verify(
    app: &Router,
    pool: &sqlx::PgPool,
    admin: &ActorPrincipal,
    rule: &Value,
    store: &PostgresAlertStore,
) {
    let id = rule["id"].as_str().unwrap();
    let cfg = format!("/api/v1/alertRules/{id}/_cfg");
    let metadata = format!("/api/v1/alertRules/{id}/_metadata");
    let reader = seed_regular_user(pool).await;
    for (method, path, body) in [
        (Method::GET, cfg.as_str(), None),
        (
            Method::PATCH,
            metadata.as_str(),
            Some(json!({"description":"ops"})),
        ),
        (
            Method::POST,
            "/api/v1/alertRules/rename",
            Some(json!({"id":id,"name":"ops"})),
        ),
    ] {
        assert_eq!(
            request(app, method.clone(), path, None, body.clone())
                .await
                .status(),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            request(app, method, path, Some(reader.clone()), body)
                .await
                .status(),
            StatusCode::FORBIDDEN
        );
    }
    let config =
        response_json(request(app, Method::GET, &cfg, Some(admin.clone()), None).await).await;
    assert_eq!(config["channelIds"], rule["channelIds"]);
    assert_eq!(config["isSystem"], false);
    assert!(config.get("createdByActorId").is_none());

    let renamed = request(
        app,
        Method::POST,
        "/api/v1/alertRules/rename",
        Some(admin.clone()),
        Some(json!({"id":id,"name":"Renamed rule"})),
    )
    .await;
    assert_eq!(renamed.status(), StatusCode::OK);
    let renamed = response_json(renamed).await;
    let mut expected_rule = rule.clone();
    expected_rule["name"] = json!("Renamed rule");
    alert_assertions::saved_rule(pool, admin, &expected_rule, &renamed).await;
    alert_assertions::runtime_rule(pool, store, &renamed).await;
    let activity: String = sqlx::query_scalar(
        "SELECT info FROM activityevents WHERE resourceid=$1 AND eventtype='AlertRuleRenamed'",
    )
    .bind(Uuid::parse_str(id).unwrap())
    .fetch_one(pool)
    .await
    .unwrap();
    let activity: Value = serde_json::from_str(&activity).unwrap();
    assert_eq!(activity["OldName"], rule["name"]);
    assert_eq!(activity["NewName"], "Renamed rule");

    for (patch, expected) in [
        (
            json!({"description":"ops","name":"ignored","channelIds":[]}),
            json!("ops"),
        ),
        (json!({}), json!("ops")),
        (json!({"description":null}), Value::Null),
    ] {
        let response = request(
            app,
            Method::PATCH,
            &metadata,
            Some(admin.clone()),
            Some(patch),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let value = response_json(response).await;
        assert_eq!(value["description"], expected);
        assert_eq!(value["name"], "Renamed rule");
        assert_eq!(value["channelIds"], rule["channelIds"]);
        assert_eq!(value["threshold"], rule["threshold"]);
        expected_rule["description"] = expected;
        alert_assertions::saved_rule(pool, admin, &expected_rule, &value).await;
        alert_assertions::runtime_rule(pool, store, &value).await;
    }
    assert_eq!(
        request(
            app,
            Method::PATCH,
            &metadata,
            Some(admin.clone()),
            Some(json!({"description":false}))
        )
        .await
        .status(),
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        request(
            app,
            Method::POST,
            "/api/v1/alertRules/rename",
            Some(admin.clone()),
            Some(json!({"id":id,"name":" "}))
        )
        .await
        .status(),
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        request(
            app,
            Method::GET,
            &format!("/api/v1/alertRules/{}/_cfg", Uuid::now_v7()),
            Some(admin.clone()),
            None
        )
        .await
        .status(),
        StatusCode::NOT_FOUND
    );
    let saved: (String, Option<String>) =
        sqlx::query_as("SELECT name, description FROM alertrules WHERE id=$1")
            .bind(Uuid::parse_str(id).unwrap())
            .fetch_one(pool)
            .await
            .unwrap();
    assert_eq!(saved, ("Renamed rule".into(), None));
    let before_rules = alert_assertions::rule_set(pool).await;
    alert_assertions::missing_rule(
        request(
            app,
            Method::POST,
            "/api/v1/alertRules/rename",
            Some(admin.clone()),
            Some(json!({"id":Uuid::now_v7(),"name":"DoesNotExist"})),
        )
        .await,
    )
    .await;
    assert_eq!(alert_assertions::rule_set(pool).await, before_rules);
    // Leave a non-null original description for the configuration PATCH port.
    assert_eq!(
        request(
            app,
            Method::PATCH,
            &metadata,
            Some(admin.clone()),
            Some(json!({"description":rule["description"]}))
        )
        .await
        .status(),
        StatusCode::OK
    );
}
