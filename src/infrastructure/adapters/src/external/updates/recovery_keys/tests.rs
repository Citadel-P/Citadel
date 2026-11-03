use super::*;
use base64::{Engine, engine::general_purpose::STANDARD};
use std::{
    fs,
    os::unix::fs::{PermissionsExt, symlink},
    path::Path,
};

fn write(path: &Path, bytes: impl AsRef<[u8]>) {
    fs::write(path, bytes).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
}

fn fixture() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    fs::create_dir(root.path().join("agent")).unwrap();
    write(
        &root.path().join("jwtsecret"),
        "fixture-jwt-key-material-1234567890",
    );
    write(
        &root.path().join("secret-encryption-key"),
        STANDARD.encode([7; 32]),
    );
    write(&root.path().join("agent/signing-key"), [9; 32]);
    root
}

#[test]
fn file_keys_are_read_only_and_report_contains_no_material() {
    let root = fixture();
    let before = fs::read(root.path().join("jwtsecret")).unwrap();
    let result = check_recovery_keys(root.path(), None, None).unwrap();
    assert_eq!(result.jwt, RecoveryKeySource::DataVolume);
    assert_eq!(result.secrets_encryption, RecoveryKeySource::DataVolume);
    assert_eq!(result.agent_signing, RecoveryKeySource::DataVolume);
    assert_eq!(
        serde_json::to_value(result).unwrap(),
        serde_json::json!({
            "jwt":"DataVolume", "secretsEncryption":"DataVolume", "agentSigning":"DataVolume"
        })
    );
    assert_eq!(fs::read(root.path().join("jwtsecret")).unwrap(), before);
    assert_eq!(fs::read_dir(root.path()).unwrap().count(), 3);
}

#[test]
fn explicit_keys_take_precedence_and_blanks_use_existing_files() {
    let root = fixture();
    let encryption = STANDARD.encode([8; 32]);
    let result = check_recovery_keys(
        root.path(),
        Some("external-jwt-fixture-123456789012345"),
        Some(&encryption),
    )
    .unwrap();
    assert_eq!(result.jwt, RecoveryKeySource::Environment);
    assert_eq!(result.secrets_encryption, RecoveryKeySource::Environment);
    assert_eq!(
        check_recovery_keys(root.path(), Some(" \n"), Some(" "))
            .unwrap()
            .jwt,
        RecoveryKeySource::DataVolume
    );
    fs::remove_file(root.path().join("secret-encryption-key")).unwrap();
    assert!(check_recovery_keys(root.path(), None, Some(&encryption)).is_ok());
    assert!(check_recovery_keys(root.path(), None, Some(" ")).is_err());
    assert!(!root.path().join("secret-encryption-key").exists());
}

#[test]
fn rejects_missing_malformed_oversized_and_unsafe_keys_without_repair() {
    for name in ["jwtsecret", "secret-encryption-key", "agent/signing-key"] {
        for mode in [
            "missing",
            "empty",
            "malformed",
            "oversized",
            "permissions",
            "symlink",
            "directory",
        ] {
            let root = fixture();
            let path = root.path().join(name);
            match mode {
                "missing" => fs::remove_file(&path).unwrap(),
                "empty" => write(&path, []),
                "malformed" => write(&path, b"private-malformed-fixture"),
                "oversized" => write(&path, vec![b'x'; 16 * 1024 + 1]),
                "permissions" => {
                    fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap()
                }
                "symlink" => {
                    fs::remove_file(&path).unwrap();
                    symlink("/does-not-exist", &path).unwrap();
                }
                "directory" => {
                    fs::remove_file(&path).unwrap();
                    fs::create_dir(&path).unwrap();
                }
                _ => unreachable!(),
            }
            let error = check_recovery_keys(root.path(), None, None).unwrap_err();
            assert!(!error.to_string().contains("private-malformed-fixture"));
            if mode == "missing" {
                assert!(!path.exists());
            }
            if mode == "malformed" {
                assert_eq!(fs::read(&path).unwrap(), b"private-malformed-fixture");
            }
        }
    }
}

#[test]
fn rejects_symlinked_ancestors_and_nonregular_files_without_blocking() {
    let root = fixture();
    let agent = root.path().join("agent");
    fs::rename(&agent, root.path().join("original-agent")).unwrap();
    symlink("original-agent", &agent).unwrap();
    assert!(check_recovery_keys(root.path(), None, None).is_err());
    let root = fixture();
    let path = root.path().join("jwtsecret");
    fs::remove_file(&path).unwrap();
    rustix::fs::mkfifoat(
        rustix::fs::CWD,
        &path,
        rustix::fs::Mode::from_raw_mode(0o600),
    )
    .unwrap();
    assert!(check_recovery_keys(root.path(), None, None).is_err());
}

#[test]
fn validates_explicit_encoding_and_lengths_without_falling_back() {
    let root = fixture();
    for key in [
        "private-invalid-key".to_owned(),
        STANDARD.encode([7; 31]),
        "x".repeat(16 * 1024 + 1),
    ] {
        let error = check_recovery_keys(root.path(), None, Some(&key)).unwrap_err();
        assert!(!error.to_string().contains(&key));
    }
    assert!(check_recovery_keys(root.path(), Some("short"), None).is_err());
    assert!(check_recovery_keys(root.path(), Some(&"x".repeat(16 * 1024 + 1)), None).is_err());
}

#[test]
#[ignore = "Requires root in a disposable container with a fresh tmpfs at /app/data"]
fn root_helper_reads_packaged_core_keys_without_trusting_other_owners() {
    assert_eq!(
        std::env::var("CITADEL_TEST_PACKAGED_KEYS").as_deref(),
        Ok("1")
    );
    assert_eq!(rustix::process::geteuid().as_raw(), 0);
    assert!(std::path::Path::new("/.dockerenv").exists());
    assert!(
        fs::read_to_string("/proc/mounts")
            .unwrap()
            .lines()
            .any(|line| {
                let fields: Vec<_> = line.split_whitespace().collect();
                fields.get(1) == Some(&"/app/data") && fields.get(2) == Some(&"tmpfs")
            })
    );
    let root = Path::new("/app/data");
    assert_eq!(fs::read_dir(root).unwrap().count(), 0);
    fs::create_dir(root.join("agent")).unwrap();
    write(
        &root.join("jwtsecret"),
        "fixture-jwt-key-material-1234567890",
    );
    write(
        &root.join("secret-encryption-key"),
        STANDARD.encode([7; 32]),
    );
    write(&root.join("agent/signing-key"), [9; 32]);
    for path in [
        root.to_path_buf(),
        root.join("agent"),
        root.join("jwtsecret"),
        root.join("secret-encryption-key"),
        root.join("agent/signing-key"),
    ] {
        std::os::unix::fs::chown(path, Some(65532), Some(0)).unwrap();
    }
    // The ordinary key API retains its existing ownership requirements.
    assert!(check_recovery_keys(root, None, None).is_err());
    super::validation::check_packaged_recovery_keys(None, None).unwrap();
    let key = root.join("agent/signing-key");
    std::os::unix::fs::chown(&key, Some(65531), None).unwrap();
    assert!(super::validation::check_packaged_recovery_keys(None, None).is_err());
    std::os::unix::fs::chown(&key, Some(65532), None).unwrap();
    fs::set_permissions(&key, fs::Permissions::from_mode(0o640)).unwrap();
    assert!(super::validation::check_packaged_recovery_keys(None, None).is_err());
    fs::remove_file(&key).unwrap();
    symlink("/tmp/not-a-key", &key).unwrap();
    assert!(super::validation::check_packaged_recovery_keys(None, None).is_err());
}
