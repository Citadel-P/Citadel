//! Exercise the real startup parser in isolated processes, without a database.
use serde_json::Value;
use std::process::{Command, Output};

fn run(root: &std::path::Path, overrides: &[(&str, &str)], command: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_citadel-server"))
        .env_clear()
        .env(
            "CITADEL_DATABASE_URL",
            "postgres://user:database-secret@localhost/citadel",
        )
        .env("CITADEL_TRANSPORT_MODE", "Disabled")
        .env("CITADEL_PUBLIC_URL", "http://localhost:8000")
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
        ("CITADEL_SERVICE_ACCOUNTS_DEFAULT_TOKEN_LIFETIME_DAYS", "7"),
        ("CITADEL_SERVICE_ACCOUNTS_MAXIMUM_TOKEN_LIFETIME_DAYS", "14"),
        (
            "CITADEL_SERVICE_ACCOUNTS_MAXIMUM_ACTIVE_TOKENS_PER_ACCOUNT",
            "2",
        ),
        (
            "CITADEL_SERVICE_ACCOUNTS_LAST_USED_WRITE_INTERVAL_MINUTES",
            "1",
        ),
        ("CITADEL_EDGE_AGENT_IMAGE_REPOSITORY", "registry.test/agent"),
        ("CITADEL_EDGE_AGENT_IMAGE_TAG", "v2.3.4+build.5"),
        ("CITADEL_EDGE_AGENT_NODE_AGENT_BOOTSTRAP_MINUTES", "3"),
        ("CITADEL_EDGE_AGENT_NODE_AGENT_SETUP_MINUTES", "2"),
        (
            "CITADEL_EDGE_AGENT_NODE_AGENT_LIMIT_NANO_CPUS",
            "1000000000",
        ),
        ("CITADEL_EDGE_AGENT_SUPPORTED_NODE_ARCHITECTURE_0", "arm64"),
        ("CITADEL_BACKUPS_DEFAULT_TIMEOUT_SECONDS", "30"),
        ("CITADEL_BACKUPS_MAX_LOG_BYTES", "2048"),
        ("CITADEL_BACKUPS_ALLOWED_CORE_PATH_0", "/mnt/backup"),
        ("CITADEL_BACKUPS_REPOSITORY_LEASE_SECONDS", "90"),
        ("CITADEL_BUILDS_RUN_RETENTION_DAYS", "12"),
        ("CITADEL_MFA_MAX_FAILED_ATTEMPTS", "3"),
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
    assert_eq!(config["execution"]["backups"]["repositoryLeaseSeconds"], 90);
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
    let probe = Command::new(std::env::current_exe().unwrap())
        .env_clear()
        .env("CITADEL_TEST_DATABASE_SOURCE", "generated-keys")
        .env(
            "CITADEL_DATABASE_URL",
            "postgres://fixture:fixture@localhost/fixture",
        )
        .env("CITADEL_TRANSPORT_MODE", "Disabled")
        .env("CITADEL_DATA_ROOT", &root)
        .args(["--exact", "configuration_values_probe", "--nocapture"])
        .output()
        .unwrap();
    assert!(
        probe.status.success(),
        "{}",
        String::from_utf8_lossy(&probe.stderr)
    );
    let output = String::from_utf8(second.stdout).unwrap();
    for secret in [&jwt, &encryption, "database-secret"] {
        assert!(!output.contains(secret));
    }
    for invalid in [
        ("CITADEL_LOG_FORMAT", "yaml"),
        ("CITADEL_LOG_COLOR_ENABLED", "invalid"),
        ("CITADEL_SERVICE_ACCOUNTS_MAXIMUM_TOKEN_LIFETIME_DAYS", "0"),
        (
            "CITADEL_SERVICE_ACCOUNTS_MAXIMUM_ACTIVE_TOKENS_PER_ACCOUNT",
            "0",
        ),
        ("CITADEL_EDGE_AGENT_NODE_AGENT_LIMIT_MEMORY_BYTES", "1"),
        ("CITADEL_EDGE_AGENT_NODE_AGENT_SETUP_MINUTES", "0"),
        ("CITADEL_BACKUPS_DEFAULT_TIMEOUT_SECONDS", "0"),
        ("CITADEL_BUILDS_RUN_RETENTION_DAYS", "-1"),
        (
            "CITADEL_CORE_AGENT_CA_CERTIFICATE_PATH",
            "/missing-certificate.pem",
        ),
    ] {
        let output = run(&root, &[invalid], &["print-effective-config"]);
        assert!(!output.status.success(), "{} accepted", invalid.0);
        assert!(String::from_utf8_lossy(&output.stderr).contains(invalid.0));
    }
    std::fs::remove_dir_all(root).unwrap();
}

#[tokio::test]
async fn log_format_and_color_are_independent_and_piped_output_defaults_to_text() {
    use std::{process::Stdio, time::Duration};
    use tokio::io::{AsyncBufReadExt, BufReader};

    for (format, color) in [
        (None, None),
        (Some("text"), Some("false")),
        (Some("text"), Some("true")),
        (Some("json"), Some("false")),
        (Some("json"), Some("true")),
    ] {
        let root = std::env::temp_dir().join(format!("citadel-logging-{}", uuid::Uuid::now_v7()));
        let mut command = tokio::process::Command::new(env!("CARGO_BIN_EXE_citadel-server"));
        command
            .env_clear()
            // Startup logs precede the connection attempt; never use a real database.
            .env(
                "CITADEL_DATABASE_URL",
                "postgres://user:secret@127.0.0.1:1/citadel",
            )
            .env("CITADEL_TRANSPORT_MODE", "Disabled")
            .env("CITADEL_PUBLIC_URL", "http://localhost:8000")
            .env("CITADEL_DATA_ROOT", &root)
            .env(
                "CITADEL_JWT_KEY",
                "explicit-signing-key-with-at-least-32-bytes",
            )
            .env(
                "CITADEL_SECRETS_ENCRYPTION_KEY",
                "AQIDBAUGBwgJCgsMDQ4PEBESExQVFhcYGRobHB0eHyA=",
            )
            .arg("serve")
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true);
        if let Some(format) = format {
            command.env("CITADEL_LOG_FORMAT", format);
        }
        if let Some(color) = color {
            command.env("CITADEL_LOG_COLOR_ENABLED", color);
        }
        let mut child = command.spawn().unwrap();
        let mut lines = BufReader::new(child.stdout.take().unwrap()).lines();
        let line = tokio::time::timeout(Duration::from_secs(10), lines.next_line())
            .await
            .expect("startup must log promptly")
            .unwrap()
            .expect("startup log line");
        child.kill().await.unwrap();
        if format == Some("json") {
            let event: Value = serde_json::from_str(&line).unwrap();
            assert_eq!(event["level"], "INFO");
            assert_eq!(event["fields"]["message"], "Starting Citadel Core");
            assert!(!line.contains('\u{1b}'));
        } else {
            assert!(line.contains("INFO"));
            assert!(line.contains("Starting Citadel Core"));
            assert!(serde_json::from_str::<Value>(&line).is_err());
            assert_eq!(line.contains('\u{1b}'), color == Some("true"));
            assert!(!line.contains("citadel_server::"));
        }
        assert!(!line.contains("effective_configuration"));
        if root.exists() {
            std::fs::remove_dir_all(root).unwrap();
        }
    }
}

#[test]
fn explicit_keys_stay_external_and_restore_never_generates_replacement_keys() {
    let root = std::env::temp_dir().join(format!("citadel-config-{}", uuid::Uuid::now_v7()));
    let output = run(
        &root,
        &[
            (
                "CITADEL_JWT_KEY",
                "explicit-signing-key-with-at-least-32-bytes",
            ),
            (
                "CITADEL_SECRETS_ENCRYPTION_KEY",
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

#[test]
fn password_minimum_is_configurable_and_rejects_invalid_values() {
    let root =
        std::env::temp_dir().join(format!("citadel-password-config-{}", uuid::Uuid::now_v7()));
    for (overrides, expected) in [
        (vec![], 15),
        (vec![("CITADEL_PASSWORDS_MINIMUM_LENGTH", "8")], 8),
        (vec![("CITADEL_PASSWORDS_MINIMUM_LENGTH", "128")], 128),
    ] {
        let output = run(&root, &overrides, &["print-effective-config"]);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let config: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(config["passwordMinimumLength"], expected);
    }
    for invalid in ["0", "7", "129", "-1", "invalid"] {
        let output = run(
            &root,
            &[("CITADEL_PASSWORDS_MINIMUM_LENGTH", invalid)],
            &["print-effective-config"],
        );
        assert!(!output.status.success());
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("CITADEL_PASSWORDS_MINIMUM_LENGTH")
        );
    }
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn update_checks_default_to_enabled_and_allow_opt_out() {
    let root =
        std::env::temp_dir().join(format!("citadel-updates-config-{}", uuid::Uuid::now_v7()));
    for (overrides, expected) in [
        (vec![], true),
        (vec![("CITADEL_UPDATE_CHECKS_ENABLED", "true")], true),
        (vec![("CITADEL_UPDATE_CHECKS_ENABLED", "false")], false),
    ] {
        let result = run(&root, &overrides, &["print-effective-config"]);
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        let config: Value = serde_json::from_slice(&result.stdout).unwrap();
        assert_eq!(config["updatesEnabled"], expected);
    }
    assert!(
        !run(
            &root,
            &[("CITADEL_UPDATE_CHECKS_ENABLED", "invalid")],
            &["print-effective-config"]
        )
        .status
        .success()
    );
    std::fs::remove_dir_all(root).unwrap();
}

// The child reads production configuration with a deliberate environment. This
// keeps key-source, path and conditional-loading assertions out of global env.
#[test]
fn configuration_values_probe() {
    let Ok(expected_source) = std::env::var("CITADEL_TEST_DATABASE_SOURCE") else {
        return;
    };
    let config = citadel_server::config::Config::from_env().unwrap();
    if expected_source == "generated-keys" {
        use base64::Engine;
        assert!(!config.identity.jwt_key_is_external);
        assert!(!config.identity.encryption_key_is_external);
        let root = &config.execution.paths.data_root;
        let jwt = std::fs::read_to_string(root.join("jwtsecret")).unwrap();
        let encoded = std::fs::read_to_string(root.join("secret-encryption-key")).unwrap();
        let encryption = base64::engine::general_purpose::STANDARD
            .decode(encoded.trim())
            .unwrap();
        assert!(config.identity.jwt_key.expose() == jwt.trim().as_bytes());
        assert!(config.identity.secret_encryption_key.expose() == encryption);
        return;
    }

    #[cfg(unix)]
    if expected_source == "non-utf8-path" {
        use std::os::unix::ffi::OsStrExt;
        assert!(
            config.execution.backups.settings.allowed_core_paths[0]
                .as_os_str()
                .as_bytes()
                == b"/mnt/backup-\xff"
        );
        return;
    }
    assert_eq!(config.effective().unwrap().database_source, expected_source);
    assert!(config.identity.jwt_key_is_external);
    assert!(config.identity.encryption_key_is_external);
    assert!(config.identity.jwt_key.expose() == b"explicit-signing-key-with-at-least-32-bytes");
    assert!(config.identity.secret_encryption_key.expose() == (1..=32).collect::<Vec<u8>>());
    assert_eq!(
        config.execution.scripts.internal_base_url,
        "http://127.0.0.1:18443"
    );
    assert_eq!(config.listen_address.port(), 18443);
    assert_eq!(
        config.transport.cors_origins,
        ["http://two.example", "http://ten.example"]
    );
    assert_eq!(
        config.node_agent_policy.supported_architectures,
        ["arm64", "amd64"]
    );
    assert_eq!(
        config.node_agent_policy.supported_architectures,
        config
            .execution
            .edge_agent
            .setup_policy
            .supported_architectures
    );
    assert_eq!(
        config.execution.backups.settings.allowed_core_paths,
        [
            std::path::PathBuf::from("/mnt/two with spaces,commas"),
            std::path::PathBuf::from("/mnt/ten")
        ]
    );
    assert!(config.realtime.is_none());
    assert_eq!(
        config.execution.edge_agent.image,
        "registry.test/override@sha256:fixture"
    );
    if expected_source == "PG_*" {
        let url = url::Url::parse(&config.database_url).unwrap();
        assert!(url.username() == "user%40host");
        assert!(url.password() == Some("a%20b%23c"));
    }
}

#[test]
fn canonical_sources_lists_ports_and_conditional_loading_agree() {
    let root = std::env::temp_dir().join(format!("citadel-canonical-{}", uuid::Uuid::now_v7()));
    let values = [
        ("CITADEL_TRANSPORT_MODE", "Disabled"),
        ("CITADEL_HTTP_LISTEN_PORT", "18443"),
        ("CITADEL_HTTP_CORS_ORIGIN_10", "http://ten.example"),
        ("CITADEL_HTTP_CORS_ORIGIN_2", "http://two.example"),
        ("CITADEL_EDGE_AGENT_SUPPORTED_NODE_ARCHITECTURE_10", "amd64"),
        ("CITADEL_EDGE_AGENT_SUPPORTED_NODE_ARCHITECTURE_2", "arm64"),
        ("CITADEL_BACKUPS_ALLOWED_CORE_PATH_10", "/mnt/ten"),
        (
            "CITADEL_BACKUPS_ALLOWED_CORE_PATH_2",
            "/mnt/two with spaces,commas",
        ),
        (
            "CITADEL_JWT_KEY",
            "explicit-signing-key-with-at-least-32-bytes",
        ),
        (
            "CITADEL_SECRETS_ENCRYPTION_KEY",
            "AQIDBAUGBwgJCgsMDQ4PEBESExQVFhcYGRobHB0eHyA=",
        ),
        ("CITADEL_REALTIME_ENABLED", "false"),
        ("CITADEL_REALTIME_QUEUE_CAPACITY", "disabled-not-parsed"),
        (
            "CITADEL_EDGE_AGENT_IMAGE",
            "registry.test/override@sha256:fixture",
        ),
        (
            "CITADEL_EDGE_AGENT_IMAGE_REPOSITORY",
            "registry.test/repository",
        ),
        ("CITADEL_EDGE_AGENT_IMAGE_TAG", "v2.0.0"),
        ("PG_HOST", "localhost"),
        ("PG_PORT", "5433"),
        ("PG_DATABASE", "fixture"),
        ("PG_USER", "user@host"),
        ("PG_PASSWORD", "a b#c"),
    ];
    for source in ["CITADEL_DATABASE_URL", "PG_*"] {
        let mut command = Command::new(std::env::current_exe().unwrap());
        command
            .env_clear()
            .envs(values)
            .env("CITADEL_DATA_ROOT", &root)
            .env("CITADEL_TEST_DATABASE_SOURCE", source)
            .args(["--exact", "configuration_values_probe", "--nocapture"]);
        if source == "CITADEL_DATABASE_URL" {
            command.env(source, "postgres://uri:uri-secret@localhost/uri_fixture");
        }
        let output = command.output().unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            !root.exists(),
            "explicit keys must not create replacement files"
        );
    }
    for (key, value) in [
        ("CITADEL_DATABASE_URL", ""),
        ("CITADEL_DATABASE_URL", "not-a-uri"),
        ("CITADEL_DATABASE_URL", "https://database-secret@localhost"),
        ("CITADEL_HTTP_CORS_ORIGIN_2", " "),
        ("CITADEL_EDGE_AGENT_SUPPORTED_NODE_ARCHITECTURE_2", ""),
        ("CITADEL_BACKUPS_ALLOWED_CORE_PATH_2", ""),
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_citadel-server"))
            .env_clear()
            .envs(values)
            .env("CITADEL_DATA_ROOT", &root)
            .env(key, value)
            .arg("print-effective-config")
            .output()
            .unwrap();
        assert!(!output.status.success(), "invalid {key} accepted");
        assert!(!String::from_utf8_lossy(&output.stderr).contains("database-secret"));
    }
    assert!(!root.exists());
}

#[test]
fn healthcheck_uses_the_canonical_nondefault_listener() {
    use std::io::{Read, Write};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port().to_string();
    let server = std::thread::spawn(move || {
        listener.set_nonblocking(true).unwrap();
        let start = std::time::Instant::now();
        let (mut socket, _) = loop {
            if let Ok(connection) = listener.accept() {
                break connection;
            }
            assert!(
                start.elapsed() < std::time::Duration::from_secs(10),
                "healthcheck did not reach configured port"
            );
            std::thread::sleep(std::time::Duration::from_millis(10));
        };
        socket
            .set_read_timeout(Some(std::time::Duration::from_secs(2)))
            .unwrap();
        let mut request = [0; 1024];
        let len = socket.read(&mut request).unwrap();
        assert!(request[..len].starts_with(b"GET /health "));
        socket
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
            .unwrap();
    });
    let output = Command::new(env!("CARGO_BIN_EXE_citadel-server"))
        .env_clear()
        .env("CITADEL_TRANSPORT_MODE", "Disabled")
        .env("CITADEL_HTTP_LISTEN_PORT", port)
        .arg("healthcheck")
        .output()
        .unwrap();
    server.join().unwrap();
    assert!(output.status.success());
}

#[cfg(unix)]
#[test]
fn non_utf8_paths_survive_unrelated_cors_enumeration() {
    use std::os::unix::ffi::OsStringExt;
    let root = std::env::temp_dir().join(format!("citadel-path-{}", uuid::Uuid::now_v7()));
    let output = Command::new(std::env::current_exe().unwrap())
        .env_clear()
        .env("CITADEL_TEST_DATABASE_SOURCE", "non-utf8-path")
        .env(
            "CITADEL_DATABASE_URL",
            "postgres://fixture:fixture@localhost/fixture",
        )
        .env("CITADEL_TRANSPORT_MODE", "Disabled")
        .env("CITADEL_DATA_ROOT", &root)
        .env(
            "CITADEL_JWT_KEY",
            "explicit-signing-key-with-at-least-32-bytes",
        )
        .env(
            "CITADEL_SECRETS_ENCRYPTION_KEY",
            "AQIDBAUGBwgJCgsMDQ4PEBESExQVFhcYGRobHB0eHyA=",
        )
        .env("CITADEL_HTTP_CORS_ORIGIN_2", "http://two.example")
        .env(
            "CITADEL_BACKUPS_ALLOWED_CORE_PATH_2",
            std::ffi::OsString::from_vec(b"/mnt/backup-\xff".to_vec()),
        )
        .args(["--exact", "configuration_values_probe", "--nocapture"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!root.exists());
}
