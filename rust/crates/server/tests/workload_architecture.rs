//! Phase 7 guards for the migrated workload resource boundary.
use std::path::{Path, PathBuf};

fn rust_files(path: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for entry in std::fs::read_dir(path).unwrap() {
        let path = entry.unwrap().path();
        if path.file_stem().is_some_and(|name| {
            let name = name.to_string_lossy();
            name == "tests" || name.ends_with("_tests")
        }) {
            continue;
        }
        if path.is_dir() {
            files.extend(rust_files(&path));
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            files.push(path);
        }
    }
    files
}

#[test]
fn workload_features_own_business_models_and_ports_without_transport_or_detached_tasks() {
    let crates = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    for feature in ["stacks", "swarm-services"] {
        for path in rust_files(&crates.join("features").join(feature).join("src")) {
            let source = std::fs::read_to_string(&path).unwrap();
            for forbidden in [
                "utoipa",
                "axum::",
                "tokio::spawn",
                "pub struct StackView",
                "pub struct ManagedSwarmServiceView",
                "pub struct StackCapabilities",
                "pub struct SwarmServiceCapabilities",
            ] {
                assert!(
                    !source.contains(forbidden),
                    "{} reintroduced {forbidden}",
                    path.display()
                );
            }
        }
        let facade =
            std::fs::read_to_string(crates.join("features").join(feature).join("src/lib.rs"))
                .unwrap();
        assert!(
            !facade
                .lines()
                .any(|line| line.starts_with("pub use ") && line.contains("::*"))
        );
        for slot in [
            "commands.rs",
            "read_models.rs",
            "repository.rs",
            "runtime.rs",
            "tasks.rs",
            "permissions.rs",
            "model/mod.rs",
            "service/mod.rs",
        ] {
            assert!(
                crates
                    .join("features")
                    .join(feature)
                    .join("src")
                    .join(slot)
                    .exists()
            );
        }
    }
}

#[test]
fn workload_persistence_never_constructs_http_views_or_capabilities() {
    let crates = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    for resource in ["stacks", "swarm_services"] {
        for path in rust_files(
            &crates
                .join("infrastructure/adapters/src/persistence/postgres")
                .join(resource),
        ) {
            let source = std::fs::read_to_string(&path).unwrap();
            for forbidden in [
                "StackView",
                "ManagedSwarmServiceView",
                "Capabilities",
                "public_latest_activity",
                "CASE WHEN $2 THEN 7",
                "THEN 127",
            ] {
                assert!(
                    !source.contains(forbidden),
                    "{} reintroduced {forbidden}",
                    path.display()
                );
            }
        }
    }
}
