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
        let source = crates.join(feature).join("src");
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
        let directory = crates.join(feature).join("src").join(resource);
        for role in ["model", "commands", "read_models", "repository", "service"] {
            assert!(
                directory.join(format!("{role}.rs")).exists()
                    || directory.join(role).join("mod.rs").exists(),
                "{feature}/{resource} is missing {role}"
            );
        }
    }
    for feature in ["deployments", "stacks", "swarm-services"] {
        let source = crates.join(feature).join("src");
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
    for entry in std::fs::read_dir(crates).unwrap() {
        let manifest = entry.unwrap().path().join("Cargo.toml");
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
    for entry in std::fs::read_dir(&crates).unwrap() {
        let manifest = entry.unwrap().path().join("Cargo.toml");
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
        let manifest = std::fs::read_to_string(crates.join(owner).join("Cargo.toml")).unwrap();
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
    let bindings = std::fs::read_to_string(crates.join("bindings/Cargo.toml")).unwrap();
    assert!(!bindings.contains("citadel-registries"));
    let git = std::fs::read_to_string(crates.join("git/Cargo.toml")).unwrap();
    assert!(!git.contains("citadel-registries"));
    assert!(!git.contains("citadel-bindings"));
}
