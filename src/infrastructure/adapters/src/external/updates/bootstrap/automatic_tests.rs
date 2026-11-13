use super::automatic::*;
use super::fixtures::Fixture;
use serde_json::json;
use std::{fs, os::unix::fs::PermissionsExt};
use tokio_util::sync::CancellationToken;

fn setup(f: &Fixture) -> AutomaticComposeSetup {
    AutomaticComposeSetup {
        inputs: f.inputs.clone(),
        state_directory: f.state.clone(),
    }
}

#[tokio::test]
async fn pending_setup_is_atomic_resumable_and_never_recreates_lost_registration() {
    let f = Fixture::new();
    let setup = setup(&f);
    let token = CancellationToken::new();
    assert!(
        prepare_automatic_setup(&setup, false, &f.runner, &token)
            .await
            .is_err()
    );
    assert!(
        prepare_automatic_setup(&setup, true, &f.runner, &token)
            .await
            .unwrap()
            .is_none()
    );
    let before = fs::read(f.state.join("installation.json")).unwrap();
    assert!(
        prepare_automatic_setup(&setup, false, &f.runner, &token)
            .await
            .unwrap()
            .is_none()
    );
    assert_eq!(fs::read(f.state.join("installation.json")).unwrap(), before);
    let registration = complete_automatic_setup(&setup, &f.runner, &token)
        .await
        .unwrap();
    assert_eq!(
        prepare_automatic_setup(&setup, false, &f.runner, &token)
            .await
            .unwrap(),
        Some(registration.clone())
    );
    assert_eq!(
        complete_automatic_setup(&setup, &f.runner, &token)
            .await
            .unwrap(),
        registration
    );
    fs::remove_file(f.state.join("installation.json")).unwrap();
    assert!(
        prepare_automatic_setup(&setup, false, &f.runner, &token)
            .await
            .is_err()
    );
    assert!(!f.state.join("installation.json").exists());
}

#[tokio::test]
async fn automatic_setup_preserves_corrupt_or_orphaned_evidence_and_permissions() {
    for evidence in ["installation.json", "operation.json", "unrecognized"] {
        let f = Fixture::new();
        fs::write(f.state.join(evidence), b"corrupt evidence").unwrap();
        fs::set_permissions(f.state.join(evidence), fs::Permissions::from_mode(0o600)).unwrap();
        assert!(
            prepare_automatic_setup(&setup(&f), true, &f.runner, &CancellationToken::new())
                .await
                .is_err()
        );
        assert_eq!(
            fs::read(f.state.join(evidence)).unwrap(),
            b"corrupt evidence"
        );
    }
    let f = Fixture::new();
    fs::set_permissions(&f.state, fs::Permissions::from_mode(0o755)).unwrap();
    assert!(
        prepare_automatic_setup(&setup(&f), false, &f.runner, &CancellationToken::new())
            .await
            .is_err()
    );
    assert!(
        prepare_automatic_setup(&setup(&f), true, &f.runner, &CancellationToken::new())
            .await
            .unwrap()
            .is_none()
    );
    assert_eq!(
        fs::metadata(&f.state).unwrap().permissions().mode() & 0o777,
        0o700
    );
}

#[tokio::test]
async fn automatic_completion_refuses_drift_cancel_and_concurrent_owner() {
    let mut f = Fixture::new();
    let setup = setup(&f);
    let token = CancellationToken::new();
    prepare_automatic_setup(&setup, true, &f.runner, &token)
        .await
        .unwrap();
    let before = fs::read(f.state.join("installation.json")).unwrap();
    f.runner.mode = "changed-daemon";
    assert!(
        complete_automatic_setup(&setup, &f.runner, &token)
            .await
            .is_err()
    );
    let state = super::super::JournalDirectory::open(f.state.clone()).unwrap();
    let owner = state.existing_lock().unwrap();
    assert!(
        complete_automatic_setup(&setup, &f.runner, &token)
            .await
            .is_err()
    );
    drop(owner);
    token.cancel();
    assert!(
        complete_automatic_setup(&setup, &f.runner, &token)
            .await
            .is_err()
    );
    assert_eq!(fs::read(f.state.join("installation.json")).unwrap(), before);
}

#[test]
fn discovery_uses_host_metadata_and_rejects_wrong_mounts_and_identity() {
    let f = Fixture::new();
    let hostname = &super::fixtures::CORE[..12];
    let mut core = f.runner.core.clone();
    core["Config"]["Hostname"] = json!(hostname);
    let path = f.inputs.directory.join(".citadel-updates");
    core["Mounts"].as_array_mut().unwrap().push(
        json!({"Type":"bind", "Source":path, "Destination":AUTOMATIC_STATE_MOUNT, "RW":false}),
    );
    assert_eq!(discover(&core, hostname).unwrap().state_directory, path);
    for field in ["Source", "Type", "RW"] {
        let mut changed = core.clone();
        let mount = changed["Mounts"]
            .as_array_mut()
            .unwrap()
            .last_mut()
            .unwrap();
        mount[field] = if field == "RW" {
            json!(true)
        } else {
            json!("other")
        };
        assert!(discover(&changed, hostname).is_err());
    }
    core["Config"]["Hostname"] = json!("other");
    assert!(discover(&core, hostname).is_err());
}

#[tokio::test]
async fn automatic_startup_never_repairs_a_missing_lock_for_existing_evidence() {
    let f = Fixture::new();
    let setup = setup(&f);
    let token = CancellationToken::new();
    prepare_automatic_setup(&setup, true, &f.runner, &token)
        .await
        .unwrap();
    fs::remove_file(f.state.join("installation.lock")).unwrap();
    assert!(
        prepare_automatic_setup(&setup, true, &f.runner, &token)
            .await
            .is_err()
    );
    assert!(!f.state.join("installation.lock").exists());
}
