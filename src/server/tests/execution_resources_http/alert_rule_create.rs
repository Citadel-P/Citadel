use super::*;
use std::sync::atomic::{AtomicBool, Ordering};

pub(super) struct Entitlement(AtomicBool);
impl Default for Entitlement {
    fn default() -> Self {
        Self(AtomicBool::new(true))
    }
}
impl citadel_alerts::AlertEntitlements for Entitlement {
    fn advanced_alerting(&self) -> BoxFuture<'_, Result<bool, AlertError>> {
        Box::pin(async { Ok(self.0.load(Ordering::SeqCst)) })
    }
}

pub(super) async fn verify(
    app: &Router,
    pool: &sqlx::PgPool,
    admin: &ActorPrincipal,
    entitlement: &Entitlement,
    channel: &Value,
    store: &PostgresAlertRepository,
) {
    let endpoint = "/api/v1/alertRules";
    let count = || sqlx::query_scalar::<_, i64>("SELECT count(*) FROM alertrules").fetch_one(pool);
    let channel_id = channel["id"].clone();
    // AlertRuleCreateTests.Create_NonThreshold_AlertRule_ReturnsSuccess,
    // Create_Threshold_AlertRule_ReturnsSuccess, Create_AlertRule_With_Channels_ReturnsSuccess.
    for input in [
        json!({"description":"Test description","type":"PlatformUnreachable","severity":"Critical","cooldownSeconds":300,"status":"Enabled"}),
        json!({"type":"PlatformCpuHigh","severity":"Warning","cooldownSeconds":60,"status":"Enabled","requiredMatches":3,"threshold":85.0}),
        json!({"name":null,"type":"PlatformUnreachable","severity":"Critical","cooldownSeconds":120,"status":"Enabled","channelIds":[channel_id]}),
        json!({"type":"PlatformUnreachable","severity":"Warning","cooldownSeconds":60}),
    ] {
        let response = request(
            app,
            Method::POST,
            endpoint,
            Some(admin.clone()),
            Some(input.clone()),
        )
        .await;
        let status = response.status();
        let saved = response_json(response).await;
        assert_eq!(status, StatusCode::OK, "{saved}");
        alert_assertions::saved_rule(pool, admin, &input, &saved).await;
        alert_assertions::runtime_rule(pool, store, &saved).await;
        let id = Uuid::parse_str(saved["id"].as_str().unwrap()).unwrap();
        let created:Value = sqlx::query_scalar("SELECT info::jsonb FROM activityevents WHERE resourceid=$1 AND eventtype='AlertRuleCreated'").bind(id).fetch_one(pool).await.unwrap();
        assert_eq!(created["AlertRule"]["Id"], saved["id"]);
        assert_eq!(created["AlertRule"]["Name"], saved["name"]);
        let path = format!("{endpoint}/{id}");
        let mut saved = saved;
        saved["capabilities"] = json!({"canRead":true,"canWrite":true,"canExecute":true});
        assert_eq!(
            response_json(request(app, Method::GET, &path, Some(admin.clone()), None).await).await,
            saved
        );
        // Create always needs AdvancedAlerting. Disable/removal remains possible
        // after downgrade; changes and re-enabling custom rules are gated.
        entitlement.0.store(false, Ordering::SeqCst);
        let before_count = count().await.unwrap();
        assert_eq!(
            request(
                app,
                Method::POST,
                endpoint,
                Some(admin.clone()),
                Some(input)
            )
            .await
            .status(),
            StatusCode::FORBIDDEN
        );
        assert_eq!(count().await.unwrap(), before_count);
        for patch in [json!({"severity":"Info"}), json!({"cooldownSeconds":600})] {
            assert_eq!(
                request(app, Method::PATCH, &path, Some(admin.clone()), Some(patch))
                    .await
                    .status(),
                StatusCode::FORBIDDEN
            );
            assert_eq!(
                response_json(request(app, Method::GET, &path, Some(admin.clone()), None).await)
                    .await,
                saved
            );
        }
        assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM activityevents WHERE resourceid=$1 AND eventtype='AlertRuleUpdated'").bind(id).fetch_one(pool).await.unwrap(),0,"rejected changes must not write success activity");
        assert_eq!(
            request(
                app,
                Method::PATCH,
                &path,
                Some(admin.clone()),
                Some(json!({"status":"Disabled","channelIds":[]}))
            )
            .await
            .status(),
            StatusCode::OK
        );
        let updated:Value = sqlx::query_scalar("SELECT info::jsonb FROM activityevents WHERE resourceid=$1 AND eventtype='AlertRuleUpdated'").bind(id).fetch_one(pool).await.unwrap();
        assert_eq!(updated["OldRule"]["Status"], "Enabled");
        assert_eq!(updated["NewRule"]["Status"], "Disabled");
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
            StatusCode::FORBIDDEN
        );
        assert_eq!(
            request(
                app,
                Method::PATCH,
                &path,
                Some(admin.clone()),
                Some(json!({"channelIds":[channel_id]}))
            )
            .await
            .status(),
            StatusCode::FORBIDDEN
        );
        entitlement.0.store(true, Ordering::SeqCst);
        assert_eq!(
            request(
                app,
                Method::DELETE,
                endpoint,
                Some(admin.clone()),
                Some(json!({"ids":[id]}))
            )
            .await
            .status(),
            StatusCode::NO_CONTENT
        );
    }
    // Windows IDs round-trip unchanged, but overlap checks share IANA rules.
    let windows_hour = json!({"$type":"Daily","name":"Paris morning","startTime":"10:00:00","endTime":"11:00:00","timezone":"Romance Standard Time"});
    let input = json!({"name":"Windows quiet hours","type":"PlatformUnreachable","severity":"Warning","quietHours":[windows_hour]});
    let response = request(
        app,
        Method::POST,
        endpoint,
        Some(admin.clone()),
        Some(input.clone()),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let saved = response_json(response).await;
    // Native DTOs serialize the optional description explicitly; timezone IDs stay unchanged.
    let mut expected = input.clone();
    expected["quietHours"][0]["description"] = Value::Null;
    alert_assertions::saved_rule(pool, admin, &expected, &saved).await;
    assert_eq!(saved["quietHours"], expected["quietHours"]);
    let mut saved = saved;
    saved["capabilities"] = json!({"canRead":true,"canWrite":true,"canExecute":true});
    let id = saved["id"].as_str().unwrap();
    let mut iana_hour = windows_hour.clone();
    iana_hour["timezone"] = json!("Europe/Paris");
    let before_rules = alert_assertions::rule_set(pool).await;
    let rejected = request(
        app,
        Method::PATCH,
        &format!("{endpoint}/{id}"),
        Some(admin.clone()),
        Some(json!({"quietHours":[windows_hour,iana_hour]})),
    )
    .await;
    alert_assertions::problem(rejected, StatusCode::BAD_REQUEST, "Quiet hours overlap.").await;
    assert_eq!(alert_assertions::rule_set(pool).await, before_rules);
    assert_eq!(
        response_json(
            request(
                app,
                Method::GET,
                &format!("{endpoint}/{id}"),
                Some(admin.clone()),
                None
            )
            .await
        )
        .await,
        saved
    );
    assert_eq!(
        request(
            app,
            Method::DELETE,
            endpoint,
            Some(admin.clone()),
            Some(json!({"ids":[id]}))
        )
        .await
        .status(),
        StatusCode::NO_CONTENT
    );

    // AlertRuleCreateTests: invalid cooldown, missing threshold requirements,
    // non-threshold rule with threshold settings. No rejected input is persisted.
    for (input, expected) in [
        (
            json!({"type":"PlatformUnreachable","severity":"Warning","cooldownSeconds":5,"status":"Enabled"}),
            json!({"CooldownSeconds":["Cooldown must be between 10s and 24h."]}),
        ),
        (
            json!({"type":"PlatformCpuHigh","severity":"Warning","cooldownSeconds":60,"status":"Enabled","scope":"All","threshold":85.0}),
            json!({"RequiredMatches":["'Required Matches' must not be empty."]}),
        ),
        (
            json!({"type":"PlatformUnreachable","severity":"Warning","cooldownSeconds":60,"status":"Enabled","requiredMatches":3,"threshold":85.0}),
            json!({"RequiredMatches":["Non-threshold alerts must not define RequiredMatches."],"Threshold":["Non-threshold alerts must not define Threshold."]}),
        ),
        (
            json!({"type":"PlatformCpuHigh","severity":"Warning","cooldownSeconds":5}),
            json!({"CooldownSeconds":["Cooldown must be between 10s and 24h."],"RequiredMatches":["'Required Matches' must not be empty."],"Threshold":["'Threshold' must not be empty."]}),
        ),
        (
            json!({"type":"PlatformUnreachable","severity":"Warning","quietHours":[{"$type":"Weekly","dayOfWeek":"Sunday","startTime":"22:00:00","endTime":"02:00:00","timezone":"Europe/Paris"},{"$type":"Weekly","dayOfWeek":"Monday","startTime":"01:00:00","endTime":"03:00:00","timezone":"Europe/Paris"}]}),
            json!("Quiet hours overlap."),
        ),
    ] {
        let before = alert_assertions::rule_set(pool).await;
        let response = request(
            app,
            Method::POST,
            endpoint,
            Some(admin.clone()),
            Some(input),
        )
        .await;
        if expected.is_object() {
            alert_assertions::validation_problem(response, expected).await;
        } else {
            alert_assertions::problem(
                response,
                StatusCode::BAD_REQUEST,
                expected.as_str().unwrap(),
            )
            .await;
        }
        assert_eq!(alert_assertions::rule_set(pool).await, before);
    }
    // Invalid enum values are now rejected during typed JSON extraction, before persistence.
    let before = alert_assertions::rule_set(pool).await;
    let response = request(app, Method::POST, endpoint, Some(admin.clone()), Some(json!({
        "type":"PlatformUnreachable", "severity":"Warning",
        "quietHours":[{"$type":"Weekly","dayOfWeek":"Someday","startTime":"22:00:00","endTime":"02:00:00","timezone":"Europe/Paris"}]
    }))).await;
    let problem = validation::problem_json(response).await;
    assert!(
        problem["errors"].to_string().contains("Someday"),
        "{problem}"
    );
    assert!(
        problem["errors"].to_string().contains("quietHours"),
        "{problem}"
    );
    assert_eq!(alert_assertions::rule_set(pool).await, before);
    // AlertRuleCreateTests.Create_AlertChannel_ReturnsSuccess and empty URL.
    let before_rules = alert_assertions::rule_set(pool).await;
    let response = request(app,Method::POST,"/api/v1/alertRules/channels",Some(admin.clone()),Some(json!({"alertDestination":"Slack","url":"https://hooks.slack.com/services/test","isActive":true}))).await;
    let status = response.status();
    let saved = response_json(response).await;
    assert_eq!(status, StatusCode::OK, "{saved}");
    assert_eq!(saved["name"], "Slack");
    assert_eq!(saved["alertDestination"], "Slack");
    assert_eq!(saved["url"], "https://hooks.slack.com/services/test");
    assert_eq!(saved["isActive"], true);
    assert_eq!(saved["createdByActorId"], json!(admin.actor_id.value()));
    chrono::DateTime::parse_from_rfc3339(saved["createdAt"].as_str().unwrap()).unwrap();
    let stored: (String, String, bool) =
        sqlx::query_as("SELECT alertdestination,url,isactive FROM alertchannels WHERE id=$1")
            .bind(Uuid::parse_str(saved["id"].as_str().unwrap()).unwrap())
            .fetch_one(pool)
            .await
            .unwrap();
    assert_eq!(
        stored,
        (
            "Slack".into(),
            "https://hooks.slack.com/services/test".into(),
            true
        )
    );
    assert_eq!(alert_assertions::rule_set(pool).await, before_rules);
    let before_channels: i64 = sqlx::query_scalar("SELECT count(*) FROM alertchannels")
        .fetch_one(pool)
        .await
        .unwrap();
    alert_assertions::problem(
        request(
            app,
            Method::POST,
            "/api/v1/alertRules/channels",
            Some(admin.clone()),
            Some(json!({"alertDestination":"Slack","url":"","isActive":true})),
        )
        .await,
        StatusCode::BAD_REQUEST,
        "Alert Channel URL must be a non-empty Shoutrrr URL no longer than 4096 characters.",
    )
    .await;
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM alertchannels")
            .fetch_one(pool)
            .await
            .unwrap(),
        before_channels
    );
    assert_eq!(alert_assertions::rule_set(pool).await, before_rules);
    assert_eq!(
        request(
            app,
            Method::DELETE,
            "/api/v1/alertRules/channels",
            Some(admin.clone()),
            Some(json!({"ids":[saved["id"]]}))
        )
        .await
        .status(),
        StatusCode::NO_CONTENT
    );
}
