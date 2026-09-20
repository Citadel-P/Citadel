use super::*;

#[test]
fn automation_activity_preserves_wire_identity_and_status() {
    let resource = Uuid::now_v7();
    let actor = ActorId::new(Uuid::now_v7());
    let run = Uuid::now_v7();
    for (info, expected) in [
        (
            ActivityEventInfo::ActionRunQueued {
                run_id: run,
                trigger: "Manual".into(),
            },
            ActivityStatus::Information,
        ),
        (
            ActivityEventInfo::ActionRunSucceeded {
                run_id: run,
                trigger: "Manual".into(),
                exit_code: Some(0),
                duration_ms: Some(12),
            },
            ActivityStatus::Success,
        ),
        (
            ActivityEventInfo::ActionRunFailed {
                run_id: run,
                trigger: "Manual".into(),
                exit_code: Some(7),
                duration_ms: Some(12),
                error_message: Some("failed".into()),
            },
            ActivityStatus::Failure,
        ),
        (
            ActivityEventInfo::ActionRunCancelled {
                run_id: run,
                trigger: "Manual".into(),
            },
            ActivityStatus::Warning,
        ),
    ] {
        let event =
            ActivityEvent::new_automation_event(resource, "Action".into(), actor, info, Utc::now())
                .unwrap();
        assert_eq!(event.status(), expected);
        assert_eq!(
            event.resource_type(),
            ActivityResourceType::AutomationAction
        );
        let info = serde_json::to_value(event.info()).unwrap();
        assert_eq!(info["RunId"], run.to_string());
        assert_eq!(info["Trigger"], "Manual");
    }
}

#[test]
fn profile_activity_derives_its_discriminators() {
    let event = ActivityEvent::new_user_event(
        Uuid::now_v7(),
        "Owner".to_owned(),
        ActorId::new(Uuid::now_v7()),
        ActivityEventInfo::user_profile_updated("Old owner".to_owned(), "Owner".to_owned()),
        Utc::now(),
    )
    .unwrap();

    assert_eq!(event.event_type(), ActivityEventType::UserProfileUpdated);
    assert_eq!(event.resource_type(), ActivityResourceType::User);
    assert_eq!(event.status(), ActivityStatus::Success);
}

#[test]
fn empty_preference_changes_are_rejected() {
    assert_eq!(
        ActivityEventInfo::user_preferences_updated(Vec::new()),
        Err(ActivityInvariantError::EmptyChanges)
    );
}

#[test]
fn revoked_other_sessions_requires_a_positive_count() {
    assert_eq!(
        ActivityEventInfo::user_other_sessions_revoked(0),
        Err(ActivityInvariantError::InvalidCount)
    );
}

#[test]
fn user_lifecycle_activity_uses_the_compatible_safe_payload_shape() {
    let snapshot = UserActivitySnapshot {
        email: "owner@example.test".to_owned(),
        is_enabled: true,
        team_ids: Vec::new(),
        role_ids: vec![Uuid::now_v7()],
        resource_accesses: Vec::new(),
    };
    let info = ActivityEventInfo::user_updated(snapshot.clone(), snapshot, true);
    let json = serde_json::to_value(&info).unwrap();
    assert_eq!(json["$type"], "UserUpdated");
    assert_eq!(json["PasswordChanged"], true);
    assert!(json.get("Password").is_none());
    assert!(json.get("PasswordHash").is_none());
    assert_eq!(info.event_type(), ActivityEventType::UserUpdated);
}

#[test]
fn mfa_activity_payloads_cannot_contain_credentials() {
    let target = Uuid::now_v7();
    let info = ActivityEventInfo::user_mfa_reset_by_administrator(target);
    let json = serde_json::to_value(&info).unwrap();

    assert_eq!(json["$type"], "UserMfaResetByAdministrator");
    assert_eq!(json["TargetUserId"], target.to_string());
    assert!(json.get("Code").is_none());
    assert!(json.get("Secret").is_none());
    assert!(json.get("RecoveryCode").is_none());
}

#[test]
fn team_lifecycle_activity_uses_the_compatible_safe_payload_shape() {
    let snapshot = TeamActivitySnapshot {
        is_enabled: true,
        member_actor_ids: vec![Uuid::now_v7()],
        role_ids: vec![Uuid::now_v7()],
        resource_accesses: Vec::new(),
    };
    let info = ActivityEventInfo::team_updated(snapshot.clone(), snapshot);
    let event = ActivityEvent::new_team_event(
        Uuid::now_v7(),
        "Operations".to_owned(),
        ActorId::new(Uuid::now_v7()),
        info,
        Utc::now(),
    )
    .unwrap();
    let json = serde_json::to_value(event.info()).unwrap();

    assert_eq!(event.event_type(), ActivityEventType::TeamUpdated);
    assert_eq!(event.resource_type(), ActivityResourceType::Team);
    assert_eq!(json["$type"], "TeamUpdated");
    assert!(json.get("Password").is_none());
    assert!(json.get("Token").is_none());
}

#[test]
fn deployment_activity_uses_the_existing_dotnet_payload_shape() {
    let deployment_id = Uuid::now_v7();
    let platform_id = Uuid::now_v7();
    let event = ActivityEvent::new_deployment_event(
        deployment_id,
        "web".to_owned(),
        platform_id,
        ActorId::new(Uuid::now_v7()),
        ActivityEventInfo::deployment_created(DeploymentActivitySnapshot {
            id: deployment_id,
            name: "web".to_owned(),
            platform_id,
            description: None,
            spec: serde_json::json!({"Image":{"$type":"Local","ImageId":"image"}}),
        }),
        Utc::now(),
    )
    .unwrap();
    let info = serde_json::to_value(event.info()).unwrap();

    assert_eq!(event.platform_id(), Some(platform_id));
    assert_eq!(event.resource_type(), ActivityResourceType::Deployment);
    assert_eq!(event.event_type(), ActivityEventType::DeploymentCreated);
    assert_eq!(event.status(), ActivityStatus::Information);
    assert_eq!(info["$type"], "DeploymentCreated");
    assert_eq!(info["Deployment"]["Id"], deployment_id.to_string());
    assert_eq!(info["Deployment"]["PlatformId"], platform_id.to_string());
}

#[test]
fn swarm_service_activity_uses_the_existing_dotnet_payload_shape() {
    let service_id = Uuid::now_v7();
    let platform_id = Uuid::now_v7();
    let operation_id = Uuid::now_v7();
    let event = ActivityEvent::new_swarm_service_event(
        service_id,
        "redis".to_owned(),
        platform_id,
        ActorId::new(Uuid::now_v7()),
        ActivityEventInfo::swarm_service_completed(
            "Scale",
            operation_id,
            Some(3),
            vec!["warning".to_owned()],
        ),
        ActivityStatus::Success,
        Utc::now(),
    )
    .unwrap();
    let json = serde_json::to_value(event.info()).unwrap();

    assert_eq!(event.resource_type(), ActivityResourceType::SwarmService);
    assert_eq!(event.event_type(), ActivityEventType::SwarmServiceScaled);
    assert_eq!(event.platform_id(), Some(platform_id));
    assert_eq!(json["$type"], "SwarmServiceScaled");
    assert_eq!(json["OperationId"], operation_id.to_string());
    assert_eq!(json["Replicas"], 3);
    assert_eq!(json["Warnings"], serde_json::json!(["warning"]));
}

#[test]
fn webhook_activities_keep_dotnet_discriminators_and_safe_shared_metadata() {
    for (resource, discriminator) in [
        (
            ActivityResourceType::GitRepository,
            "GitRepoWebhookReceived",
        ),
        (ActivityResourceType::Stack, "StackWebhookReceived"),
        (ActivityResourceType::Build, "BuildWebhookReceived"),
        (
            ActivityResourceType::SwarmService,
            "SwarmServiceWebhookReceived",
        ),
    ] {
        let request = Uuid::now_v7();
        let event = ActivityEvent::new_webhook_event(
            Uuid::now_v7(),
            "resource".into(),
            None,
            resource,
            WebhookActivityDetails {
                request_id: request,
                auth_type: "generic".into(),
                execution: "update".into(),
                status: "queued",
                reason: None,
                source: WebhookActivitySource::default(),
                dispatched_branch: None,
                dispatched_commit_sha: None,
            },
            Utc::now(),
        )
        .unwrap();
        let json = serde_json::to_value(event.info()).unwrap();
        assert_eq!(json["$type"], discriminator);
        assert_eq!(json["RequestId"], request.to_string());
        assert_eq!(json["Status"], "queued");
        assert_eq!(event.status(), ActivityStatus::Success);
        assert_eq!(event.resource_type(), resource);
        assert!(json.get("Headers").is_none() && json.get("Body").is_none());
    }
}

#[test]
fn role_lifecycle_activity_uses_the_compatible_safe_payload_shape() {
    let snapshot = RoleActivitySnapshot {
        role_type: "Custom".into(),
        permissions: vec![RolePermissionActivitySnapshot {
            resource_type: ResourceType::Registry,
            permission_level: PermissionLevel::Read,
            specific_permissions: 0,
        }],
    };
    let info = ActivityEventInfo::role_updated(snapshot.clone(), snapshot);
    let event = ActivityEvent::new_role_event(
        Uuid::now_v7(),
        "Registry reader".to_owned(),
        ActorId::new(Uuid::now_v7()),
        info,
        Utc::now(),
    )
    .unwrap();
    let json = serde_json::to_value(event.info()).unwrap();

    assert_eq!(event.event_type(), ActivityEventType::RoleUpdated);
    assert_eq!(event.resource_type(), ActivityResourceType::Role);
    assert_eq!(json["$type"], "RoleUpdated");
    assert_eq!(json["NewRole"]["RoleType"], "Custom");
    assert!(json.get("Password").is_none());
    assert!(json.get("Token").is_none());
}

#[test]
fn oidc_provider_activity_never_contains_client_credentials() {
    let snapshot = OidcProviderActivitySnapshot {
        id: Uuid::now_v7(),
        name: "corporate".to_owned(),
        description: None,
        display_name: "Corporate login".to_owned(),
        issuer: "https://issuer.example.test".to_owned(),
        client_id: "citadel-client".to_owned(),
        scopes: "openid profile email".to_owned(),
        enabled: true,
        auto_provision_users: false,
        allow_email_auto_link: true,
        require_email_verified: true,
        allowed_email_domains: Some("example.test".to_owned()),
        required_claim_name: None,
        required_claim_values: None,
        default_role_id: None,
    };
    let info = ActivityEventInfo::oidc_provider_created(snapshot);
    let event = ActivityEvent::new_oidc_provider_event(
        Uuid::now_v7(),
        "corporate".to_owned(),
        ActorId::new(Uuid::now_v7()),
        info,
        Utc::now(),
    )
    .unwrap();
    let json = serde_json::to_value(event.info()).unwrap();

    assert_eq!(event.event_type(), ActivityEventType::OidcProviderCreated);
    assert_eq!(event.resource_type(), ActivityResourceType::OidcProvider);
    assert_eq!(json["$type"], "OidcProviderCreated");
    assert!(json["Provider"].get("ClientSecret").is_none());
    assert!(json["Provider"].get("ClientSecretCiphertext").is_none());
}

#[test]
fn catalog_activity_uses_compatible_fields_and_git_creation_status() {
    let snapshot = GitRepositoryActivitySnapshot {
        id: Uuid::now_v7(),
        name: "repository".to_owned(),
        description: None,
        url: "https://git.example.test/team/repository".to_owned(),
        default_branch: "main".to_owned(),
        git_account_id: None,
        sync_mode: "Manual".to_owned(),
        sync_interval_minutes: None,
        webhook: None,
        on_clone: None,
        on_pull: None,
        resolved_commit_sha: None,
    };
    let event = ActivityEvent::new_git_repository_event(
        snapshot.id,
        snapshot.name.clone(),
        ActorId::new(Uuid::now_v7()),
        ActivityEventInfo::git_repo_created(snapshot),
        Utc::now(),
    )
    .unwrap();
    let json = serde_json::to_value(event.info()).unwrap();

    assert_eq!(event.status(), ActivityStatus::Information);
    assert_eq!(json["$type"], "GitRepoCreated");
    assert!(json.get("GitRepo").is_some());
    assert!(json.get("Repository").is_none());
}
