use super::super::{InstallationLock, JournalDirectory, RegisteredComposeInstallation};
use super::*;
use crate::external::backups::system_recovery::{PgDumpSystemBackupBuilder, RecoveryAssets};
use citadel_updates::*;
use sqlx::postgres::PgPoolOptions;
use std::{fs, os::unix::fs::PermissionsExt, path::PathBuf, time::Duration};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

pub(super) struct Fixture {
    root: tempfile::TempDir,
    pub(super) state: JournalDirectory,
    pub(super) owner: InstallationLock,
    pub(super) operation: UpdateOperation,
}
impl Fixture {
    pub(super) fn new(instance: Uuid) -> Self {
        let root = tempfile::tempdir().unwrap();
        fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let state_path = root.path().join("state");
        fs::create_dir(&state_path).unwrap();
        fs::set_permissions(&state_path, fs::Permissions::from_mode(0o700)).unwrap();
        let inputs = root.path().join("installer");
        fs::create_dir(&inputs).unwrap();
        let compose = inputs.join("custom.yml");
        let edge = inputs.join("edge.yml");
        fs::write(
            &compose,
            b"services: {server: {image: 'ghcr.io/citadel-p/citadel:0.1.1'}}\n",
        )
        .unwrap();
        fs::write(&edge, b"services: {server: {ports: ['18001:18001']}}\n").unwrap();
        fs::write(
            inputs.join(".env"),
            b"PG_PASSWORD=private-checkpoint-fixture\n",
        )
        .unwrap();
        fs::set_permissions(inputs.join(".env"), fs::Permissions::from_mode(0o600)).unwrap();
        let state = JournalDirectory::open(state_path.clone()).unwrap();
        let owner = state.lock().unwrap();
        let mut registration = RegisteredComposeInstallation {
            format: INSTALLER_FORMAT,
            kind: InstallationKind::DockerCompose,
            id: Uuid::now_v7(),
            instance_id: instance,
            directory: inputs.clone(),
            project: "test-checkpoint".into(),
            compose_files: vec![edge, compose],
            environment_file: inputs.join(".env"),
            docker_socket: "/var/run/docker.sock".into(),
            daemon_id: "fixture".into(),
            core_container_id: "a".repeat(64),
            database_container_id: "b".repeat(64),
            core_volume: "test_core".into(),
            database_volume: "test_database".into(),
            network: "test_network".into(),
            state_directory: state_path.clone(),
            executor_uid: rustix::process::geteuid().as_raw(),
            executor_gid: rustix::process::getegid().as_raw(),
            configuration_sha256: "a".repeat(64),
            runtime_sha256: "b".repeat(64),
        };
        registration.configuration_sha256 =
            registration.fingerprint(std::path::Path::new("/")).unwrap();
        state.create("installation.json", &registration).unwrap();
        let source = CoreIdentity {
            version: "0.1.1".into(),
            image: format!("ghcr.io/citadel-p/citadel@sha256:{}", "a".repeat(64)),
            catalog_sha256: "b".repeat(64),
            architecture: "amd64".into(),
        };
        let now = chrono::Utc::now();
        let mut operation = UpdateOperation::approve(
            UpdatePlan {
                id: Uuid::now_v7(),
                instance_id: instance,
                target: CoreIdentity {
                    version: "0.1.2".into(),
                    image: format!("ghcr.io/citadel-p/citadel@sha256:{}", "c".repeat(64)),
                    ..source.clone()
                },
                helper_image: source.image.clone(),
                source,
                configuration_sha256: registration.configuration_sha256.clone(),
                registration_id: registration.id,
                reviewed_at: now,
                expires_at: now + chrono::Duration::minutes(10),
                checkpoint_directory: state_path.to_str().unwrap().into(),
                agent_requirements: vec![],
            },
            citadel_primitives::ActorId::new(Uuid::now_v7()),
            Uuid::now_v7(),
            now,
        )
        .unwrap();
        let mut history = vec![operation.clone()];
        operation.advance(0, UpdatePhase::Draining, now).unwrap();
        history.push(operation.clone());
        operation.advance(1, UpdatePhase::BackingUp, now).unwrap();
        history.push(operation.clone());
        state
            .save(&UpdateJournal {
                protocol: UPDATE_PROTOCOL,
                operation: operation.clone(),
                helper_container_id: "d".repeat(64),
                history,
                verified_target: None,
                completion_acknowledged: false,
                maintenance_released: false,
            })
            .unwrap();
        let keys = root.path().join("keys");
        fs::create_dir_all(keys.join("agent")).unwrap();
        for name in ["jwtsecret", "secret-encryption-key", "agent/signing-key"] {
            fs::write(keys.join(name), b"private-fixture-key").unwrap();
        }
        Self {
            root,
            state,
            owner,
            operation,
        }
    }
    fn builder(
        &self,
        pool: sqlx::PgPool,
        url: &str,
        executable: std::ffi::OsString,
    ) -> PgDumpSystemBackupBuilder {
        PgDumpSystemBackupBuilder::new(
            pool,
            url.into(),
            executable,
            self.root.path().join("unused"),
            4096,
            "0.1.1".into(),
        )
        .with_recovery_assets(RecoveryAssets {
            core_data_path: self.root.path().join("keys"),
            jwt_key_is_external: false,
            encryption_key_is_external: false,
        })
    }
    pub(super) fn stage(&self) -> PathBuf {
        self.state
            .path()
            .join(format!("checkpoint-{}", self.operation.id))
    }
}

#[tokio::test]
async fn refuses_cancelled_stale_inputs_foreign_lock_and_partial_attempts() {
    for case in 0..5 {
        let f = Fixture::new(Uuid::now_v7());
        let token = CancellationToken::new();
        let other = Fixture::new(Uuid::now_v7());
        let owner = if case == 2 { &other.owner } else { &f.owner };
        match case {
            0 => token.cancel(),
            1 => fs::write(
                f.root.path().join("installer/.env"),
                "PG_PASSWORD=changed\n",
            )
            .unwrap(),
            3 => {
                fs::create_dir(f.stage()).unwrap();
                fs::write(f.stage().join("retained"), "crash evidence").unwrap();
            }
            4 => {
                let mut journal = f.state.journal(f.operation.id).unwrap();
                journal.operation.fence = Uuid::now_v7();
                f.state.save(&journal).unwrap();
            }
            _ => (),
        }
        let pool = PgPoolOptions::new()
            .connect_lazy("postgres://unused:unused@127.0.0.1:1/unused")
            .unwrap();
        let builder = f.builder(
            pool,
            "postgres://unused:unused@127.0.0.1:1/unused",
            "missing-pg-dump".into(),
        );
        let error = prepare_update_checkpoint(&f.state, owner, &f.operation, &builder, &token)
            .await
            .unwrap_err();
        assert!(!error.to_string().contains("private-checkpoint-fixture"));
        assert!(!f.stage().join("checkpoint.json").exists());
        if case == 3 {
            assert_eq!(
                fs::read_to_string(f.stage().join("retained")).unwrap(),
                "crash evidence"
            );
        } else {
            assert!(!f.stage().exists());
        }
        assert!(f.state.lock().is_err(), "owner remains held");
    }
}

#[tokio::test]
async fn disk_write_failure_and_unsafe_bundle_never_publish_success() {
    use super::storage::*;
    let f = Fixture::new(Uuid::now_v7());
    let stage = create_attempt(&f.state, "checkpoint-storage-test").unwrap();
    // A real ENOSPC device exercises the write path without exhausting host disk.
    let full = fs::OpenOptions::new()
        .write(true)
        .open("/dev/full")
        .unwrap();
    assert!(sync_input(full, b"private-checkpoint-fixture").is_err());
    std::os::unix::fs::symlink("/dev/full", stage.path().join("input-0")).unwrap();
    assert!(write_input(&stage, "input-0", b"secret").is_err());
    let bundle = stage.path().join("bundle");
    fs::create_dir(&bundle).unwrap();
    std::os::unix::fs::symlink("/etc/passwd", bundle.join("manifest.json")).unwrap();
    assert!(
        sync_bundle(
            &bundle,
            &["manifest.json".into()],
            &CancellationToken::new()
        )
        .await
        .is_err()
    );
    assert!(!stage.path().join("checkpoint.json").exists());
}

#[tokio::test]
#[ignore = "requires fresh disposable CITADEL_UPDATE_TEST_DATABASE_URL and CITADEL_CHECKPOINT_TARGET_DATABASE_URL named citadel_update_test_*, plus pg_dump/pg_restore"]
async fn checkpoint_builds_real_bundle_and_preserves_failed_or_interrupted_attempts() {
    let url = std::env::var("CITADEL_UPDATE_TEST_DATABASE_URL").unwrap();
    assert!(
        url::Url::parse(&url)
            .unwrap()
            .path()
            .starts_with("/citadel_update_test_")
    );
    let pool = PgPoolOptions::new()
        .max_connections(2)
        .connect(&url)
        .await
        .unwrap();
    assert!(
        sqlx::query_scalar::<_, bool>(
            "SELECT to_regclass('public.citadelinstanceidentity') IS NULL"
        )
        .fetch_one(&pool)
        .await
        .unwrap()
    );
    sqlx::raw_sql("CREATE TABLE citadelinstanceidentity (id int PRIMARY KEY, instanceid uuid NOT NULL, createdat timestamptz DEFAULT now()); CREATE TABLE checkpoint_marker(value text); INSERT INTO checkpoint_marker VALUES ('preserved');").execute(&pool).await.unwrap();
    let instance = Uuid::now_v7();
    sqlx::query("INSERT INTO citadelinstanceidentity(id,instanceid) VALUES(1,$1)")
        .bind(instance)
        .execute(&pool)
        .await
        .unwrap();
    let f = Fixture::new(instance);
    let builder = f.builder(
        pool.clone(),
        &url,
        std::env::var_os("CITADEL_PG_DUMP").unwrap(),
    );
    let token = CancellationToken::new();
    let before = f.state.journal(f.operation.id).unwrap();
    let result = prepare_update_checkpoint(&f.state, &f.owner, &f.operation, &builder, &token)
        .await
        .unwrap();
    assert_eq!(f.state.journal(f.operation.id).unwrap(), before);
    assert_eq!(result.instance_id, instance);
    assert_eq!(result.installer_inputs.len(), 3);
    assert_eq!(
        result.installer_inputs[0]
            .original_path
            .file_name()
            .unwrap(),
        "edge.yml"
    );
    for input in &result.installer_inputs {
        assert_eq!(
            fs::read(f.stage().join(&input.saved_name)).unwrap(),
            fs::read(&input.original_path).unwrap()
        );
        assert_eq!(
            fs::metadata(f.stage().join(&input.saved_name))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
    }
    let receipt: PreparedUpdateCheckpoint = JournalDirectory::open(f.stage())
        .unwrap()
        .read("checkpoint.json")
        .unwrap();
    assert_eq!(receipt, result);
    assert!(!format!("{result:?}").contains("private-checkpoint-fixture"));
    assert_eq!(
        fs::read(result.bundle.join("recovery/agent/signing-key")).unwrap(),
        b"private-fixture-key"
    );
    assert!(
        prepare_update_checkpoint(&f.state, &f.owner, &f.operation, &builder, &token)
            .await
            .is_err()
    );
    // Validate the actual archive through pg_restore's read-only table of contents.
    let restore = std::env::var_os("CITADEL_PG_RESTORE").unwrap();
    let output = std::process::Command::new(restore)
        .arg("--list")
        .arg(result.bundle.join("database/citadel.dump"))
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("checkpoint_marker"));

    let target_url = std::env::var("CITADEL_CHECKPOINT_TARGET_DATABASE_URL").unwrap();
    let target_address = url::Url::parse(&target_url).unwrap();
    assert!(target_address.path().starts_with("/citadel_update_test_"));
    assert_ne!(target_address.path(), url::Url::parse(&url).unwrap().path());
    let target = PgPoolOptions::new()
        .max_connections(1)
        .connect(&target_url)
        .await
        .unwrap();
    assert!(
        sqlx::query_scalar::<_, bool>(
            "SELECT to_regclass('public.citadelinstanceidentity') IS NULL"
        )
        .fetch_one(&target)
        .await
        .unwrap()
    );
    use crate::external::backups::system_recovery::{
        CitadelSystemRestoreOptions, restore_citadel_system,
    };
    restore_citadel_system(
        &CitadelSystemRestoreOptions {
            bundle: result.bundle.clone(),
            database_url: target_url,
            pg_restore: std::env::var_os("CITADEL_PG_RESTORE").unwrap(),
            maximum_output: 4096,
            secret_encryption_key: Some(zeroize::Zeroizing::new(vec![7; 32])),
        },
        &token,
    )
    .await
    .unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, String>("SELECT value FROM checkpoint_marker")
            .fetch_one(&target)
            .await
            .unwrap(),
        "preserved"
    );
    assert_eq!(
        sqlx::query_scalar::<_, Uuid>("SELECT instanceid FROM citadelinstanceidentity WHERE id=1")
            .fetch_one(&target)
            .await
            .unwrap(),
        instance
    );
    target.close().await;

    for case in 0..5 {
        let f = Fixture::new(if case == 3 { Uuid::now_v7() } else { instance });
        let script = f.root.path().join("dump-fixture");
        let body = match case {
            0 => "echo private-checkpoint-fixture >&2; exit 1",
            1 => "echo started > started; exec /bin/sleep 30",
            2 => "echo PG_PASSWORD=changed >> ../../installer/.env; printf PGDMPfixture > \"$5\"",
            3 => "exit 99",
            _ => "printf invalid > \"$5\"",
        };
        // Script runs from state/checkpoint-ID/ID. Use an absolute fixture path
        // for controlled drift; no production or personal paths are involved.
        let body = body.replace(
            "../../installer/.env",
            f.root.path().join("installer/.env").to_str().unwrap(),
        );
        fs::write(&script, format!("#!/bin/sh\n{body}\n")).unwrap();
        fs::set_permissions(&script, fs::Permissions::from_mode(0o700)).unwrap();
        let builder = f.builder(pool.clone(), &url, script.into_os_string());
        let token = CancellationToken::new();
        let cancel = async {
            if case == 1 {
                let started = f
                    .stage()
                    .join(f.operation.id.simple().to_string())
                    .join("started");
                tokio::time::timeout(Duration::from_secs(10), async {
                    while !started.exists() {
                        tokio::time::sleep(Duration::from_millis(10)).await;
                    }
                })
                .await
                .unwrap();
                token.cancel();
            }
        };
        let (result, ()) = tokio::join!(
            prepare_update_checkpoint(&f.state, &f.owner, &f.operation, &builder, &token),
            cancel
        );
        let error = result.unwrap_err();
        assert!(!error.to_string().contains("private-checkpoint-fixture"));
        assert!(f.stage().join("operation.json").exists());
        assert!(!f.stage().join("checkpoint.json").exists());
        if case == 1 {
            assert!(matches!(error, UpdateError::Cancelled));
        }
        assert!(f.state.lock().is_err());
    }
    assert_eq!(
        sqlx::query_scalar::<_, Uuid>("SELECT instanceid FROM citadelinstanceidentity WHERE id=1")
            .fetch_one(&pool)
            .await
            .unwrap(),
        instance
    );
    pool.close().await;
}
