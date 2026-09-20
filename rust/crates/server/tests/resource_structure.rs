use std::path::{Path, PathBuf};

fn crates() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

#[test]
fn features_use_resource_roles_without_domain_application_layers() {
    let crates = crates();
    for feature in [
        "identity",
        "deployments",
        "stacks",
        "swarm-services",
        "git",
        "builds",
        "backups",
        "automation",
        "alerts",
        "platforms",
        "tags",
        "registries",
        "bindings",
        "discovery",
        "activities",
        "licensing",
    ] {
        let source = crates.join("features").join(feature).join("src");
        assert!(source.is_dir(), "{feature}");
        assert!(!source.join("domain").exists(), "{feature}");
        assert!(!source.join("application").exists(), "{feature}");
    }
    for resource in [
        "identity/users",
        "identity/teams",
        "identity/roles",
        "identity/service_accounts",
        "identity/actors",
        "identity/profile",
        "identity/authentication",
        "identity/mfa",
        "identity/oidc",
        "git/accounts",
        "git/repositories",
    ] {
        let (feature, resource) = resource.split_once('/').unwrap();
        let directory = crates
            .join("features")
            .join(feature)
            .join("src")
            .join(resource);
        for role in ["model", "commands", "read_models", "repository", "service"] {
            assert!(
                directory.join(format!("{role}.rs")).exists()
                    || directory.join(role).join("mod.rs").exists(),
                "{feature}/{resource} is missing {role}"
            );
        }
    }
    for feature in ["deployments", "stacks", "swarm-services"] {
        let source = crates.join("features").join(feature).join("src");
        assert!(source.is_dir(), "{feature}");
        for role in ["model", "commands", "read_models", "repository", "service"] {
            assert!(
                source.join(format!("{role}.rs")).exists()
                    || source.join(role).join("mod.rs").exists(),
                "{feature} is missing {role}"
            );
        }
    }
}

#[test]
fn domain_umbrella_has_no_workspace_member_or_dependency() {
    let crates = crates();
    assert!(!crates.join("domain").exists());
    let workspace = std::fs::read_to_string(crates.parent().unwrap().join("Cargo.toml")).unwrap();
    assert!(!workspace.contains("crates/domain"));
    for manifest in manifests(&crates) {
        if manifest.exists() {
            let source = std::fs::read_to_string(&manifest).unwrap();
            assert!(!source.contains("citadel-domain"), "{}", manifest.display());
        }
    }
}

#[test]
fn metadata_features_have_explicit_owners_and_no_umbrella_dependency() {
    let crates = crates();
    assert!(!crates.join("resources").exists());
    let workspace = std::fs::read_to_string(crates.parent().unwrap().join("Cargo.toml")).unwrap();
    assert!(!workspace.contains("crates/resources"));
    for manifest in manifests(&crates) {
        if manifest.exists() {
            let source = std::fs::read_to_string(&manifest).unwrap();
            assert!(
                !source.contains("citadel-resources"),
                "{}",
                manifest.display()
            );
        }
    }
    for owner in ["tags", "registries", "bindings", "discovery", "primitives"] {
        let manifest =
            std::fs::read_to_string(crates.join("features").join(owner).join("Cargo.toml"))
                .unwrap();
        for forbidden in [
            "citadel-git",
            "citadel-identity",
            "citadel-platforms",
            "citadel-adapters",
            "citadel-application",
        ] {
            assert!(!manifest.contains(forbidden), "{owner}: {forbidden}");
        }
    }
    let bindings = std::fs::read_to_string(crates.join("features/bindings/Cargo.toml")).unwrap();
    assert!(!bindings.contains("citadel-registries"));
    let git = std::fs::read_to_string(crates.join("features/git/Cargo.toml")).unwrap();
    assert!(!git.contains("citadel-registries"));
    assert!(!git.contains("citadel-bindings"));
}

fn manifests(directory: &Path) -> Vec<PathBuf> {
    std::fs::read_dir(directory)
        .unwrap()
        .flat_map(|entry| {
            let path = entry.unwrap().path();
            if path.is_dir() && path.join("Cargo.toml").is_file() {
                vec![path.join("Cargo.toml")]
            } else if path.is_dir()
                && matches!(
                    path.file_name().and_then(|s| s.to_str()),
                    Some("features" | "infrastructure")
                )
            {
                manifests(&path)
            } else {
                vec![]
            }
        })
        .collect()
}

#[test]
fn architectural_groups_enforce_inward_dependencies() {
    let root = crates();
    let mut groups = std::fs::read_dir(&root)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect::<Vec<_>>();
    groups.sort();
    assert_eq!(groups, ["features", "infrastructure", "server"]);
    let features = root.join("features").canonicalize().unwrap();
    let infrastructure = root.join("infrastructure").canonicalize().unwrap();
    let server = root.join("server").canonicalize().unwrap();
    let output = std::process::Command::new(env!("CARGO"))
        .args(["metadata", "--format-version=1", "--no-deps", "--offline"])
        .current_dir(root.parent().unwrap())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let metadata: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    for package in metadata["packages"].as_array().unwrap() {
        let manifest = Path::new(package["manifest_path"].as_str().unwrap());
        let owner = manifest.parent().unwrap().canonicalize().unwrap();
        for dependency in package["dependencies"].as_array().unwrap() {
            if let Some(path) = dependency["path"].as_str() {
                let target = Path::new(path).canonicalize().unwrap();
                if owner.starts_with(&features) {
                    assert!(
                        target.starts_with(&features),
                        "{} points outward to {}",
                        manifest.display(),
                        target.display()
                    );
                }
                if owner.starts_with(&infrastructure) {
                    assert!(
                        !target.starts_with(&server),
                        "{} depends on Server",
                        manifest.display()
                    );
                }
            }
        }
    }
    for feature in ["execution", "git", "automation"] {
        for path in feature_sources(&features.join(feature).join("src")) {
            let source = std::fs::read_to_string(&path).unwrap();
            assert!(
                !source.contains("tokio::process::Command"),
                "{} owns concrete process execution",
                path.display()
            );
            assert!(
                !source.contains("citadel_processes"),
                "{} imports the process implementation",
                path.display()
            );
        }
    }
}

fn feature_sources(directory: &Path) -> Vec<PathBuf> {
    std::fs::read_dir(directory)
        .unwrap()
        .flat_map(|entry| {
            let path = entry.unwrap().path();
            if path.is_dir() {
                feature_sources(&path)
            } else if path.extension().is_some_and(|e| e == "rs") {
                vec![path]
            } else {
                vec![]
            }
        })
        .collect()
}
