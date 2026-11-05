use super::super::docker::SOCKET;
use super::super::{JournalDirectory, RegisteredComposeInstallation};
use super::*;
use super::{executor::validate_executor, registration::REGISTRATION};
use citadel_updates::UpdateError;
use serde_json::json;
use std::sync::atomic::Ordering;
use std::{fs, os::unix::fs::PermissionsExt, path::PathBuf};
use tokio_util::sync::CancellationToken;

use super::fixtures::*;

#[tokio::test]
async fn startup_observes_registration_without_repairing_or_holding_the_helper_lock() {
    use crate::external::updates::check_startup_registration;
    let fixture = Fixture::new();
    // Explicitly configured empty/missing state must not become a new registration.
    assert!(check_startup_registration(&fixture.state).is_err());
    assert_eq!(fs::read_dir(&fixture.state).unwrap().count(), 0);
    assert!(check_startup_registration(&fixture.state.join("missing")).is_err());
    let registration = fixture.register().await.unwrap();
    assert_eq!(
        check_startup_registration(&fixture.state).unwrap(),
        registration
    );
    let state = JournalDirectory::open(fixture.state.clone()).unwrap();
    let lock = state.lock().unwrap();
    assert!(check_startup_registration(&fixture.state).is_err());
    drop(lock);
    assert_eq!(
        check_startup_registration(&fixture.state).unwrap(),
        registration
    );
    // Startup does not retain ownership while waiting for any future helper.
    drop(state.lock().unwrap());
    fs::remove_file(fixture.state.join("installation.lock")).unwrap();
    assert!(check_startup_registration(&fixture.state).is_err());
    assert!(!fixture.state.join("installation.lock").exists());
}

#[tokio::test]
async fn startup_never_trusts_completed_or_orphaned_execution_evidence() {
    use crate::external::updates::check_startup_registration;
    for name in [
        "maintenance.json",
        "01900000-0000-7000-8000-000000000002.json",
        ".interrupted.tmp",
    ] {
        let fixture = Fixture::new();
        fixture.register().await.unwrap();
        let evidence =
            br#"{"phase":"Succeeded","maintenanceReleased":true,"completionAcknowledged":true}"#;
        fs::write(fixture.state.join(name), evidence).unwrap();
        assert!(
            check_startup_registration(&fixture.state).is_err(),
            "{name}"
        );
        assert_eq!(fs::read(fixture.state.join(name)).unwrap(), evidence);
    }
}

#[tokio::test]
async fn startup_rejects_corrupt_foreign_or_unprotected_registration() {
    use crate::external::updates::check_startup_registration;
    for mutation in 0..6 {
        let fixture = Fixture::new();
        let mut registration = fixture.register().await.unwrap();
        let state = JournalDirectory::open(fixture.state.clone()).unwrap();
        match mutation {
            0 => fs::write(fixture.state.join(REGISTRATION), b"corrupt").unwrap(),
            1 => {
                registration.state_directory = PathBuf::from("/another-state-directory");
                state.write(REGISTRATION, &registration).unwrap();
            }
            2 => {
                registration.executor_uid = registration.executor_uid.wrapping_add(1);
                state.write(REGISTRATION, &registration).unwrap();
            }
            3 => fs::set_permissions(
                fixture.state.join(REGISTRATION),
                fs::Permissions::from_mode(0o644),
            )
            .unwrap(),
            4 => {
                fs::rename(
                    fixture.state.join(REGISTRATION),
                    fixture.state.join("original.json"),
                )
                .unwrap();
                std::os::unix::fs::symlink("original.json", fixture.state.join(REGISTRATION))
                    .unwrap();
            }
            _ => fs::set_permissions(&fixture.state, fs::Permissions::from_mode(0o755)).unwrap(),
        }
        let before = fs::read(fixture.state.join(REGISTRATION)).unwrap();
        assert!(
            check_startup_registration(&fixture.state).is_err(),
            "{mutation}"
        );
        assert_eq!(fs::read(fixture.state.join(REGISTRATION)).unwrap(), before);
    }
}

#[test]
fn containerized_registration_requires_its_own_identity_and_exact_persistent_mounts() {
    let fixture = Fixture::new();
    let executor = json!({"Id": CORE, "Config":{"Hostname": &CORE[..12]}, "Mounts":[
        {"Type":"bind","Source":fixture.inputs.directory,"Destination":fixture.inputs.directory,"RW":false},
        {"Type":"bind","Source":fixture.state,"Destination":fixture.state,"RW":true},
        {"Type":"bind","Source":SOCKET,"Destination":SOCKET,"RW":true}
    ]});
    validate_executor(&executor, &CORE[..12], &fixture.inputs, &fixture.state).unwrap();
    for mutation in 0..6 {
        let mut changed = executor.clone();
        match mutation {
            0 => changed["Id"] = json!(DATABASE),
            1 => changed["Mounts"][0]["RW"] = json!(true),
            2 => {
                changed["Mounts"].as_array_mut().unwrap().remove(1);
            }
            3 => changed["Mounts"][1]["Source"] = json!("/different-host-directory"),
            4 => changed["Mounts"][1]["Type"] = json!("volume"),
            _ => changed["Mounts"]
                .as_array_mut()
                .unwrap()
                .push(json!({"Type":"bind","Source":"/","Destination":"/host","RW":true})),
        }
        assert!(validate_executor(&changed, &CORE[..12], &fixture.inputs, &fixture.state).is_err());
    }
}

#[tokio::test]
async fn registration_is_private_durable_idempotent_and_does_not_store_secrets() {
    let fixture = Fixture::new();
    let first = fixture.register().await.unwrap();
    let path = fixture.state.join(REGISTRATION);
    let bytes = fs::read(&path).unwrap();
    assert!(!String::from_utf8_lossy(&bytes).contains("private-test-password"));
    assert_eq!(
        fs::metadata(&path).unwrap().permissions().mode() & 0o777,
        0o600
    );
    assert_eq!(first.instance_id.to_string(), INSTANCE);
    assert_eq!(first.core_container_id, CORE);
    assert_eq!(first.database_container_id, DATABASE);
    assert_eq!(fixture.register().await.unwrap(), first);
    assert_eq!(fs::read(&path).unwrap(), bytes);
    assert!(
        fixture
            .runner
            .homes
            .lock()
            .unwrap()
            .iter()
            .all(|path| !path.exists())
    );
}

#[tokio::test]
async fn unsafe_ambiguous_and_changing_installations_never_write_registration() {
    for mode in [
        "replicas",
        "rootless",
        "shared-volume",
        "shared-network",
        "changed-daemon",
        "changed-inputs",
        "missing-identity",
        "cancel",
        "tool-failure",
    ] {
        let mut fixture = Fixture::new();
        fixture.runner.mode = mode;
        let error = fixture.register().await.unwrap_err();
        assert!(!error.to_string().contains("private-test-password"));
        assert!(!fixture.state.join(REGISTRATION).exists(), "{mode}");
        JournalDirectory::open(fixture.state.clone())
            .unwrap()
            .lock()
            .unwrap();
        assert!(
            fixture
                .runner
                .homes
                .lock()
                .unwrap()
                .iter()
                .all(|path| !path.exists())
        );
    }
}

#[tokio::test]
async fn different_existing_registration_and_orphaned_evidence_are_preserved() {
    let mut fixture = Fixture::new();
    fixture.register().await.unwrap();
    let original = fs::read(fixture.state.join(REGISTRATION)).unwrap();
    fixture.runner.core["Image"] = json!(format!("sha256:{}", "e".repeat(64)));
    assert!(matches!(
        fixture.register().await,
        Err(UpdateError::Conflict(_))
    ));
    assert_eq!(
        fs::read(fixture.state.join(REGISTRATION)).unwrap(),
        original
    );
    let fixture = Fixture::new();
    JournalDirectory::open(fixture.state.clone())
        .unwrap()
        .write("recovery.json", &json!({"phase":"RecoveryRequired"}))
        .unwrap();
    assert!(fixture.register().await.is_err());
    assert_eq!(fixture.runner.calls.load(Ordering::SeqCst), 0);
    assert!(fixture.state.join("recovery.json").exists());
}

#[tokio::test]
async fn lock_contention_and_corrupt_registration_fail_before_docker() {
    let fixture = Fixture::new();
    let state = JournalDirectory::open(fixture.state.clone()).unwrap();
    let lock = state.lock().unwrap();
    assert!(matches!(
        fixture.register().await,
        Err(UpdateError::Conflict(_))
    ));
    drop(lock);
    state.write(REGISTRATION, &"not a registration").unwrap();
    assert!(fixture.register().await.is_err());
    assert_eq!(fixture.runner.calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn environment_ports_mounts_and_service_identity_must_match_the_invocation() {
    for mutation in 0..6 {
        let mut fixture = Fixture::new();
        match mutation {
            0 => fixture.runner.core["Config"]["Env"][0] = json!("PG_HOST=external"),
            1 => {
                fixture.runner.core["HostConfig"]["PortBindings"]["8000/tcp"][0]["HostPort"] =
                    json!("9999")
            }
            2 => fixture.runner.core["Mounts"][0]["Name"] = json!("wrong-volume"),
            3 => {
                fixture.runner.core["Config"]["Labels"]["com.docker.compose.project.working_dir"] =
                    json!("/wrong")
            }
            4 => {
                fixture.runner.core["NetworkSettings"]["Networks"]["citadel-test_default"]["NetworkID"] =
                    json!("wrong")
            }
            _ => fixture.runner.core["State"]["Health"]["Status"] = json!("unhealthy"),
        }
        assert!(fixture.register().await.is_err(), "{mutation}");
        assert!(!fixture.state.join(REGISTRATION).exists());
    }
}

#[tokio::test]
async fn unknown_installation_kinds_are_preserved_and_never_sent_to_docker() {
    let fixture = Fixture::new();
    let registration = fixture.register().await.unwrap();
    let mut value = serde_json::to_value(registration).unwrap();
    value["kind"] = json!("Kubernetes");
    let state = JournalDirectory::open(fixture.state.clone()).unwrap();
    state.write(REGISTRATION, &value).unwrap();
    let before = fs::read(fixture.state.join(REGISTRATION)).unwrap();
    let calls = fixture.runner.calls.load(Ordering::SeqCst);
    assert!(fixture.register().await.is_err());
    assert_eq!(fixture.runner.calls.load(Ordering::SeqCst), calls);
    assert_eq!(fs::read(fixture.state.join(REGISTRATION)).unwrap(), before);
}

#[tokio::test]
async fn live_check_preserves_registration_and_releases_the_lock() {
    let fixture = Fixture::new();
    let registered = fixture.register().await.unwrap();
    let path = fixture.state.join(REGISTRATION);
    let bytes = fs::read(&path).unwrap();
    let modified = fs::metadata(&path).unwrap().modified().unwrap();
    assert_eq!(fixture.check().await.unwrap().registration, registered);
    assert_eq!(fs::read(&path).unwrap(), bytes);
    assert_eq!(fs::metadata(&path).unwrap().modified().unwrap(), modified);
    assert_eq!(fs::read_dir(&fixture.state).unwrap().count(), 2);
    let state = JournalDirectory::open(fixture.state.clone()).unwrap();
    let lock = state.existing_lock().unwrap();
    let calls = fixture.runner.calls.load(Ordering::SeqCst);
    assert!(fixture.check().await.is_err());
    assert_eq!(fixture.runner.calls.load(Ordering::SeqCst), calls);
    drop(lock);
    assert!(fixture.check().await.is_ok());
}

#[tokio::test]
async fn live_check_never_repairs_missing_state() {
    let fixture = Fixture::new();
    assert!(fixture.check().await.is_err());
    assert_eq!(fs::read_dir(&fixture.state).unwrap().count(), 0);
    fixture.register().await.unwrap();
    fs::remove_file(fixture.state.join("installation.lock")).unwrap();
    let calls = fixture.runner.calls.load(Ordering::SeqCst);
    assert!(fixture.check().await.is_err());
    assert!(!fixture.state.join("installation.lock").exists());
    assert_eq!(fixture.runner.calls.load(Ordering::SeqCst), calls);
}

#[tokio::test]
async fn live_check_rejects_unsafe_state_and_changed_inputs_before_external_work() {
    for mutation in 0..9 {
        let fixture = Fixture::new();
        let mut registration = fixture.register().await.unwrap();
        let state = JournalDirectory::open(fixture.state.clone()).unwrap();
        match mutation {
            0 => fs::write(fixture.state.join("recovery.json"), b"preserve recovery").unwrap(),
            1 => fs::write(
                fixture.inputs.directory.join(".env"),
                "PG_PASSWORD=changed\n",
            )
            .unwrap(),
            2 => registration.executor_uid = registration.executor_uid.wrapping_add(1),
            3 => registration.executor_gid = registration.executor_gid.wrapping_add(1),
            4 => registration.state_directory = "/different-state".into(),
            5 => registration.docker_socket = "/different.sock".into(),
            6 => {
                let mut value = serde_json::to_value(&registration).unwrap();
                value["kind"] = json!("Kubernetes");
                state.write(REGISTRATION, &value).unwrap();
            }
            7 => fs::write(fixture.state.join(REGISTRATION), b"corrupt").unwrap(),
            _ => fs::set_permissions(
                fixture.state.join(REGISTRATION),
                fs::Permissions::from_mode(0o644),
            )
            .unwrap(),
        }
        if (2..=5).contains(&mutation) {
            state.write(REGISTRATION, &registration).unwrap();
        }
        let before = fs::read(fixture.state.join(REGISTRATION)).unwrap();
        let calls = fixture.runner.calls.load(Ordering::SeqCst);
        assert!(fixture.check().await.is_err(), "{mutation}");
        assert_eq!(fixture.runner.calls.load(Ordering::SeqCst), calls);
        assert_eq!(fs::read(fixture.state.join(REGISTRATION)).unwrap(), before);
        if mutation == 0 {
            assert_eq!(
                fs::read(fixture.state.join("recovery.json")).unwrap(),
                b"preserve recovery"
            );
        }
    }
}

#[tokio::test]
async fn live_check_accepts_external_keys_only_from_matching_installer_environment() {
    let mut fixture = Fixture::new();
    for setting in ["CITADEL_JWT_KEY", "CITADEL_SECRETS_ENCRYPTION_KEY"] {
        fixture.runner.config["services"]["server"]["environment"][setting] =
            json!("external-key-fixture");
        fixture.runner.core["Config"]["Env"]
            .as_array_mut()
            .unwrap()
            .push(json!(format!("{setting}=external-key-fixture")));
    }
    fixture.register().await.unwrap();
    let checked = fixture.check().await.unwrap();
    assert_eq!(
        checked.recovery_keys.jwt,
        super::super::RecoveryKeySource::Environment
    );
    assert_eq!(
        checked.recovery_keys.secrets_encryption,
        super::super::RecoveryKeySource::Environment
    );
    assert!(
        !serde_json::to_string(&checked)
            .unwrap()
            .contains("external-key-fixture")
    );
    fixture.runner.core["Config"]["Env"]
        .as_array_mut()
        .unwrap()
        .pop();
    assert!(fixture.check().await.is_err());
}

#[tokio::test]
async fn live_check_rejects_drift_and_ambiguity_without_rewriting_registration() {
    for mode in [
        "replicas",
        "rootless",
        "shared-volume",
        "shared-network",
        "changed-daemon",
        "changed-inputs",
        "different-instance",
        "cancel",
        "tool-failure",
        "changed-image",
        "evidence-during-check",
        "invalid-catalog",
        "recovery-key-failure",
        "unregistered-key",
        "invalid-key-report",
        "external-agent-key",
        "key-cancel",
    ] {
        let mut fixture = Fixture::new();
        fixture.register().await.unwrap();
        let before = fs::read(fixture.state.join(REGISTRATION)).unwrap();
        fixture.runner.mode = mode;
        if mode == "changed-image" {
            fixture.runner.core["Image"] = json!(format!("sha256:{}", "e".repeat(64)));
        }
        let error = fixture.check().await.unwrap_err();
        if mode == "cancel" || mode == "key-cancel" {
            assert!(matches!(error, UpdateError::Cancelled));
        }
        assert!(!error.to_string().contains("private-test-password"));
        assert!(!error.to_string().contains("private key detail"));
        assert_eq!(fs::read(fixture.state.join(REGISTRATION)).unwrap(), before);
        JournalDirectory::open(fixture.state.clone())
            .unwrap()
            .existing_lock()
            .unwrap();
        assert!(
            fixture
                .runner
                .homes
                .lock()
                .unwrap()
                .iter()
                .all(|p| !p.exists())
        );
        if mode == "evidence-during-check" {
            assert_eq!(
                fs::read(fixture.state.join("recovery.json")).unwrap(),
                b"preserve recovery"
            );
        }
    }
}

#[tokio::test]
async fn live_check_rejects_registration_replacement_and_initial_cancellation() {
    let mut fixture = Fixture::new();
    fixture.register().await.unwrap();
    fixture.runner.mode = "registration-during-check";
    assert!(fixture.check().await.is_err());
    let state = JournalDirectory::open(fixture.state.clone()).unwrap();
    let changed: RegisteredComposeInstallation = state.read(REGISTRATION).unwrap();
    assert_ne!(changed.instance_id.to_string(), INSTANCE);
    let calls = fixture.runner.calls.load(Ordering::SeqCst);
    let cancellation = CancellationToken::new();
    cancellation.cancel();
    assert!(matches!(
        check_registered_compose_installation(&fixture.state, &fixture.runner, &cancellation).await,
        Err(UpdateError::Cancelled)
    ));
    assert_eq!(fixture.runner.calls.load(Ordering::SeqCst), calls);
}

#[test]
fn checkpoint_budget_is_bounded_and_invalid_sizes_are_refused() {
    assert_eq!(
        super::checkpoint::minimum_bytes(1024).unwrap(),
        64 * 1024 * 1024 + 2048
    );
    assert!(super::checkpoint::minimum_bytes(0).is_err());
    assert!(super::checkpoint::minimum_bytes(u64::MAX).is_err());
}

#[test]
fn checkpoint_storage_probe_cleans_up_and_preserves_existing_evidence() {
    let fixture = Fixture::new();
    let state = JournalDirectory::open(fixture.state.clone()).unwrap();
    state
        .write("preserved.json", &json!({"evidence":true}))
        .unwrap();
    let before = fs::read(fixture.state.join("preserved.json")).unwrap();
    let checked = super::checkpoint::check_storage(&state, 1024).unwrap();
    assert_eq!(checked.directory, fixture.state);
    assert!(checked.available_bytes >= checked.minimum_bytes);
    assert_eq!(fs::read_dir(&fixture.state).unwrap().count(), 1);
    assert_eq!(
        fs::read(fixture.state.join("preserved.json")).unwrap(),
        before
    );
    assert!(super::checkpoint::check_storage(&state, u64::MAX / 4).is_err());
    assert_eq!(fs::read_dir(&fixture.state).unwrap().count(), 1);
}

#[test]
fn checkpoint_storage_rejects_a_replaced_directory_before_writing() {
    let fixture = Fixture::new();
    let state = JournalDirectory::open(fixture.state.clone()).unwrap();
    let original = fixture.state.with_file_name("original-state");
    fs::rename(&fixture.state, &original).unwrap();
    fs::create_dir(&fixture.state).unwrap();
    fs::set_permissions(&fixture.state, fs::Permissions::from_mode(0o700)).unwrap();
    assert!(super::checkpoint::check_storage(&state, 1024).is_err());
    assert_eq!(fs::read_dir(&original).unwrap().count(), 0);
    assert_eq!(fs::read_dir(&fixture.state).unwrap().count(), 0);
}

#[tokio::test]
async fn explicit_refresh_updates_idle_input_fingerprint_but_preserves_identity_and_evidence() {
    let mut fixture = Fixture::new();
    let original = fixture.register().await.unwrap();
    use std::io::Write;
    std::fs::OpenOptions::new()
        .append(true)
        .open(&original.environment_file)
        .unwrap()
        .write_all(b"\n# intentional operator change\n")
        .unwrap();
    assert!(fixture.register().await.is_err());
    assert!(fixture.check().await.is_err());
    let updated = refresh_compose_installation(
        &fixture.inputs,
        fixture.state.clone(),
        &fixture.runner,
        &CancellationToken::new(),
    )
    .await
    .unwrap();
    assert_eq!(updated.id, original.id);
    assert_eq!(updated.instance_id, original.instance_id);
    assert_ne!(updated.configuration_sha256, original.configuration_sha256);
    fixture.check().await.unwrap();
    fixture.runner.mode = "different-instance";
    assert!(
        refresh_compose_installation(
            &fixture.inputs,
            fixture.state.clone(),
            &fixture.runner,
            &CancellationToken::new()
        )
        .await
        .is_err()
    );
    fixture.runner.mode = "normal";
    let state = JournalDirectory::open(fixture.state.clone()).unwrap();
    state
        .create("unresolved.json", &json!({"preserve":true}))
        .unwrap();
    assert!(
        refresh_compose_installation(
            &fixture.inputs,
            fixture.state.clone(),
            &fixture.runner,
            &CancellationToken::new()
        )
        .await
        .is_err()
    );
    assert_eq!(
        state
            .read::<RegisteredComposeInstallation>(REGISTRATION)
            .unwrap(),
        updated
    );
    assert_eq!(
        state.read::<serde_json::Value>("unresolved.json").unwrap(),
        json!({"preserve":true})
    );
}
