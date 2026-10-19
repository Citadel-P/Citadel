mod support {
    pub mod source_dependencies;
}
use support::source_dependencies::Dependencies;

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
    for root in [
        "src/realtime",
        "src/realtime_groups",
        "src/license_realtime.rs",
    ] {
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
                "HttpState",
                "api::error",
                "api::request_validation",
                "citadel_adapters::",
                "citadel_docker::",
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
    let source = platform_route_sources();
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
    let source = platform_route_sources();
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
        // Cancellation is exercised through a real HTTP -> Edge log read in
        // platforms_http::logs; the spelling of an RAII guard is not its contract.
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

fn fixture(path: &Path) -> bool {
    let stem = path.file_stem().unwrap().to_string_lossy();
    stem == "tests"
        || stem == "test_support"
        || stem.ends_with("_tests")
        || path.components().any(|part| part.as_os_str() == "tests")
}

fn assert_no_dependencies(path: &Path, forbidden: &[&str]) {
    let dependencies = Dependencies::parse(&std::fs::read_to_string(path).unwrap());
    for dependency in forbidden {
        assert!(
            !dependencies.contains(dependency),
            "{} depends on {dependency}",
            path.display()
        );
    }
}

#[test]
fn every_feature_keeps_external_io_and_transport_implementations_out() {
    let features = Path::new(env!("CARGO_MANIFEST_DIR")).join("../features");
    for path in sources(&features).into_iter().filter(|path| !fixture(path)) {
        let dependencies = Dependencies::parse(&std::fs::read_to_string(&path).unwrap());
        assert!(
            dependencies.filesystem_probe().is_none(),
            "{} performs filesystem probe {:?}; use a workspace port",
            path.display(),
            dependencies.filesystem_probe()
        );
        assert_no_dependencies(
            &path,
            &[
                "sqlx",
                "axum",
                "utoipa",
                "reqwest",
                "tonic",
                "citadel_adapters",
                "citadel_contracts",
                "citadel_docker_api",
                "citadel_docker",
                "citadel_processes",
                "citadel_runtime",
                "std::fs",
                "tokio::fs",
                "std::process",
                "tokio::process",
                "std::net::TcpStream",
                "std::net::TcpListener",
                "std::net::UdpSocket",
                "tokio::net",
                "std::env::var",
                "std::env::var_os",
            ],
        );
    }
}

#[test]
fn server_database_handles_stay_in_composition_and_startup() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    for path in sources(&root).into_iter().filter(|path| !fixture(path)) {
        // These are the executable composition/lifecycle roots, never route or worker code.
        let relative = path.strip_prefix(&root).unwrap();
        let wiring = relative.starts_with("composition")
            || ["app.rs", "cli.rs", "startup.rs", "router.rs"]
                .iter()
                .any(|allowed| relative == Path::new(allowed));
        if !wiring {
            assert_no_dependencies(&path, &["sqlx", "citadel_adapters::persistence"]);
        }
    }
}

#[test]
fn all_http_and_realtime_handlers_consume_ports_not_concrete_adapters() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    for area in ["api", "realtime", "realtime_groups"] {
        for path in sources(&root.join(area))
            .into_iter()
            .filter(|path| !fixture(path))
        {
            assert_no_dependencies(
                &path,
                &[
                    "citadel_adapters",
                    "citadel_contracts",
                    "citadel_docker_api",
                    "citadel_docker",
                    "sqlx",
                ],
            );
        }
    }
}

#[test]
fn agent_has_no_database_access() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../agent/src");
    for path in sources(&root).into_iter().filter(|path| !fixture(path)) {
        assert_no_dependencies(
            &path,
            &["sqlx", "citadel_database", "citadel_adapters::persistence"],
        );
    }
}

#[test]
fn sql_execution_and_row_mapping_belong_to_postgres_persistence() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    for area in [
        "server/src",
        "agent/src",
        "infrastructure/adapters/src",
        "infrastructure/docker/src",
    ] {
        for path in sources(&root.join(area))
            .into_iter()
            .filter(|path| !fixture(path))
        {
            if path.starts_with(root.join("infrastructure/adapters/src/persistence/postgres")) {
                continue;
            }
            let dependencies = Dependencies::parse(&std::fs::read_to_string(&path).unwrap());
            assert!(
                dependencies.sqlx_implementation().is_none(),
                "{} uses SQLx implementation {:?}; only pool/error wiring is allowed here",
                path.display(),
                dependencies.sqlx_implementation()
            );
            assert_no_dependencies(
                &path,
                &[
                    "sqlx::query",
                    "sqlx::query_as",
                    "sqlx::query_scalar",
                    "sqlx::raw_sql",
                    "sqlx::QueryBuilder",
                    "sqlx::Row",
                    "sqlx::FromRow",
                    "sqlx::Executor",
                    "sqlx::postgres::PgRow",
                    "sqlx::postgres::PgListener",
                ],
            );
        }
    }
}

fn platform_route_sources() -> String {
    fn read(directory: &Path, result: &mut String) {
        for entry in std::fs::read_dir(directory).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                read(&path, result);
            } else if path.extension().is_some_and(|ext| ext == "rs") {
                result.push_str(&std::fs::read_to_string(path).unwrap());
                result.push('\n');
            }
        }
    }
    let mut result = String::new();
    read(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("src/api/routes/platforms"),
        &mut result,
    );
    result
}
