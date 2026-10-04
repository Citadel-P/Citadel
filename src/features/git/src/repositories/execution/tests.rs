use super::*;
use super::{credentials::normalize_remote_url, discovery::compose_projects};

fn entry(path: &str) -> GitTreeEntry {
    GitTreeEntry {
        name: path
            .rsplit_once('/')
            .map_or(path, |(_, name)| name)
            .to_owned(),
        path: path.to_owned(),
        entry_type: GitEntryType::File,
        size: Some(1),
        mode: "100644".to_owned(),
        object_id: "a".repeat(40),
    }
}

#[test]
fn account_relative_urls_preserve_transport_and_ssh_username() {
    assert_eq!(
        normalize_remote_url(
            "team/repository.git",
            Some(("git.example.test", GitTransport::Https, None)),
        )
        .unwrap(),
        "https://git.example.test/team/repository.git"
    );
    assert_eq!(
        normalize_remote_url(
            "team/repository.git",
            Some(("git.example.test", GitTransport::Ssh, Some("deploy"))),
        )
        .unwrap(),
        "deploy@git.example.test:team/repository.git"
    );
}

#[test]
fn repository_paths_reject_metadata_and_traversal() {
    assert!(validate_repository_path("services/api", true).is_ok());
    assert!(validate_repository_path("../secret", true).is_err());
    assert!(validate_repository_path(".git/config", true).is_err());
    assert!(validate_repository_path("", true).is_err());
    assert_eq!(validate_repository_path("", false).unwrap(), "");
}

#[test]
fn compose_discovery_groups_compose_and_env_files_without_materialization() {
    let projects = compose_projects(&[
        entry("compose.yaml"),
        entry("compose.override.yml"),
        entry(".env"),
        entry("apps/api/docker-compose.yml"),
        entry("apps/api/.env"),
        entry("README.md"),
    ]);

    assert_eq!(projects.len(), 2);
    assert_eq!(projects[0].working_directory, ".");
    assert_eq!(projects[0].compose_paths[0], "compose.yaml");
    assert_eq!(projects[0].env_file_paths, [".env"]);
    assert_eq!(projects[1].working_directory, "apps/api");
    assert_eq!(projects[1].suggested_watch_paths, ["apps/api/**"]);
}

#[test]
fn compose_discovery_recognizes_named_files_without_matching_unrelated_yaml() {
    use super::discovery::is_compose_file;
    for name in [
        "compose.yml",
        "docker-compose.yaml",
        "sample-app-compose.yaml",
        "demo-app-compose-prod.yml",
        "compose.production.yaml",
        "app.compose.yml",
        "app_compose_prod.yaml",
        "Sample-Compose.YAML",
    ] {
        assert!(is_compose_file(name), "{name}");
    }
    for name in [
        "decompose.yaml",
        "composer.yml",
        "compose.json",
        "compose.yaml.bak",
        "app.yaml",
    ] {
        assert!(!is_compose_file(name), "{name}");
    }
}

#[test]
fn compose_discovery_lists_named_env_files_in_their_project_directory() {
    let projects = compose_projects(&[
        entry("docker-compose.yaml"),
        entry("sample-app-compose.yaml"),
        entry(".env"),
        entry(".env.production"),
        entry("demo-app.env"),
        entry("sample-app.env.local"),
        entry("environment.yaml"),
        entry("env.rs"),
        entry("apps/api/compose.yaml"),
        entry("apps/api/.env.local"),
    ]);
    assert_eq!(projects.len(), 2);
    assert_eq!(
        projects[0].compose_paths,
        ["docker-compose.yaml", "sample-app-compose.yaml"]
    );
    assert_eq!(
        projects[0].env_file_paths,
        [".env", ".env.production", "demo-app.env", "sample-app.env.local"]
    );
    assert_eq!(projects[1].env_file_paths, ["apps/api/.env.local"]);
}
