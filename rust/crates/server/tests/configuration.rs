//! Exercise the real startup parser in isolated processes, without a database.
use serde_json::Value;
use std::process::{Command, Output};

fn run(root: &std::path::Path, overrides: &[(&str, &str)], command: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_citadel-server"))
        .env_clear()
        .env(
            "DATABASE_URL",
            "postgres://user:database-secret@localhost/citadel",
        )
        .env("Transport__Mode", "Disabled")
        .env("Transport__PublicUrl", "http://localhost:8000")
        .env("CITADEL_DATA_ROOT", root)
        .envs(overrides.iter().copied())
        .args(command)
        .output()
        .unwrap()
}

#[test]
fn configured_limits_reach_startup_and_generated_keys_survive_restarts() {
    let root = std::env::temp_dir().join(format!("citadel-config-{}", uuid::Uuid::now_v7()));
    let overrides = [
        ("ServiceAccounts__DefaultTokenLifetimeDays", "7"),
        ("ServiceAccounts__MaximumTokenLifetimeDays", "14"),
        ("ServiceAccounts__MaximumActiveTokensPerAccount", "2"),
        ("ServiceAccounts__LastUsedWriteIntervalMinutes", "1"),
        ("EdgeAgent__AgentImageRepository", "registry.test/agent"),
        ("EdgeAgent__AgentImageTag", "v2.3.4+build.5"),
        ("EdgeAgent__NodeAgentBootstrapMinutes", "3"),
        ("EdgeAgent__NodeAgentSetupMinutes", "2"),
        ("EdgeAgent__NodeAgentLimitNanoCpus", "1000000000"),
        ("EdgeAgent__SupportedNodeArchitectures__0", "arm64"),
        ("Backups__DefaultTimeoutSeconds", "30"),
        ("Backups__MaxLogBytes", "2048"),
        ("Backups__AllowedCorePaths__0", "/mnt/backup"),
        ("Backups__RepositoryLeaseSeconds", "60"),
        ("Builds__RunRetentionDays", "12"),
        ("Mfa__MaxFailedAttempts", "3"),
        ("Mfa__MaximumFailedAttempts", "9"),
    ];
    let first = run(&root, &overrides, &["print-effective-config"]);
    assert!(
        first.status.success(),
        "{}",
        String::from_utf8_lossy(&first.stderr)
    );
    let config: Value = serde_json::from_slice(&first.stdout).unwrap();
    assert_eq!(
        config["serviceAccountLimits"]["defaultTokenLifetimeDays"],
        7
    );
    assert_eq!(
        config["serviceAccountLimits"]["maximumTokenLifetimeDays"],
        14
    );
    assert_eq!(
        config["serviceAccountLimits"]["maximumActiveTokensPerAccount"],
        2
    );
    assert_eq!(config["serviceAccountLastUsedIntervalSeconds"], 60);
    assert_eq!(
        config["execution"]["agentImage"],
        "registry.test/agent:2.3.4"
    );
    assert_eq!(config["execution"]["nodeAgent"]["bootstrapSeconds"], 180);
    assert_eq!(config["execution"]["nodeAgent"]["setupSeconds"], 120);
    assert_eq!(config["execution"]["nodeAgent"]["nanoCpus"], 1_000_000_000);
    assert_eq!(
        config["execution"]["nodeAgent"]["architectures"],
        serde_json::json!(["arm64"])
    );
    assert_eq!(config["execution"]["backups"]["defaultTimeoutSeconds"], 30);
    assert_eq!(config["execution"]["backups"]["maximumLogBytes"], 2048);
    assert_eq!(
        config["execution"]["backups"]["allowedCorePaths"],
        serde_json::json!(["/mnt/backup"])
    );
    assert_eq!(config["execution"]["backups"]["repositoryLeaseSeconds"], 60);
    assert_eq!(config["execution"]["buildRetentionDays"], 12);
    assert_eq!(config["mfaMaximumFailedAttempts"], 3);
    let jwt = std::fs::read_to_string(root.join("jwtsecret")).unwrap();
    let encryption = std::fs::read_to_string(root.join("secret-encryption-key")).unwrap();
    let second = run(&root, &overrides, &["print-effective-config"]);
    assert!(second.status.success());
    assert_eq!(
        std::fs::read_to_string(root.join("jwtsecret")).unwrap(),
        jwt
    );
    assert_eq!(
        std::fs::read_to_string(root.join("secret-encryption-key")).unwrap(),
        encryption
    );
    let output = String::from_utf8(second.stdout).unwrap();
    for secret in [&jwt, &encryption, "database-secret"] {
        assert!(!output.contains(secret));
    }
    for invalid in [
        ("ServiceAccounts__MaximumTokenLifetimeDays", "0"),
        ("ServiceAccounts__MaximumActiveTokensPerAccount", "0"),
        ("EdgeAgent__NodeAgentLimitMemoryBytes", "1"),
        ("EdgeAgent__NodeAgentSetupMinutes", "0"),
        ("Backups__DefaultTimeoutSeconds", "0"),
        ("Builds__RunRetentionDays", "-1"),
        (
            "AgentTransport__CaCertificatePath",
            "/missing-certificate.pem",
        ),
    ] {
        let output = run(&root, &[invalid], &["print-effective-config"]);
        assert!(!output.status.success(), "{} accepted", invalid.0);
        assert!(String::from_utf8_lossy(&output.stderr).contains(invalid.0));
    }
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn explicit_keys_stay_external_and_restore_never_generates_replacement_keys() {
    let root = std::env::temp_dir().join(format!("citadel-config-{}", uuid::Uuid::now_v7()));
    let output = run(
        &root,
        &[
            ("Jwt__Key", "explicit-signing-key-with-at-least-32-bytes"),
            (
                "Secrets__EncryptionKey",
                "AQIDBAUGBwgJCgsMDQ4PEBESExQVFhcYGRobHB0eHyA=",
            ),
        ],
        &["print-effective-config"],
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!root.exists());
    let output = run(
        &root,
        &[],
        &[
            "restore-system",
            "--bundle",
            "/missing-bundle",
            "--confirm-instance-replacement",
        ],
    );
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("original key"));
    assert!(!root.exists());
}
