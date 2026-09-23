use super::*;
use citadel_alerts::{AlertObservation, AlertRepository};

// Explicit values from AlertRuleCreateTests / AlertRulePatchTests snapshots.
// Rust includes null/default fields and creation metadata in its shared view;
// assert those too instead of comparing two potentially incorrect API reads.
pub(super) async fn saved_rule(
    pool: &sqlx::PgPool,
    admin: &ActorPrincipal,
    input: &Value,
    actual: &Value,
) {
    let id = Uuid::parse_str(actual["id"].as_str().unwrap()).unwrap();
    assert!(!id.is_nil());
    let mut expected = json!({
        "id": id,
        "name": input["name"].as_str().unwrap_or(input["type"].as_str().unwrap()),
        "description": input["description"], "type": input["type"],
        "severity": input["severity"], "cooldownSeconds": input["cooldownSeconds"],
        "requiredMatches": input["requiredMatches"], "threshold": input["threshold"],
        "status": input["status"].as_str().unwrap_or("Enabled"),
        "channelIds": input.get("channelIds").cloned().unwrap_or(json!([])),
        "limitedTo": input.get("limitedTo").cloned().unwrap_or(json!([])),
        "quietHours": input.get("quietHours").cloned().unwrap_or(json!([])),
        "createdByActorId": admin.actor_id.value()
    });
    let created =
        chrono::DateTime::parse_from_rfc3339(actual["createdAt"].as_str().unwrap()).unwrap();
    assert_eq!(created.offset().local_minus_utc(), 0);
    expected["createdAt"] = actual["createdAt"].clone();
    assert_eq!(actual, &expected);
    let stored: Value = sqlx::query_scalar("SELECT to_jsonb(r) FROM alertrules r WHERE id=$1")
        .bind(id)
        .fetch_one(pool)
        .await
        .unwrap();
    for (public, column) in [
        ("id", "id"),
        ("name", "name"),
        ("description", "description"),
        ("type", "type"),
        ("severity", "severity"),
        ("cooldownSeconds", "cooldownseconds"),
        ("requiredMatches", "requiredmatches"),
        ("threshold", "threshold"),
        ("status", "status"),
        ("limitedTo", "limitedto"),
        ("quietHours", "quiethours"),
        ("createdByActorId", "createdbyactorid"),
    ] {
        if public == "threshold" && expected[public].is_number() {
            // PostgreSQL jsonb emits 85 for a stored float8 value of 85.0.
            // Compare the numeric value, not serde_json's integer/float tags.
            assert_eq!(
                stored[column].as_f64().unwrap(),
                expected[public].as_f64().unwrap()
            );
        } else {
            assert_eq!(stored[column], expected[public], "persisted {column}");
        }
    }
    assert_eq!(
        chrono::DateTime::parse_from_rfc3339(stored["createdat"].as_str().unwrap()).unwrap(),
        created
    );
    let links: Vec<Uuid> = sqlx::query_scalar(
        "SELECT alertchannelid FROM alertrulechannels WHERE alertruleid=$1 ORDER BY alertchannelid",
    )
    .bind(id)
    .fetch_all(pool)
    .await
    .unwrap();
    let mut expected_links: Vec<Uuid> =
        serde_json::from_value(expected["channelIds"].clone()).unwrap();
    expected_links.sort();
    assert_eq!(links, expected_links);
}

// Snapshot the entire persisted Rule set and links, not just its count. Channel
// mutations must not accidentally modify Rules or their runtime configuration.
pub(super) async fn rule_set(pool: &sqlx::PgPool) -> Value {
    sqlx::query_scalar("SELECT jsonb_build_object('rules', (SELECT COALESCE(jsonb_agg(to_jsonb(r) ORDER BY r.id),'[]') FROM alertrules r), 'links', (SELECT COALESCE(jsonb_agg(to_jsonb(c) ORDER BY c.alertruleid,c.alertchannelid),'[]') FROM alertrulechannels c))")
        .fetch_one(pool).await.unwrap()
}

// Verify committed changes through the same long-lived
// store held by the HTTP router and exercise its next evaluation after commit.
pub(super) async fn runtime_rule(
    pool: &sqlx::PgPool,
    store: &PostgresAlertRepository,
    expected: &Value,
) {
    let id = Uuid::parse_str(expected["id"].as_str().unwrap()).unwrap();
    let current = serde_json::to_value(
        citadel_server::api::resources::alerts::views::AlertRuleView::from(
            store.get_rule(id).await.unwrap(),
        ),
    )
    .unwrap();
    assert_eq!(&current, expected);
    // This combined test owns a disposable database. Temporarily isolate this
    // Rule from built-in/other fixture Rules; never alter the Rule under test.
    let other_ids: Vec<Uuid> = sqlx::query_scalar(
        "UPDATE alertrules SET status='Disabled' WHERE id<>$1 AND status='Enabled' RETURNING id",
    )
    .bind(id)
    .fetch_all(pool)
    .await
    .unwrap();
    let resource = Uuid::now_v7();
    let observation = AlertObservation {
        alert_type: expected["type"].as_str().unwrap().into(),
        info: json!({"humanMessage":"HTTP configuration visibility probe"}),
        resource_id: resource,
        resource_name: "parity probe".into(),
        resource_type: "Platform".into(),
        deduplication_component: "http-parity".into(),
        observed_at: chrono::Utc::now(),
        value: expected["threshold"].as_f64().map(|value| value + 1.0),
        matched: true,
    };
    let required = expected["requiredMatches"].as_i64().unwrap_or(1);
    let enabled = expected["status"] == "Enabled";
    let mut event = None;
    for index in 1..=required {
        event = store.process_event(&observation).await.unwrap();
        assert_eq!(event.is_some(), enabled && index == required);
    }
    if let Some(event) = event {
        assert_eq!(event.alert_rule_id, id);
        assert_eq!(event.alert_type, expected["type"].as_str().unwrap());
        assert_eq!(event.severity, expected["severity"].as_str().unwrap());
        let delivered: Vec<Uuid> = sqlx::query_scalar("SELECT alertchannelid FROM alertdeliveryoutbox WHERE alerteventid=$1 ORDER BY alertchannelid")
            .bind(event.id).fetch_all(pool).await.unwrap();
        let ids: Vec<Uuid> = serde_json::from_value(expected["channelIds"].clone()).unwrap();
        let active: Vec<Uuid> = sqlx::query_scalar(
            "SELECT id FROM alertchannels WHERE id=ANY($1) AND isactive ORDER BY id",
        )
        .bind(&ids)
        .fetch_all(pool)
        .await
        .unwrap();
        assert_eq!(
            delivered, active,
            "next evaluation must use current Channel links"
        );
        // A recovered condition resolves the incident, then a matched condition
        // inside the newly saved cooldown must not produce a replacement event.
        let mut recovered = observation.clone();
        recovered.matched = false;
        recovered.value = expected["threshold"].as_f64().map(|value| value - 1.0);
        recovered.observed_at += Duration::seconds(1);
        assert!(store.process_event(&recovered).await.unwrap().is_none());
        assert_eq!(store.get_event(event.id).await.unwrap().status, "Resolved");
        let mut retrigger = observation.clone();
        retrigger.observed_at += Duration::seconds(2);
        if expected["cooldownSeconds"]
            .as_i64()
            .is_some_and(|value| value > 2)
        {
            for _ in 0..required {
                assert!(store.process_event(&retrigger).await.unwrap().is_none());
            }
        }
    }
    sqlx::query("DELETE FROM alertdeliveryoutbox WHERE alerteventid IN (SELECT id FROM alertevents WHERE resourceid=$1)").bind(resource).execute(pool).await.unwrap();
    sqlx::query("DELETE FROM alertevents WHERE resourceid=$1")
        .bind(resource)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM alertrulestates WHERE resourceid=$1")
        .bind(resource)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("UPDATE alertrules SET status='Enabled' WHERE id=ANY($1)")
        .bind(other_ids)
        .execute(pool)
        .await
        .unwrap();
}

pub(super) async fn validation_problem(response: axum::response::Response, errors: Value) {
    compatibility_problem(
        response,
        StatusCode::BAD_REQUEST,
        json!({
            "type":"https://tools.ietf.org/html/rfc9110#section-15.5.1",
            "title":"One or more validation errors occurred.", "status":400, "errors":errors
        }),
    )
    .await;
}

pub(super) async fn missing_rule(response: axum::response::Response) {
    compatibility_problem(
        response,
        StatusCode::NOT_FOUND,
        json!({
            "type":"https://tools.ietf.org/html/rfc9110#section-15.5.5",
            "title":"Not Found", "status":404, "detail":"The provided alert rule does not exist"
        }),
    )
    .await;
}

async fn compatibility_problem(
    response: axum::response::Response,
    status: StatusCode,
    mut expected: Value,
) {
    assert_eq!(response.status(), status);
    assert_eq!(
        response.headers()["content-type"],
        "application/problem+json"
    );
    assert_eq!(response.headers()["cache-control"], "no-store");
    let bytes = to_bytes(response.into_body(), 16 * 1024).await.unwrap();
    let actual: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(actual["traceId"], "alert-parity-request");
    expected["traceId"] = json!("alert-parity-request");
    assert_eq!(actual, expected);
}

// Validation explanations must be visible to the frontend, including cases
// without a named field snapshot. Not-found keeps its existing envelope.
pub(super) async fn problem(response: axum::response::Response, status: StatusCode, detail: &str) {
    assert_eq!(response.status(), status);
    assert_eq!(
        response.headers()["content-type"],
        "application/problem+json"
    );
    assert_eq!(response.headers()["cache-control"], "no-store");
    let bytes = to_bytes(response.into_body(), 16 * 1024).await.unwrap();
    let body: Value = serde_json::from_slice(&bytes).unwrap();
    if status == StatusCode::BAD_REQUEST {
        assert_eq!(
            body,
            json!({
                "type":"https://tools.ietf.org/html/rfc9110#section-15.5.1",
                "title":"One or more validation errors occurred.",
                "status":400,"errors":{"$":[detail]},"traceId":"alert-parity-request"
            })
        );
        return;
    }
    assert!(!body["requestId"].as_str().unwrap().is_empty());
    let (kind, title) = if status == StatusCode::NOT_FOUND {
        ("not_found", "Not found")
    } else {
        ("validation_error", "Validation failed")
    };
    assert_eq!(
        body,
        json!({"type":kind,"title":title,"status":status.as_u16(),"detail":detail,"requestId":body["requestId"]})
    );
}
