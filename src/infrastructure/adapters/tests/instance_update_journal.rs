#![cfg(target_os = "linux")]
use citadel_adapters::external::updates::JournalDirectory;
use std::{
    fs,
    os::unix::fs::{PermissionsExt, symlink},
};

struct TemporaryDirectory(std::path::PathBuf);
impl TemporaryDirectory {
    fn path(&self) -> &std::path::Path {
        &self.0
    }
}
impl Drop for TemporaryDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn private_directory() -> TemporaryDirectory {
    let directory = TemporaryDirectory(
        std::env::temp_dir().join(format!("citadel-update-journal-{}", uuid::Uuid::now_v7())),
    );
    fs::create_dir(directory.path()).unwrap();
    fs::set_permissions(directory.path(), fs::Permissions::from_mode(0o700)).unwrap();
    directory
}

#[test]
fn journal_survives_reopening_and_lock_is_exclusive() {
    let temp = private_directory();
    let first = JournalDirectory::open(temp.path().into()).unwrap();
    let second = JournalDirectory::open(temp.path().into()).unwrap();
    let lock = first.lock().unwrap();
    assert!(second.lock().is_err());
    first
        .write("status.json", &serde_json::json!({"phase":"Draining"}))
        .unwrap();
    assert_eq!(
        second.read::<serde_json::Value>("status.json").unwrap()["phase"],
        "Draining"
    );
    drop(lock);
    second.lock().unwrap();
    assert_eq!(
        fs::metadata(temp.path().join("status.json"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o600
    );
}

#[test]
fn corrupt_large_and_non_private_state_is_rejected() {
    let temp = private_directory();
    let journal = JournalDirectory::open(temp.path().into()).unwrap();
    journal.write("state.json", &"valid").unwrap();
    fs::write(temp.path().join("state.json"), b"not json").unwrap();
    assert!(journal.read::<serde_json::Value>("state.json").is_err());
    assert!(
        journal
            .write("state.json", &"x".repeat(256 * 1024))
            .is_err()
    );
    fs::set_permissions(
        temp.path().join("state.json"),
        fs::Permissions::from_mode(0o644),
    )
    .unwrap();
    assert!(journal.read::<serde_json::Value>("state.json").is_err());
    fs::set_permissions(temp.path(), fs::Permissions::from_mode(0o755)).unwrap();
    assert!(JournalDirectory::open(temp.path().into()).is_err());
}

#[test]
fn exclusive_creation_preserves_existing_state_and_never_follows_links() {
    let temp = private_directory();
    let state = JournalDirectory::open(temp.path().into()).unwrap();
    assert!(
        state
            .read_optional::<String>("installation.json")
            .unwrap()
            .is_none()
    );
    state.create("installation.json", &"original").unwrap();
    assert!(state.create("installation.json", &"replacement").is_err());
    assert_eq!(
        state.read::<String>("installation.json").unwrap(),
        "original"
    );
    symlink(
        temp.path().join("installation.json"),
        temp.path().join("linked.json"),
    )
    .unwrap();
    assert!(state.create("linked.json", &"replacement").is_err());
    assert!(state.read_optional::<String>("linked.json").is_err());
    assert_eq!(
        state.read::<String>("installation.json").unwrap(),
        "original"
    );
    assert_eq!(fs::read_dir(temp.path()).unwrap().count(), 2);
}

#[test]
fn symlink_inputs_and_traversal_are_rejected_without_following_them() {
    let temp = private_directory();
    let outside = private_directory();
    fs::write(outside.path().join("secret"), b"original").unwrap();
    let journal = JournalDirectory::open(temp.path().into()).unwrap();
    symlink(
        outside.path().join("secret"),
        temp.path().join("state.json"),
    )
    .unwrap();
    assert!(journal.read::<String>("state.json").is_err());
    assert!(journal.write("../secret", &"changed").is_err());
    journal.write("state.json", &"new state").unwrap();
    assert_eq!(
        fs::read(outside.path().join("secret")).unwrap(),
        b"original"
    );
    symlink(
        outside.path().join("secret"),
        temp.path().join("installation.lock"),
    )
    .unwrap();
    assert!(journal.lock().is_err());
    symlink(outside.path(), temp.path().join("linked")).unwrap();
    assert!(JournalDirectory::open(temp.path().join("linked")).is_err());
}

#[test]
fn replacing_directory_path_does_not_redirect_an_open_journal() {
    let temp = private_directory();
    let path = temp.path().join("state");
    fs::create_dir(&path).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
    let journal = JournalDirectory::open(path.clone()).unwrap();
    fs::rename(&path, temp.path().join("retained")).unwrap();
    let outside = private_directory();
    symlink(outside.path(), &path).unwrap();
    journal.write("state.json", &"retained").unwrap();
    assert!(!outside.path().join("state.json").exists());
    assert!(temp.path().join("retained/state.json").exists());
}

#[test]
fn registration_fingerprint_detects_changed_inputs_and_requires_private_environment() {
    use citadel_adapters::external::updates::RegisteredComposeInstallation;
    use uuid::Uuid;
    let installation = private_directory();
    let state = private_directory();
    let base = installation.path().join("docker-compose.yml");
    let overlay = installation.path().join("compose.edge.yml");
    let environment = installation.path().join(".env");
    fs::write(&base, "services: {}\n").unwrap();
    fs::write(&overlay, "services: {}\n").unwrap();
    fs::write(&environment, "COMPOSE_PROJECT_NAME=citadel-test\n").unwrap();
    fs::set_permissions(&environment, fs::Permissions::from_mode(0o600)).unwrap();
    let mut registration = RegisteredComposeInstallation {
        format: 1,
        kind: citadel_updates::InstallationKind::DockerCompose,
        id: Uuid::now_v7(),
        instance_id: Uuid::now_v7(),
        directory: installation.path().into(),
        project: "citadel-test".into(),
        compose_files: vec![base, overlay],
        environment_file: environment.clone(),
        docker_socket: "/var/run/docker.sock".into(),
        daemon_id: "fixture-daemon".into(),
        core_container_id: "fixture-core".into(),
        database_container_id: "fixture-database".into(),
        core_volume: "fixture_core".into(),
        database_volume: "fixture_pg".into(),
        network: "fixture_default".into(),
        state_directory: state.path().into(),
        executor_uid: rustix::process::geteuid().as_raw(),
        executor_gid: rustix::process::getegid().as_raw(),
        configuration_sha256: "a".repeat(64),
        runtime_sha256: "b".repeat(64),
    };
    let initial = registration.fingerprint(std::path::Path::new("/")).unwrap();
    registration.compose_files.reverse();
    assert_ne!(
        registration.fingerprint(std::path::Path::new("/")).unwrap(),
        initial
    );
    registration.compose_files.reverse();
    fs::write(
        &environment,
        "COMPOSE_PROJECT_NAME=citadel-test\nCITADEL_HTTP_PORT=9000\n",
    )
    .unwrap();
    assert_ne!(
        registration.fingerprint(std::path::Path::new("/")).unwrap(),
        initial
    );
    fs::set_permissions(&environment, fs::Permissions::from_mode(0o644)).unwrap();
    assert!(registration.fingerprint(std::path::Path::new("/")).is_err());
    registration
        .compose_files
        .push(state.path().join("unregistered.yml"));
    assert!(registration.validate().is_err());
}
