use std::process::Command;

#[cfg(target_os = "linux")]
#[test]
fn registration_rejects_invalid_state_path_without_accessing_docker_or_database() {
    let output = Command::new(env!("CARGO_BIN_EXE_citadel-server"))
        .env_clear()
        .args([
            "register-update-installation",
            "--directory",
            "/opt/citadel",
            "--project",
            "citadel",
            "--compose-file",
            "/opt/citadel/docker-compose.yml",
            "--state-directory",
            "relative",
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("canonical absolute paths"));
}

#[test]
fn identity_diagnostic_requires_database_configuration_without_starting_core() {
    let output = Command::new(env!("CARGO_BIN_EXE_citadel-server"))
        .env_clear()
        .arg("instance-identity-json")
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("PostgreSQL"));
}

#[cfg(target_os = "linux")]
#[test]
fn compose_check_rejects_invalid_local_inputs_without_starting_core() {
    let output = Command::new(env!("CARGO_BIN_EXE_citadel-server"))
        .env_clear()
        .args([
            "check-update-compose",
            "--directory",
            "relative/path",
            "--project",
            "citadel",
            "--compose-file",
            "relative/path/docker-compose.yml",
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(error.contains("canonical absolute paths"), "{error}");
    assert!(!error.contains("PostgreSQL"));
}

#[test]
fn build_upgrade_descriptor_needs_no_database_and_qualifies_no_transitions() {
    let output = Command::new(env!("CARGO_BIN_EXE_citadel-server"))
        .env_clear()
        .env("CITADEL_UPDATE_CHECKS_ENABLED", "false")
        .args(["upgrade-descriptor-json"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let descriptor: citadel_updates::UpgradeDescriptor =
        serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(descriptor.schema_version, 1);
    assert_eq!(descriptor.product, "citadel");
    assert_eq!(descriptor.target_version, env!("CITADEL_BUILD_VERSION"));
    assert_eq!(
        descriptor.target_catalog_sha256,
        citadel_database::MigrationRunner::catalog_sha256().unwrap()
    );
    assert_eq!(
        descriptor.installer_format,
        citadel_updates::INSTALLER_FORMAT
    );
    assert_eq!(descriptor.update_protocol, citadel_updates::UPDATE_PROTOCOL);
    assert!(descriptor.sources.is_empty());
}

#[test]
fn verification_honours_server_network_policy_without_initializing_core() {
    let output = Command::new(env!("CARGO_BIN_EXE_citadel-server"))
        .env_clear()
        .env("CITADEL_UPDATE_CHECKS_ENABLED", "false")
        .args(["verify-update-release", "--version", "1.2.3"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(
        error.contains("Release checks are disabled by server configuration"),
        "{error}"
    );
    assert!(!error.contains("PostgreSQL"));
}

#[test]
fn verification_rejects_invalid_policy_and_noncanonical_versions_before_network_access() {
    for (policy, version, expected) in [
        ("invalid", "1.2.3", "CITADEL_UPDATE_CHECKS_ENABLED"),
        ("true", "latest", "Invalid release version"),
        ("true", "1.2.3/../../other", "Invalid release version"),
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_citadel-server"))
            .env_clear()
            .env("CITADEL_UPDATE_CHECKS_ENABLED", policy)
            .args(["verify-update-release", "--version", version])
            .output()
            .unwrap();
        assert!(!output.status.success());
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(error.contains(expected), "{error}");
        assert!(output.stdout.is_empty());
    }
}
