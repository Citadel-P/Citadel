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
fn phase10_features_keep_http_presentation_and_docker_protocols_out() {
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
        "server/src/runtime_targets.rs",
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
