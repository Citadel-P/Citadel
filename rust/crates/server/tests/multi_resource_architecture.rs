use std::path::{Path, PathBuf};
fn sources(path: &Path) -> Vec<PathBuf> {
    std::fs::read_dir(path)
        .unwrap()
        .flat_map(|entry| {
            let path = entry.unwrap().path();
            if path.file_stem().is_some_and(|name| {
                let name = name.to_string_lossy();
                name == "tests" || name.ends_with("_tests")
            }) {
                return vec![];
            }
            if path.is_dir() {
                sources(&path)
            } else if path.extension().is_some_and(|e| e == "rs") {
                vec![path]
            } else {
                vec![]
            }
        })
        .collect()
}
#[test]
fn migrated_resources_keep_transport_and_detached_tasks_out_of_features() {
    let crates = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    for feature in ["builds", "git", "backups", "automation", "alerts"] {
        for path in sources(&crates.join("features").join(feature).join("src")) {
            let source = std::fs::read_to_string(&path).unwrap();
            for forbidden in [
                "AutomationActionView",
                "AutomationRunView",
                "AlertChannelView",
                "AlertRuleView",
                "AlertEventView",
                "utoipa",
                "axum::",
                "tokio::spawn",
                "pub struct BuildProjectView",
                "pub struct BuildRunView",
                "pub struct BuildAgentPoolView",
                "pub struct GitAccountView",
                "pub struct GitRepositoryView",
                "pub struct BackupRepositoryView",
                "pub struct BackupPolicyView",
                "pub struct BackupRunView",
                "pub struct BackupRestoreRunView",
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
    }
    for (feature, resources) in [
        ("automation", &["actions", "runs"][..]),
        ("alerts", &["channels", "rules", "events"][..]),
        ("builds", &["projects", "runs", "agent_pools"][..]),
        ("git", &["accounts", "repositories"][..]),
        (
            "backups",
            &["repositories", "policies", "runs", "restores"][..],
        ),
    ] {
        for resource in resources {
            assert!(
                crates
                    .join("features")
                    .join(feature)
                    .join("src")
                    .join(resource)
                    .join("model.rs")
                    .exists()
            );
        }
    }
}
#[test]
fn migrated_postgres_maps_semantic_data_without_http_projection() {
    let crates = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    for feature in ["builds", "git", "backups", "automation", "alerts"] {
        for path in sources(
            &crates
                .join("infrastructure/adapters/src/persistence/postgres")
                .join(feature),
        ) {
            let source = std::fs::read_to_string(&path).unwrap();
            for forbidden in [
                "AutomationActionView",
                "AutomationRunView",
                "AlertChannelView",
                "AlertRuleView",
                "AlertEventView",
                "GitRepositoryView",
                "BuildProjectView",
                "BackupRunView",
                "public_latest_activity",
                "Capabilities",
                "READ_MASK",
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
