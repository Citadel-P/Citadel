use std::process::Command;

#[cfg(target_os = "linux")]
#[test]
fn recovery_key_diagnostic_is_read_only_and_needs_no_database() {
    use base64::Engine;
    use std::{fs, os::unix::fs::PermissionsExt};
    let root =
        std::env::temp_dir().join(format!("citadel-update-key-test-{}", uuid::Uuid::now_v7()));
    fs::create_dir(&root).unwrap();
    fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
    let encryption = base64::engine::general_purpose::STANDARD.encode([7; 32]);
    let run = |external: bool| {
        let mut command = Command::new(env!("CARGO_BIN_EXE_citadel-server"));
        command
            .env_clear()
            .env("CITADEL_DATA_ROOT", &root)
            .env("CITADEL_UPDATE_CHECKS_ENABLED", "false")
            .arg("check-update-recovery-keys");
        if external {
            command
                .env(
                    "CITADEL_JWT_KEY",
                    "external-fixture-key-material-1234567890",
                )
                .env("CITADEL_SECRETS_ENCRYPTION_KEY", &encryption);
        }
        command.output().unwrap()
    };
    let missing = run(false);
    assert!(!missing.status.success());
    assert!(missing.stdout.is_empty());
    assert_eq!(fs::read_dir(&root).unwrap().count(), 0);
    fs::create_dir(root.join("agent")).unwrap();
    for (file, bytes) in [
        (
            "jwtsecret",
            b"file-fixture-key-material-1234567890".as_slice(),
        ),
        ("secret-encryption-key", encryption.as_bytes()),
        ("agent/signing-key", &[7; 32]),
    ] {
        fs::write(root.join(file), bytes).unwrap();
        fs::set_permissions(root.join(file), fs::Permissions::from_mode(0o600)).unwrap();
    }
    for external in [false, true] {
        let output = run(external);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        let source = if external {
            "Environment"
        } else {
            "DataVolume"
        };
        assert_eq!(
            value,
            serde_json::json!({"jwt":source, "secretsEncryption":source, "agentSigning":"DataVolume"})
        );
        assert_eq!(fs::read_dir(&root).unwrap().count(), 3);
    }
    fs::remove_file(root.join("agent/signing-key")).unwrap();
    assert!(!run(true).status.success());
    assert!(!root.join("agent/signing-key").exists());
    let output = Command::new(env!("CARGO_BIN_EXE_citadel-server"))
        .env_clear()
        .env("CITADEL_DATA_ROOT", &root)
        .env(
            "CITADEL_CORE_AGENT_PRIVATE_KEY_PATH",
            "/private-fixture-key",
        )
        .arg("check-update-recovery-keys")
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(!String::from_utf8_lossy(&output.stderr).contains("/private-fixture-key"));
    fs::remove_dir_all(&root).unwrap();
}

#[cfg(target_os = "linux")]
#[test]
fn installation_check_rejects_missing_state_without_creating_it_or_starting_core() {
    use std::os::unix::fs::DirBuilderExt;
    let temporary =
        std::env::temp_dir().join(format!("citadel-update-check-{}", uuid::Uuid::now_v7()));
    std::fs::DirBuilder::new()
        .mode(0o700)
        .create(&temporary)
        .unwrap();
    let missing = temporary.join("missing");
    for path in [
        std::path::Path::new("relative"),
        &missing,
        temporary.as_path(),
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_citadel-server"))
            .env_clear()
            .env("CITADEL_UPDATE_CHECKS_ENABLED", "false")
            .arg("check-update-installation")
            .arg("--state-directory")
            .arg(path)
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(error.contains("Update"), "{error}");
        assert!(!error.contains("PostgreSQL"));
        assert!(!error.contains("Release checks are disabled"));
    }
    assert_eq!(std::fs::read_dir(&temporary).unwrap().count(), 0);
    std::fs::remove_dir(temporary).unwrap();
}

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
    for verify in [false, true] {
        let mut command = Command::new(env!("CARGO_BIN_EXE_citadel-server"));
        command.env_clear().arg("instance-identity-json");
        if verify {
            command.arg("--verify-migrations");
        }
        let output = command.output().unwrap();
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        assert!(String::from_utf8_lossy(&output.stderr).contains("PostgreSQL"));
    }
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

#[cfg(target_os = "linux")]
#[test]
fn target_inspection_rejects_disabled_policy_and_invalid_inputs_before_docker() {
    for (policy, version, architecture, expected) in [
        (
            "false",
            "1.2.3",
            "amd64",
            "Release checks are disabled by server configuration",
        ),
        ("invalid", "1.2.3", "amd64", "CITADEL_UPDATE_CHECKS_ENABLED"),
        ("true", "latest", "amd64", "Invalid release version"),
        ("true", "1.2.3", "other", "invalid value"),
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_citadel-server"))
            .env_clear()
            .env("CITADEL_UPDATE_CHECKS_ENABLED", policy)
            .args([
                "inspect-update-release",
                "--version",
                version,
                "--architecture",
                architecture,
            ])
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(error.contains(expected), "{error}");
        assert!(!error.contains("PostgreSQL"));
    }
}

#[cfg(target_os = "linux")]
#[test]
fn helper_handoff_command_rejects_invalid_input_without_starting_core() {
    use std::io::Write;
    use std::process::Stdio;
    for payload in [b"{}".to_vec(), vec![b' '; 256 * 1024 + 1]] {
        let mut child = Command::new(env!("CARGO_BIN_EXE_citadel-server"))
            .env_clear()
            .arg("update-helper-handoff")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child.stdin.take().unwrap().write_all(&payload).unwrap();
        let output = child.wait_with_output().unwrap();
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        assert!(!String::from_utf8_lossy(&output.stderr).contains("DATABASE_URL"));
    }
}
