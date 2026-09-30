use super::{requests::*, spec::*, views::*};
use citadel_git::repositories::PatchField;
use citadel_primitives::ResourceControlState;
use serde_json::json;
use uuid::Uuid;

#[test]
fn webhook_patch_preserves_omitted_null_and_value() {
    let patch: citadel_git::GitRepositoryPatch =
        serde_json::from_value::<GitRepositoryPatch>(json!({}))
            .unwrap()
            .into();
    assert!(matches!(patch.webhook, PatchField::Missing));
    let patch: citadel_git::GitRepositoryPatch =
        serde_json::from_value::<GitRepositoryPatch>(json!({"webhook":null}))
            .unwrap()
            .into();
    assert!(matches!(patch.webhook, PatchField::Null));
    let patch: citadel_git::GitRepositoryPatch = serde_json::from_value::<GitRepositoryPatch>(json!({
        "webhook":{"enabled":true,"provider":"Generic","authScheme":"BearerToken","secret":"fixture","branchFilter":"main"}
    })).unwrap().into();
    let PatchField::Value(config) = patch.webhook else {
        panic!("expected webhook config")
    };
    let config = serde_json::to_value(config).unwrap();
    assert_eq!(config["secret"], "fixture");
    assert_eq!(config["branchFilter"], "main");
    assert_eq!(config["provider"], "Generic");
    for invalid in [
        json!({"provider":"Invalid"}),
        json!({"authScheme":"Invalid"}),
        json!(false),
    ] {
        assert!(serde_json::from_value::<GitRepositoryPatch>(json!({"webhook":invalid})).is_err());
    }
}

#[test]
fn create_preserves_webhook_and_command_defaults() {
    let input: NewGitRepository = serde_json::from_value(json!({
        "name":"repo", "url":"https://example.com/repo.git", "defaultBranch":"main",
        "webhook":{}, "onClone":{"commands":["echo ready"]}
    }))
    .unwrap();
    let feature: citadel_git::CreateGitRepository = input.into();
    let config = feature.webhook.unwrap();
    let config = serde_json::to_value(config).unwrap();
    assert_eq!(config["enabled"], false);
    assert_eq!(config["provider"], "GitHub");
    assert_eq!(config["authScheme"], "GitHubHmacSha256");
    assert_eq!(feature.on_clone.unwrap().path, "./");
    assert_eq!(feature.sync_interval_minutes, Some(5));
}

fn repository() -> citadel_git::GitRepository {
    citadel_git::GitRepository {
        id: Uuid::new_v4(),
        name: "repo".into(),
        description: None,
        url: "https://example.com/repo.git".into(),
        default_branch: "main".into(),
        git_account_id: None,
        sync_mode: citadel_git::GitRepositorySyncMode::Manual,
        sync_interval_minutes: None,
        webhook: None,
        on_clone: None,
        on_pull: None,
        status: citadel_git::GitRepositoryStatus::Healthy,

        control_state: citadel_primitives::ResourceControlState::Idle,
        audit: citadel_primitives::AuditMetadata {
            created_at: chrono::Utc::now(),
            created_by_actor_id: citadel_primitives::ActorId::new(Uuid::new_v4()),
        },

        latest_activity: None,
        tags: vec![],
    }
}

#[test]
fn repository_views_validate_stored_vocabulary() {
    let view = GitRepositoryView::try_from(repository()).unwrap();
    assert_eq!(view.status, GitRepositoryStatus::Healthy);
    assert_eq!(view.control_state, ResourceControlState::Idle);
    for control_state in ["Idle", "Queued", "Processing"] {
        let mut resource = repository();
        resource.control_state = control_state.parse().unwrap();
        let view = GitRepositoryView::try_from(resource).unwrap();
        assert_eq!(
            serde_json::to_value(view).unwrap()["controlState"],
            control_state
        );
    }
    assert!(
        "invalid"
            .parse::<citadel_git::GitRepositoryStatus>()
            .is_err()
    );
    assert!("invalid".parse::<ResourceControlState>().is_err());
    assert!(
        serde_json::from_value::<citadel_primitives::WebhookConfig>(json!({"provider":"invalid"}))
            .is_err()
    );
}

#[test]
fn reference_status_includes_the_actual_syncing_state() {
    for status in ["Pending", "Syncing", "Healthy", "Degraded"] {
        let view = GitRepositoryRefView::try_from(citadel_git::GitRepositoryRef {
            id: Uuid::new_v4(),
            git_repository_id: Uuid::new_v4(),
            branch: "main".into(),
            resolved_commit_sha: None,
            status: status.parse().unwrap(),
            last_error: None,
            last_synced_at: chrono::Utc::now(),
        })
        .unwrap();
        assert_eq!(serde_json::to_value(view).unwrap()["status"], status);
    }
    assert!(serde_json::from_value::<GitRepositoryRefStatus>(json!("invalid")).is_err());
}

#[test]
fn browser_views_preserve_symlinks_submodules_and_renames() {
    let entry: GitDirectoryEntry = citadel_git::GitDirectoryEntry {
        name: "module".into(),
        path: "vendor/module".into(),
        entry_type: citadel_git::GitBrowserEntryType::Submodule,
        size: None,
        mode: "160000".into(),
        target_commit_sha: Some("abc123".into()),
    }
    .into();
    let wire = serde_json::to_value(entry).unwrap();
    assert_eq!(wire["type"], "Submodule");
    assert_eq!(wire["targetCommitSha"], "abc123");
    let path: GitChangedPath = citadel_git::GitChangedPath {
        status: citadel_git::GitChangedPathStatus::Renamed,
        path: "new.yml".into(),
        previous_path: Some("old.yml".into()),
    }
    .into();
    let wire = serde_json::to_value(path).unwrap();
    assert_eq!(wire["status"], "Renamed");
    assert_eq!(wire["previousPath"], "old.yml");
    assert_eq!(
        serde_json::to_value(GitEntryTypeView::from(
            citadel_git::GitBrowserEntryType::Symlink
        ))
        .unwrap(),
        "Symlink"
    );
}

#[test]
fn repository_schemas_derive_from_native_contracts() {
    let doc = serde_json::to_value(crate::openapi::document(false)).unwrap();
    let schemas = &doc["components"]["schemas"];
    assert_eq!(
        schemas["GitRepositoryRefStatus"]["enum"],
        json!(["Pending", "Syncing", "Healthy", "Degraded"])
    );
    assert_eq!(
        schemas["GitRepositoryView"]["properties"]["status"]["$ref"],
        "#/components/schemas/GitRepositoryStatus"
    );
    for (field, contract) in [
        ("NewGitRepository", "WebhookConfig"),
        ("GitRepositoryPatch", "WebhookPatch"),
        ("GitRepositoryConfigResponse", "WebhookConfig"),
        ("GitRepositoryView", "WebhookConfig"),
    ] {
        assert!(
            schemas[field]["properties"]["webhook"]
                .to_string()
                .contains(&format!("#/components/schemas/{contract}")),
            "{field}"
        );
    }
    for (path, schema) in [
        ("files", "GitDirectoryListing"),
        ("files/content", "GitFileContent"),
        ("compare", "GitCommitComparison"),
        ("compose-projects", "GitComposeDiscovery"),
    ] {
        assert_eq!(
            doc["paths"][format!("/api/v1/gitRepositories/{{id}}/{path}")]["get"]["responses"]["200"]
                ["content"]["application/json"]["schema"]["$ref"],
            format!("#/components/schemas/{schema}")
        );
    }
}
