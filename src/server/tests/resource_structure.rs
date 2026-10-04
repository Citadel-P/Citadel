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
    assert!(!workspace.contains("src/domain"));
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
    assert!(!workspace.contains("src/resources"));
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
    // Shared offline SQLx query metadata is a supported source-root directory.
    assert_eq!(
        groups,
        [
            ".sqlx",
            "agent",
            "features",
            "frontend",
            "infrastructure",
            "server",
            "tools"
        ]
    );
    let features = root.join("features").canonicalize().unwrap();
    let infrastructure = root.join("infrastructure").canonicalize().unwrap();
    let server = root.join("server").canonicalize().unwrap();
    let agent = root.join("agent").canonicalize().unwrap();
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
                        !target.starts_with(&server) && !target.starts_with(&agent),
                        "{} depends on an executable host",
                        manifest.display()
                    );
                }
                if owner.starts_with(&agent) || owner.starts_with(&server) {
                    assert!(
                        target.starts_with(&features) || target.starts_with(&infrastructure),
                        "{} depends on another executable host",
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

#[test]
fn umbrella_services_have_feature_owners_without_reverse_dependencies() {
    let crates = crates();
    let features = crates.join("features");
    for old in ["application", "domain"] {
        assert!(!features.join(old).exists(), "{old} umbrella was recreated");
    }
    for manifest in manifests(&crates) {
        let source = std::fs::read_to_string(&manifest).unwrap();
        for forbidden in ["citadel-application", "citadel-domain"] {
            assert!(
                !source.contains(forbidden),
                "{} depends on {forbidden}",
                manifest.display()
            );
        }
    }
    for owner in ["activities", "licensing"] {
        let root = features.join(owner);
        let manifest = std::fs::read_to_string(root.join("Cargo.toml")).unwrap();
        assert!(
            !manifest.contains("citadel-identity"),
            "{owner} depends on Identity"
        );
        if owner == "licensing" {
            assert!(
                !manifest.contains("citadel-activities"),
                "Licensing depends on Activities"
            );
        }
        for path in feature_sources(&root.join("src")) {
            let source = std::fs::read_to_string(&path).unwrap();
            for forbidden in [
                "ActorPrincipal",
                "IdentityError",
                "public_activity_info",
                "public_latest_activity",
                "pub struct InstallLicenseRequest",
                "pub struct LicenseView",
                "pub struct LicenseCapabilityView",
                "pub struct LicenseEntitlementsView",
                "pub struct LicenseRequestView",
                "https://citadel.local/problems/",
            ] {
                assert!(
                    !source.contains(forbidden),
                    "{} contains {forbidden}",
                    path.display()
                );
            }
        }
    }
}

#[test]
fn server_resources_live_under_api_and_composition_has_no_hidden_configuration() {
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    for entry in std::fs::read_dir(&source).unwrap() {
        let entry = entry.unwrap();
        let name = entry.file_name();
        let name = name.to_str().unwrap();
        if name.ends_with("_http.rs") {
            assert!(
                ["application_info_http.rs", "diagnostics_http.rs"].contains(&name),
                "resource endpoint family belongs under api/: {name}"
            );
        }
    }
    assert!(!source.join("state.rs").exists());
    for entry in std::fs::read_dir(source.join("composition")).unwrap() {
        let path = entry.unwrap().path();
        let code = std::fs::read_to_string(&path).unwrap();
        assert!(!code.contains("std::env"), "{}", path.display());
        assert!(!code.contains("env::var"), "{}", path.display());
    }
    let graph = std::fs::read_to_string(source.join("composition/mod.rs")).unwrap();
    let graph = graph
        .split("pub struct ServerComponents {")
        .nth(1)
        .unwrap()
        .split('}')
        .next()
        .unwrap();
    for worker_only in ["runtime_targets", "image_scanner", "alert_deliveries"] {
        assert!(
            !graph.contains(worker_only),
            "{worker_only} belongs to Jobs"
        );
    }
}

#[test]
fn feature_facades_and_dependencies_keep_presentation_in_server() {
    for manifest in manifests(&crates().join("features")) {
        let package = manifest.parent().unwrap();
        let dependencies = std::fs::read_to_string(&manifest).unwrap();
        assert!(!dependencies.contains("utoipa"), "{}", manifest.display());
        let facade = std::fs::read_to_string(package.join("src/lib.rs")).unwrap();
        for line in facade.lines().map(str::trim) {
            for definition in ["pub struct ", "pub enum ", "pub trait ", "pub fn ", "impl "] {
                assert!(
                    !line.starts_with(definition),
                    "{}: {line}",
                    package.display()
                );
            }
        }
        for path in feature_sources(&package.join("src")) {
            let source = std::fs::read_to_string(&path).unwrap();
            assert!(!source.contains("utoipa"), "{}", path.display());
            assert!(!source.contains("#[schema("), "{}", path.display());
        }
    }
}

#[test]
fn features_inject_task_ownership_except_bounded_password_cpu_work() {
    let features = crates().join("features");
    let mut blocking = 0;
    for path in feature_sources(&features) {
        let relative = path.strip_prefix(&features).unwrap();
        if path.file_stem().unwrap() == "tests"
            || relative
                .components()
                .any(|part| part.as_os_str() == "tests")
            || path
                .file_stem()
                .unwrap()
                .to_string_lossy()
                .ends_with("_tests")
        {
            continue;
        }
        let source = std::fs::read_to_string(&path).unwrap();
        let production = source.split("#[cfg(test)]").next().unwrap();
        for unowned in [
            "tokio::spawn(",
            "tokio::task::spawn(",
            "TaskTracker::new(",
            "JoinSet::new(",
        ] {
            assert!(
                !production.contains(unowned),
                "{}: {unowned}",
                path.display()
            );
        }
        let count = production.matches("tokio::task::spawn_blocking(").count();
        if count != 0 {
            assert_eq!(
                relative,
                Path::new("identity/src/authentication/service.rs")
            );
            blocking += count;
        }
    }
    assert_eq!(
        blocking, 2,
        "review finite, semaphore-bounded password jobs explicitly"
    );
}

#[test]
fn server_owns_error_rendering_and_startup_uses_only_endpoint_metadata() {
    let server = crates().join("server/src");
    for path in feature_sources(&server.join("api")) {
        let source = std::fs::read_to_string(&path).unwrap();
        for legacy in [
            "identity_error_response",
            "IdentityHttpResult",
            "IdentityHttpError",
        ] {
            assert!(!source.contains(legacy), "{}: {legacy}", path.display());
        }
        if ![
            "actors",
            "authentication",
            "mfa",
            "oidc",
            "profile",
            "roles",
            "service_accounts",
            "teams",
            "users",
        ]
        .contains(&path.file_stem().unwrap().to_str().unwrap())
        {
            assert!(!source.contains("-> IdentityError"), "{}", path.display());
        }
    }
    let catalog = std::fs::read_to_string(server.join("api/endpoint_catalog.rs")).unwrap();
    let production = catalog.split("#[cfg(test)]").next().unwrap();
    assert!(!production.contains("crate::openapi::"));
    assert!(!production.contains("json_document"));
    for path in feature_sources(&server) {
        let source = std::fs::read_to_string(&path).unwrap();
        assert!(!source.contains("#[path ="), "{}", path.display());
    }
}

#[test]
fn routes_and_api_resources_have_separate_owners() {
    let server = crates().join("server/src");
    let api = server.join("api");
    assert!(api.join("routes/deployments.rs").is_file());
    assert!(api.join("resources/deployments/requests.rs").is_file());
    assert!(api.join("resources/deployments/views.rs").is_file());
    assert!(!api.join("deployments").exists());
    assert!(!api.join("routes.rs").exists());
    for entry in std::fs::read_dir(api.join("routes")).unwrap() {
        let path = entry.unwrap().path();
        assert!(
            path.is_file(),
            "route families must use a single named file: {}",
            path.display()
        );
    }
    for path in feature_sources(&api.join("routes")) {
        assert_ne!(path.file_name().unwrap(), "handlers.rs");
        let code = std::fs::read_to_string(&path).unwrap();
        for forbidden in [
            "pub struct Tracked",
            "RealtimeNotifier {",
            "ToSchema",
            "Serialize",
            "Deserialize",
            "#[serde",
            "#[schema",
        ] {
            assert!(!code.contains(forbidden), "{}: {forbidden}", path.display());
        }
    }
    for path in feature_sources(&api.join("resources")) {
        let code = std::fs::read_to_string(&path).unwrap();
        for forbidden in [
            "crate::api::routes::",
            "axum::",
            "sqlx::",
            "#[utoipa::path",
            "DynamicTasks",
        ] {
            assert!(!code.contains(forbidden), "{}: {forbidden}", path.display());
        }
    }
}
