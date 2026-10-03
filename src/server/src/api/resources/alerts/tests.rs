use super::{patch::*, requests::*, views::*};
use citadel_alerts::AlertRuleStatus;
use serde_json::{Value, json};
use uuid::Uuid;

fn rule_input() -> Value {
    json!({
        "name":"Platform unavailable", "type":"PlatformUnreachable", "severity":"Warning",
        "limitedTo":[{"resourceId":Uuid::now_v7(),"resourceType":"Platform"}],
        "quietHours":[{"$type":"Weekly","name":"Sunday night","timezone":"Europe/Paris",
            "dayOfWeek":"Sunday","startTime":"22:00:00","endTime":"02:00:00"}]
    })
}

#[test]
fn typed_rule_round_trips_configuration_and_evaluates_overnight_quiet_hours() {
    let request: AlertRuleInput = serde_json::from_value(rule_input()).unwrap();
    let mut config: citadel_alerts::AlertRuleConfiguration = request.into();
    config.validate_create().unwrap();
    assert!(citadel_alerts::is_in_quiet_hours(
        &config.quiet_hours,
        "2026-07-19T23:00:00Z".parse().unwrap()
    ));
    assert!(!citadel_alerts::is_in_quiet_hours(
        &config.quiet_hours,
        "2026-07-18T23:00:00Z".parse().unwrap()
    ));
    let response = AlertRuleInput::try_from(config.clone()).unwrap();
    assert_eq!(response.status, AlertRuleStatus::Enabled);
    let wire = serde_json::to_value(&response).unwrap();
    assert_eq!(wire["quietHours"][0]["$type"], "Weekly");
    assert!(wire["quietHours"][0].get("scheduleType").is_none());
    let round_trip: citadel_alerts::AlertRuleConfiguration = response.into();
    assert_eq!(round_trip.quiet_hours, config.quiet_hours);
    assert_eq!(round_trip.limited_to, config.limited_to);
}

#[test]
fn typed_rule_rejects_invalid_enums_and_malformed_nested_fields() {
    for (pointer, value) in [
        ("/type", json!("Unknown")),
        ("/severity", json!("Unknown")),
        ("/status", json!("Unknown")),
        ("/limitedTo/0/resourceId", json!("bad-id")),
        ("/limitedTo/0/resourceType", json!("Unknown")),
        ("/quietHours/0/$type", json!("Monthly")),
        ("/quietHours/0/dayOfWeek", json!("Someday")),
    ] {
        let mut input = rule_input();
        input["status"] = json!("Enabled");
        *input.pointer_mut(pointer).unwrap() = value;
        assert!(
            serde_json::from_value::<AlertRuleInput>(input).is_err(),
            "{pointer}"
        );
    }
    let mut input = rule_input();
    input["quietHours"][0]
        .as_object_mut()
        .unwrap()
        .remove("dayOfWeek");
    assert!(serde_json::from_value::<AlertRuleInput>(input).is_err());
}

#[test]
fn quiet_hour_timezone_and_overlap_validation_remain_in_the_domain() {
    for input in [
        {
            let mut input = rule_input();
            input["quietHours"][0]["timezone"] = json!("Invalid/Zone");
            input
        },
        {
            let mut input = rule_input();
            let hour = input["quietHours"][0].clone();
            input["quietHours"].as_array_mut().unwrap().push(hour);
            input
        },
    ] {
        let request: AlertRuleInput = serde_json::from_value(input).unwrap();
        let mut config: citadel_alerts::AlertRuleConfiguration = request.into();
        assert!(config.validate().is_err());
    }
}

#[test]
fn patches_preserve_omission_and_explicit_null_and_ignore_identity_fields() {
    let patch: PatchAlertRuleInput = serde_json::from_value(json!({
        "name":"ignored", "threshold":null, "quietHours":null, "status":"Disabled"
    }))
    .unwrap();
    assert_eq!(
        patch.into_value(),
        json!({"threshold":null,"quietHours":null,"status":"Disabled"})
    );
    let patch: PatchAlertRuleMetadata =
        serde_json::from_value(json!({"description":null})).unwrap();
    assert_eq!(patch.into_value(), json!({"description":null}));
    let patch: PatchAlertRuleMetadata = serde_json::from_value(json!({})).unwrap();
    assert_eq!(patch.into_value(), json!({}));
    for invalid in [json!([]), json!(null), json!("invalid")] {
        assert!(serde_json::from_value::<PatchAlertRuleInput>(invalid.clone()).is_err());
        assert!(serde_json::from_value::<PatchAlertChannelInput>(invalid.clone()).is_err());
        assert!(serde_json::from_value::<PatchAlertRuleMetadata>(invalid).is_err());
    }
    assert!(
        serde_json::from_value::<PatchAlertRuleInput>(json!({"quietHours":[{"$type":"Weekly"}]}))
            .is_err()
    );
}

#[test]
fn channel_patch_accepts_partial_update_and_destination_uses_wire_spelling() {
    let patch: PatchAlertChannelInput = serde_json::from_value(json!({"isActive":false})).unwrap();
    assert_eq!(patch.into_value(), json!({"isActive":false}));
    for destination in ["Google_Chat", "Zulip_Chat", "IFTTT", "Generic"] {
        let input: AlertChannelInput=serde_json::from_value(json!({"alertDestination":destination,"url":"https://example.com/hook","isActive":true})).unwrap();
        let mut config: citadel_alerts::AlertChannelConfiguration = input.into();
        config.validate().unwrap();
        assert_eq!(config.alert_destination, destination);
        let view = AlertChannelInput::try_from(config).unwrap();
        assert_eq!(
            serde_json::to_value(view).unwrap()["alertDestination"],
            destination
        );
    }
}

#[test]
fn event_view_preserves_runtime_details_and_native_actor_type() {
    let now = chrono::Utc::now();
    let info =
        json!({"HumanMessage":"Platform disconnected","Error":{"message":"transport closed"}});
    let event = citadel_alerts::AlertEvent {
        id: Uuid::now_v7(),
        alert_rule_id: Uuid::now_v7(),
        alert_type: "PlatformUnreachable".into(),
        severity: citadel_alerts::AlertSeverity::Warning,
        status: citadel_alerts::AlertEventStatus::Resolved,
        message: "Platform disconnected".into(),
        info: info.clone(),
        resource_id: Some(Uuid::now_v7()),
        resource_name: "edge".into(),
        resource_type: "Platform".into(),
        acknowledged_by_actor_id: None,
        acknowledged_at: None,
        resolved_by_actor_id: Some(Uuid::from_u128(1)),
        resolved_at: Some(now),
        actor_id: Some(Uuid::from_u128(1)),
        actor_name: Some("System".into()),
        actor_type: Some("System".into()),
        resolution_note: None,
        created_at: now,
        updated_at: now,
    };
    let view = AlertEventView::try_from(event).unwrap();
    let wire = serde_json::to_value(view).unwrap();
    assert_eq!(wire["info"], info);
    assert_eq!(wire["actorType"], "System");
    assert_eq!(wire["status"], "Resolved");
    assert!(
        wire["resourcePath"]
            .as_str()
            .unwrap()
            .starts_with("/platforms/edit/")
    );
}

#[test]
fn alert_openapi_is_derived_from_native_dtos() {
    let doc = serde_json::to_value(crate::openapi::document(false)).unwrap();
    let schemas = &doc["components"]["schemas"];
    for owner in ["AlertRuleInput", "AlertRuleView", "PatchAlertRuleInput"] {
        let field = &schemas[owner]["properties"]["quietHours"];
        // Nullable PATCH arrays still reference the same native item schema.
        assert!(
            field
                .to_string()
                .contains("#/components/schemas/AlertQuietHour"),
            "{owner}: {field}"
        );
    }
    assert!(schemas["AlertQuietHour"]["oneOf"].is_array());
    assert_eq!(
        schemas["AlertRuleInput"]["properties"]["severity"]["$ref"],
        "#/components/schemas/AlertSeverity"
    );
    let paths = &doc["paths"];
    assert_eq!(
        paths["/api/v1/alertRules/{id}/_cfg"]["get"]["responses"]["200"]["content"]["application/json"]
            ["schema"]["$ref"],
        "#/components/schemas/AlertRuleConfig"
    );
    for media in ["application/json", "application/merge-patch+json"] {
        assert_eq!(
            paths["/api/v1/alertRules/channels/{id}"]["patch"]["requestBody"]["content"][media]["schema"]
                ["$ref"],
            "#/components/schemas/PatchAlertChannelInput"
        );
    }
}

#[test]
fn channel_creation_attribution_remains_flat_on_the_wire() {
    let actor = Uuid::now_v7();
    let created_at = chrono::Utc::now();
    let view = AlertChannelView::try_from(citadel_alerts::AlertChannel {
        id: Uuid::now_v7(),
        name: "ops".into(),
        alert_destination: "Generic".into(),
        url: "https://alerts.example.test/hook".into(),
        is_active: true,
        audit: citadel_primitives::AuditMetadata {
            created_by_actor_id: citadel_primitives::ActorId::new(actor),
            created_at,
        },
    })
    .unwrap();
    let wire = serde_json::to_value(view).unwrap();
    assert_eq!(wire["createdByActorId"], json!(actor));
    assert_eq!(wire["createdAt"], json!(created_at));
    assert!(wire.get("audit").is_none());
}
