use std::path::{Path, PathBuf};

fn sources(directory: &Path) -> Vec<PathBuf> {
    std::fs::read_dir(directory)
        .unwrap()
        .flat_map(|entry| {
            let path = entry.unwrap().path();
            if path.is_dir() {
                sources(&path)
            } else if path.extension().is_some_and(|extension| extension == "rs") {
                vec![path]
            } else {
                vec![]
            }
        })
        .collect()
}

#[test]
fn features_keep_http_presentation_and_docker_protocols_out() {
    let crates = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    for feature in [
        "platforms",
        "identity",
        "tags",
        "registries",
        "bindings",
        "discovery",
    ] {
        let root = crates.join("features").join(feature);
        for path in sources(&root.join("src")) {
            let source = std::fs::read_to_string(&path).unwrap();
            for forbidden in ["utoipa", "axum::", "citadel_docker_api"] {
                assert!(
                    !source.contains(forbidden),
                    "{}: {forbidden}",
                    path.display()
                );
            }
            for line in source.lines().map(str::trim) {
                if let Some(declaration) = line.strip_prefix("pub struct ") {
                    let name = declaration.split([' ', '<', '{', '(']).next().unwrap();
                    assert!(!name.ends_with("View"), "{}: {name}", path.display());
                }
            }
        }
        let facade = std::fs::read_to_string(root.join("src/lib.rs")).unwrap();
        for line in facade.lines() {
            assert!(!line.starts_with("pub struct ") && !line.starts_with("pub trait "));
            assert!(!line.starts_with("impl "));
            assert!(!(line.starts_with("pub use ") && line.contains("::*")));
        }
    }
}

#[test]
fn runtime_targets_and_stack_claims_use_closed_platform_classifications() {
    let crates = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    for relative in [
        "infrastructure/adapters/src/connectors/routing/platforms/registry.rs",
        "features/platforms/src/jobs/inventory_reconciliation.rs",
        "features/stacks/src/model/operations.rs",
    ] {
        let source = std::fs::read_to_string(crates.join(relative)).unwrap();
        assert!(!source.contains("pub platform_type: String"), "{relative}");
        assert!(!source.contains("pub connector_type: String"), "{relative}");
        assert!(
            !source.contains("platform_type.eq_ignore_ascii_case"),
            "{relative}"
        );
    }
    for owner in ["tags", "registries", "bindings"] {
        let repository = std::fs::read_to_string(
            crates
                .join("features")
                .join(owner)
                .join("src/repository.rs"),
        )
        .unwrap();
        assert!(!repository.contains("GitRepositoryPersistence"));
        assert!(!repository.contains("update_platform_description"));
    }
}

#[test]
fn generic_runtime_has_no_identity_dependency() {
    let output = std::process::Command::new(env!("CARGO"))
        .args([
            "metadata",
            "--no-deps",
            "--format-version",
            "1",
            "--offline",
        ])
        .current_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let metadata: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let runtime = metadata["packages"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["name"] == "citadel-runtime")
        .unwrap();
    assert!(
        runtime["dependencies"]
            .as_array()
            .unwrap()
            .iter()
            .all(|d| d["name"] != "citadel-identity")
    );
}

#[test]
fn realtime_does_not_depend_on_http_routes_or_select_transports() {
    let server = Path::new(env!("CARGO_MANIFEST_DIR"));
    for root in ["src/realtime_groups", "src/realtime/shared_reads.rs"] {
        let root = server.join(root);
        let paths = if root.is_dir() {
            sources(&root)
        } else {
            vec![root]
        };
        for path in paths {
            let source = std::fs::read_to_string(&path).unwrap();
            for forbidden in [
                "api::routes",
                "PlatformsHttpState",
                "RuntimeRef",
                "DockerClient",
                "AgentClient",
                "sqlx::",
            ] {
                assert!(
                    !source.contains(forbidden),
                    "{}: {forbidden}",
                    path.display()
                );
            }
        }
    }
}

#[test]
fn platform_http_depends_on_feature_ports() {
    let source = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("src/api/routes/platforms.rs"),
    )
    .unwrap();
    for forbidden in [
        "sqlx::",
        "PgPool",
        "citadel_adapters",
        "RuntimeRef",
        "DockerClient",
        "AgentClient",
        "runtime_for(",
    ] {
        assert!(
            !source.contains(forbidden),
            "Platform SQL belongs in persistence adapters: {forbidden}"
        );
    }
}

#[test]
fn migrated_platform_handlers_use_capabilities_instead_of_transport_switches() {
    let source = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("src/api/routes/platforms.rs"),
    )
    .unwrap();
    for name in [
        "inspect_target",
        "exposed_ports",
        "inspect_image",
        "task_logs",
        "read_logs",
        "task_statistics",
        "read_task_runtime",
        "list_networks",
        "lookup_platform_resources",
        "get_network",
        "create_network",
        "delete_networks",
        "list_volumes",
        "get_volume",
        "create_volume",
        "delete_volumes",
        "pull",
        "delete_images",
        "prune",
    ] {
        let signature = format!("async fn {name}(");
        let body = source
            .split_once(&signature)
            .expect(name)
            .1
            .split_once("\n}\n")
            .expect(name)
            .0;
        for forbidden in ["RuntimeRef", "runtime_for(", "runtime_for_node("] {
            assert!(
                !body.contains(forbidden),
                "{name} must use feature capabilities, not {forbidden}"
            );
        }
        assert!(
            body.contains("drop_guard()"),
            "{name} must cancel work when the request ends"
        );
    }
}

#[test]
fn sensitive_workspace_io_belongs_to_infrastructure() {
    let crates = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    for relative in [
        "features/git/src/cli.rs",
        "features/git/src/repositories/execution/credentials.rs",
        "features/automation/src/service/execution.rs",
        "features/backups/src/service/backups.rs",
    ] {
        let source = std::fs::read_to_string(crates.join(relative)).unwrap();
        let production = source.split("#[cfg(test)]\nmod tests").next().unwrap();
        for forbidden in [
            "tokio::fs::",
            "std::fs::",
            "citadel_processes",
            "citadel_adapters",
        ] {
            assert!(!production.contains(forbidden), "{relative}: {forbidden}");
        }
    }
}
