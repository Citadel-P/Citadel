//! Guard the migrated family; other resources adopt policies in their own phases.
use std::path::Path;

#[test]
fn deployment_handlers_use_named_policies() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    check(&root.join("api/deployments/handlers.rs"));
    check(&root.join("api/deployments/adoption.rs"));
}

#[test]
fn shared_entry_points_use_deployment_policies() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    // Scope shared modules to Deployment paths: their other resource families
    // and container-owner fallback checks migrate in later phases.
    for (file, marker, policies) in [
        (
            "platforms_http/container_inspection.rs",
            "async fn read_deployment(",
            &[
                "require_resource::<ReadDeployment>",
                "require_resource::<InspectDeployment>",
            ][..],
        ),
        (
            "realtime_groups/logs.rs",
            "\"StartDeploymentLogs\" =>",
            &["deployment_permission::<citadel_deployments::permissions::ViewDeploymentLogs>"][..],
        ),
        (
            "realtime_groups/terminal.rs",
            "if owner == Owner::Deployment",
            &["deployment_permission::<citadel_deployments::permissions::OpenDeploymentTerminal>"]
                [..],
        ),
        (
            "realtime_groups/reader.rs",
            "async fn deployment_permission<",
            &["require_resource::<P>"][..],
        ),
    ] {
        let source = std::fs::read_to_string(root.join(file)).unwrap();
        assert_policy_source(block(&source, marker), file, policies);
    }

    let file = "platforms_http/statistics.rs";
    let source = std::fs::read_to_string(root.join(file)).unwrap();
    let workload = block(&source, "async fn workload(");
    let deployment = workload
        .split_once("StatisticsWorkload::Deployment =>")
        .expect("Deployment statistics branch must exist")
        .1
        .split_once("StatisticsWorkload::Stack")
        .expect("Stack statistics branch must delimit the Deployment branch")
        .0;
    assert_policy_source(
        deployment,
        file,
        &["require_resource::<citadel_deployments::permissions::ReadDeployment>"],
    );
}

fn assert_policy_source(source: &str, location: &str, policies: &[&str]) {
    let source: String = source.chars().filter(|ch| !ch.is_whitespace()).collect();
    for legacy in [
        ".authorize(",
        ".authorize_resource(",
        ".permission(",
        ".permission_for_resource(",
        ".global_permission(",
        "PermissionLevel::",
        "SpecificPermission::",
    ] {
        assert!(!source.contains(legacy), "{location} reintroduced {legacy}");
    }
    for policy in policies {
        assert!(source.contains(policy), "{location} must call {policy}");
    }
}

fn block<'a>(source: &'a str, marker: &str) -> &'a str {
    let start = source
        .find(marker)
        .unwrap_or_else(|| panic!("missing {marker}"));
    let body = start + source[start..].find('{').expect("missing block body");
    let mut depth = 0;
    for (offset, ch) in source[body..].char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return &source[body..=body + offset];
                }
            }
            _ => {}
        }
    }
    panic!("unbalanced block for {marker}");
}

fn check(path: &Path) {
    if path.is_dir() {
        for entry in std::fs::read_dir(path).unwrap() {
            check(&entry.unwrap().path());
        }
    } else if path.extension().is_some_and(|extension| extension == "rs") {
        let source = std::fs::read_to_string(path).unwrap();
        for legacy in [".authorize(", ".authorize_resource("] {
            assert!(
                !source.contains(legacy),
                "{} reintroduced {legacy}",
                path.display()
            );
        }
        // Collection capabilities legitimately evaluate the projected hierarchy.
        // Exempt only that balanced function body, never everything following it.
        let source = without_collection_capabilities(&source);
        for legacy in ["PermissionLevel::", "SpecificPermission::"] {
            assert!(
                !source.contains(legacy),
                "{} reintroduced {legacy}",
                path.display()
            );
        }
    }
}

fn without_collection_capabilities(source: &str) -> String {
    let Some(start) = source.find("async fn collection_capabilities(") else {
        return source.to_owned();
    };
    let body = start + source[start..].find('{').unwrap();
    let mut depth = 0;
    for (offset, ch) in source[body..].char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return format!("{}{}", &source[..start], &source[body + offset + 1..]);
                }
            }
            _ => {}
        }
    }
    panic!("unbalanced collection capability function");
}

#[test]
fn collection_exception_does_not_hide_later_declarations() {
    let code =
        "async fn collection_capabilities() { if true { } } fn added() { PermissionLevel::Read; }";
    assert!(without_collection_capabilities(code).contains("PermissionLevel::Read"));
}

#[test]
fn shared_scope_keeps_nested_checks_and_excludes_other_resources() {
    let source = r#"if owner == Owner::Deployment {
        if enabled { self.permission(p, kind, id, specific).await?; }
    } else { self.permission(p, stack, id, specific).await?; }"#;
    let deployment = block(source, "if owner == Owner::Deployment");
    assert!(deployment.contains("self.permission(p, kind"));
    assert!(!deployment.contains("self.permission(p, stack"));
    assert!(std::panic::catch_unwind(|| assert_policy_source(deployment, "fixture", &[])).is_err());
}

#[test]
fn deployment_architecture_keeps_presentation_out_of_feature_and_persistence() {
    let crates = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let feature = crates.join("deployments");
    let manifest = std::fs::read_to_string(feature.join("Cargo.toml")).unwrap();
    for dependency in ["utoipa", "axum", "citadel-server", "citadel-adapters"] {
        assert!(
            !manifest.contains(dependency),
            "feature depends on {dependency}"
        );
    }
    for directory in [
        feature.join("src"),
        crates.join("adapters/src/postgres/deployments"),
    ] {
        check_resource_boundary(&directory);
    }
    for removed in [
        "deployments/src/model.rs",
        "deployments/src/service.rs",
        "adapters/src/deployment_store.rs",
        "server/src/deployments_http.rs",
    ] {
        assert!(
            !crates.join(removed).exists(),
            "legacy module returned: {removed}"
        );
    }
}

fn check_resource_boundary(path: &Path) {
    if path.is_dir() {
        for entry in std::fs::read_dir(path).unwrap() {
            check_resource_boundary(&entry.unwrap().path());
        }
        return;
    }
    if path.extension().is_none_or(|extension| extension != "rs")
        || path.file_name().is_some_and(|name| name == "tests.rs")
    {
        return;
    }
    let source = std::fs::read_to_string(path).unwrap();
    for forbidden in [
        "DeploymentView",
        "DeploymentCapabilities",
        "ResourceCapabilities",
        "utoipa",
        "citadel_server",
        "public_latest_activity",
        "tokio::spawn",
    ] {
        assert!(
            !source.contains(forbidden),
            "{} contains {forbidden}",
            path.display()
        );
    }
}
