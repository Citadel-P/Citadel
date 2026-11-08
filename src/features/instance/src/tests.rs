use crate::*;

#[test]
fn defaults_validate_and_omitted_fields_use_defaults() {
    let defaults = InstanceConfiguration::default();
    defaults.validate().unwrap();
    assert_eq!(
        serde_json::from_str::<InstanceConfiguration>("{}").unwrap(),
        defaults
    );
    assert_eq!(defaults.password_minimum_length, 15);
    assert_eq!(defaults.browser_session_lifetime_days, 30);
    assert_eq!(defaults.statistics_collection_interval_seconds, 10);
    assert_eq!(defaults.statistics_write_interval_seconds, 60);
    assert_eq!(defaults.scripts_default_timeout_seconds, 300);
    assert_eq!(defaults.build_retention_days, 90);
    assert_eq!(defaults.backup_default_timeout_seconds, 120);
    assert!(!defaults.require_direct_agent_https);
    assert!(defaults.agent_image_override.is_empty());
}

#[test]
fn partial_patches_preserve_other_choices_and_validate_effective_pairs() {
    let saved = InstanceConfiguration::default();
    let overrides = InstanceOverrides::default();
    let patch: PatchInstanceConfiguration = serde_json::from_str(r#"{"scriptsDefaultTimeoutSeconds":30,"scriptsMaximumTimeoutSeconds":60,"buildCleanupEnabled":false}"#).unwrap();
    assert_eq!(
        serde_json::to_value(&patch)
            .unwrap()
            .as_object()
            .unwrap()
            .len(),
        3
    );
    let next = patch.apply(&saved, &overrides).unwrap();
    assert_eq!(next.scripts_maximum_timeout_seconds, 60);
    assert_eq!(next.scripts_default_timeout_seconds, 30);
    assert_eq!(next.build_retention_days, 90);
    assert!(!next.build_cleanup_enabled);
    assert_eq!(saved.scripts_default_timeout_seconds, 300);
    let patch: PatchInstanceConfiguration =
        serde_json::from_str(r#"{"scriptsMaximumTimeoutSeconds":60}"#).unwrap();
    assert!(patch.apply(&saved, &overrides).is_err());
    let overrides = InstanceOverrides {
        scripts_default_timeout_seconds: Some(30),
        ..Default::default()
    };
    assert!(patch.apply(&saved, &overrides).is_ok());
    let patch: PatchInstanceConfiguration =
        serde_json::from_str(r#"{"serviceAccountMaximumTokenLifetimeDays":30}"#).unwrap();
    assert!(patch.apply(&saved, &overrides).is_err());
    let overrides = InstanceOverrides {
        service_account_default_token_lifetime_days: Some(10),
        ..Default::default()
    };
    assert!(patch.apply(&saved, &overrides).is_ok());
}

#[test]
fn deployment_override_conflicts_and_invalid_inputs_do_not_mutate_saved_choices() {
    let saved = InstanceConfiguration::default();
    let overrides = InstanceOverrides {
        scripts_enabled: Some(false),
        log_verbosity: Some("citadel_server=debug,sqlx=warn".into()),
        ..Default::default()
    };
    assert!(!saved.effective(&overrides).scripts_enabled);
    assert_eq!(
        saved.effective(&overrides).log_verbosity,
        "citadel_server=debug,sqlx=warn"
    );
    for input in [
        r#"{"scriptsEnabled":true}"#,
        r#"{"scriptsEnabled":null}"#,
        r#"{"passwordMinimumLength":7}"#,
        r#"{"passwordMinimumLength":129}"#,
        r#"{"statisticsCollectionIntervalSeconds":0}"#,
        r#"{"scriptsMaximumParallelRuns":65}"#,
        r#"{"scriptsMaximumRemoteRuns":257}"#,
        r#"{"buildPreparationSlots":17}"#,
        r#"{"buildTrackingSlots":129}"#,
        r#"{"backupTrackingSlots":129}"#,
        r#"{"backupDefaultTimeoutSeconds":4}"#,
        r#"{"logVerbosity":"sqlx=trace"}"#,
        r#"{"agentImageOverride":"invalid image"}"#,
    ] {
        let patch: PatchInstanceConfiguration = serde_json::from_str(input).unwrap();
        assert!(patch.apply(&saved, &overrides).is_err(), "accepted {input}");
    }
    assert!(serde_json::from_str::<PatchInstanceConfiguration>(r#"{"unknown":1}"#).is_err());
    assert!(
        serde_json::from_str::<PatchInstanceConfiguration>(
            r#"{"scriptsDefaultTimeoutSeconds":1.5}"#
        )
        .is_err()
    );
    assert_eq!(saved, InstanceConfiguration::default());
}

#[test]
fn every_promoted_field_round_trips_and_can_be_partially_patched() {
    let defaults = InstanceConfiguration::default();
    let json = serde_json::to_value(&defaults).unwrap();
    assert_eq!(json.as_object().unwrap().len(), 25);
    for (name, default) in json.as_object().unwrap() {
        let value = match default {
            serde_json::Value::Bool(v) => serde_json::json!(!v),
            serde_json::Value::Number(v) => serde_json::json!(v.as_u64().unwrap() + 1),
            serde_json::Value::String(_) if name == "logFormat" => serde_json::json!("Json"),
            serde_json::Value::String(_) if name == "logVerbosity" => serde_json::json!("debug"),
            _ => serde_json::json!("registry.example.test/citadel/agent:custom"),
        };
        let patch: PatchInstanceConfiguration =
            serde_json::from_value(serde_json::json!({name: value})).unwrap();
        let next = patch.apply(&defaults, &Default::default()).unwrap();
        let mut expected = json.clone();
        expected[name] = value;
        assert_eq!(serde_json::to_value(&next).unwrap(), expected);
        assert_eq!(
            serde_json::from_value::<InstanceConfiguration>(expected).unwrap(),
            next
        );
    }
}

#[test]
fn log_format_keeps_stored_json_values_and_rejects_unknown_formats() {
    for (json, expected) in [("\"Text\"", LogFormat::Text), ("\"Json\"", LogFormat::Json)] {
        assert_eq!(serde_json::from_str::<LogFormat>(json).unwrap(), expected);
        assert_eq!(serde_json::to_string(&expected).unwrap(), json);
    }
    assert_eq!(
        serde_json::from_str::<InstanceConfiguration>("{}")
            .unwrap()
            .log_format,
        LogFormat::Text
    );
    assert!(serde_json::from_str::<PatchInstanceConfiguration>(r#"{"logFormat":"xml"}"#).is_err());
}

#[test]
fn https_policy_checks_the_effective_configuration_and_only_the_deployment_connector() {
    let saved = InstanceConfiguration::default();
    let patch: PatchInstanceConfiguration =
        serde_json::from_str(r#"{"requireDirectAgentHttps":true}"#).unwrap();
    let next = patch.apply(&saved, &InstanceOverrides::default()).unwrap();
    assert!(next.validate_deployment(true).is_err());
    assert!(next.validate_deployment(false).is_ok());
    assert!(saved.validate_deployment(true).is_ok());
    let overrides = InstanceOverrides {
        require_direct_agent_https: Some(false),
        ..Default::default()
    };
    assert!(next.effective(&overrides).validate_deployment(true).is_ok());
}
