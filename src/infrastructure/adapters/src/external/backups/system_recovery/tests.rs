use super::{builder::*, files::*, postgres::*, recovery::*};
use citadel_backups::BackupError;
use std::{collections::BTreeMap, path::Path};
use uuid::Uuid;

#[tokio::test]
async fn oversized_recovery_manifest_is_rejected_before_parsing() {
    let root = std::env::temp_dir().join(format!("citadel-recovery-limit-{}", Uuid::now_v7()));
    tokio::fs::create_dir(&root).await.unwrap();
    let file = tokio::fs::File::create(root.join("manifest.json"))
        .await
        .unwrap();
    file.set_len(1024 * 1024 * 1024).await.unwrap();
    drop(file);
    tokio::fs::write(root.join("checksums.json"), b"{}")
        .await
        .unwrap();
    let error = validate_recovery_bundle(&root).await.unwrap_err();
    assert!(matches!(error, BackupError::Validation(_)));
    assert!(error.to_string().contains("size limit"));
    tokio::fs::remove_dir_all(root).await.unwrap();
}

#[tokio::test]
async fn recovery_key_read_accepts_the_limit_and_rejects_an_extra_byte() {
    let path = std::env::temp_dir().join(format!("citadel-recovery-key-{}", Uuid::now_v7()));
    let file = tokio::fs::File::create(&path).await.unwrap();
    file.set_len(MAX_RECOVERY_KEY_BYTES).await.unwrap();
    let key = read_recovery_file(&path, MAX_RECOVERY_KEY_BYTES)
        .await
        .unwrap();
    assert_eq!(key.len() as u64, MAX_RECOVERY_KEY_BYTES);
    file.set_len(MAX_RECOVERY_KEY_BYTES + 1).await.unwrap();
    assert!(matches!(
        read_recovery_file(&path, MAX_RECOVERY_KEY_BYTES).await,
        Err(BackupError::Validation(_))
    ));
    drop(file);
    tokio::fs::remove_file(path).await.unwrap();
}

#[tokio::test]
async fn file_backed_keys_are_archived_and_external_keys_are_never_copied() {
    let root = std::env::temp_dir().join(format!("citadel-recovery-{}", Uuid::now_v7()));
    let data = root.join("custom-data");
    tokio::fs::create_dir_all(data.join("agent")).await.unwrap();
    for file in ["jwtsecret", "secret-encryption-key", "agent/signing-key"] {
        tokio::fs::write(data.join(file), b"original-key")
            .await
            .unwrap();
    }
    let assets = RecoveryAssets {
        core_data_path: data.clone(),
        jwt_key_is_external: false,
        encryption_key_is_external: false,
    };
    let bundle = root.join("bundle");
    let mut checksums = BTreeMap::new();
    let (files, external) = copy_recovery_assets(&assets, &bundle, &mut checksums)
        .await
        .unwrap();
    assert_eq!(files.len(), 3);
    assert!(external.is_empty());
    for file in files {
        assert_eq!(
            tokio::fs::read(bundle.join(&file)).await.unwrap(),
            b"original-key"
        );
        assert_eq!(
            checksums[&file],
            sha256_file(&bundle.join(&file)).await.unwrap()
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                std::fs::metadata(bundle.join(&file))
                    .unwrap()
                    .permissions()
                    .mode()
                    & 0o777,
                0o600
            );
        }
    }
    let external_bundle = root.join("external");
    let (files, external) = copy_recovery_assets(
        &RecoveryAssets {
            jwt_key_is_external: true,
            encryption_key_is_external: true,
            ..assets.clone()
        },
        &external_bundle,
        &mut BTreeMap::new(),
    )
    .await
    .unwrap();
    assert_eq!(files, ["recovery/agent/signing-key"]);
    assert_eq!(
        external,
        ["CITADEL_JWT_KEY", "CITADEL_SECRETS_ENCRYPTION_KEY"]
    );
    assert!(!external_bundle.join("recovery/jwtsecret").exists());
    tokio::fs::remove_file(data.join("secret-encryption-key"))
        .await
        .unwrap();
    assert!(
        copy_recovery_assets(&assets, &root.join("missing"), &mut BTreeMap::new())
            .await
            .is_err()
    );
    tokio::fs::remove_dir_all(root).await.unwrap();
}

#[test]
fn postgres_credentials_are_passed_in_environment_not_arguments() {
    let (arguments, environment) = pg_dump_request(
        "postgres://citadel:secret@database:5432/citadel?sslmode=require",
        Path::new("dump"),
    )
    .unwrap();
    assert!(
        !arguments
            .iter()
            .any(|value| value.to_string_lossy().contains("secret"))
    );
    assert!(
        environment
            .iter()
            .any(|(key, value)| { key == "PGPASSWORD" && value == "secret" })
    );
    assert!(
        environment
            .iter()
            .any(|(key, value)| { key == "PGSSLMODE" && value == "require" })
    );
}

#[test]
fn restore_is_transactional_and_credentials_are_not_arguments() {
    let (arguments, environment) = pg_restore_request(
        "postgres://citadel:secret@database:5432/citadel",
        Path::new("citadel.dump"),
    )
    .unwrap();
    assert!(arguments.contains(&"--single-transaction".into()));
    assert!(arguments.contains(&"--exit-on-error".into()));
    assert!(!arguments.iter().any(|value| value == "secret"));
    assert!(
        environment
            .iter()
            .any(|(key, value)| key == "PGPASSWORD" && value == "secret")
    );
}

#[tokio::test]
async fn recovery_bundle_rejects_parent_path() {
    let root = std::env::temp_dir().join(format!("citadel-recovery-{}", Uuid::now_v7()));
    tokio::fs::create_dir(&root).await.unwrap();
    let error = checked_bundle_file(&root, "../citadel.dump")
        .await
        .unwrap_err();
    assert!(error.to_string().contains("unsafe path"));
    tokio::fs::remove_dir_all(root).await.unwrap();
}

#[tokio::test]
async fn recovery_bundle_detects_database_tampering_before_restore() {
    let root = std::env::temp_dir().join(format!("citadel-recovery-{}", Uuid::now_v7()));
    let database = root.join("database");
    tokio::fs::create_dir_all(&database).await.unwrap();
    let dump = database.join("citadel.dump");
    tokio::fs::write(&dump, b"PGDMPfixture").await.unwrap();
    let dump_hash = sha256_file(&dump).await.unwrap();
    tokio::fs::create_dir(root.join("recovery")).await.unwrap();
    let key = root.join("recovery/jwtsecret");
    tokio::fs::write(&key, b"original-key").await.unwrap();
    let manifest = RecoveryManifest {
        format_version: 1,
        product: "citadel",
        core_version: "test",
        instance_id: Uuid::now_v7(),
        created_at: chrono::Utc::now(),
        database: DatabaseManifest {
            engine: "PostgreSQL",
            path: "database/citadel.dump",
            sha256: dump_hash.clone(),
        },
        required_external_configuration: vec!["CITADEL_JWT_KEY", "CITADEL_SECRETS_ENCRYPTION_KEY"],
        recovery_files: vec!["recovery/jwtsecret".into()],
    };
    write_json(&root.join("manifest.json"), &manifest)
        .await
        .unwrap();
    let checksums = BTreeMap::from([
        (
            "recovery/jwtsecret".to_owned(),
            sha256_file(&key).await.unwrap(),
        ),
        ("database/citadel.dump".to_owned(), dump_hash),
        (
            "manifest.json".to_owned(),
            sha256_file(&root.join("manifest.json")).await.unwrap(),
        ),
    ]);
    write_json(&root.join("checksums.json"), &checksums)
        .await
        .unwrap();
    assert_eq!(
        validate_recovery_bundle(&root).await.unwrap(),
        dump.canonicalize().unwrap()
    );

    tokio::fs::write(&key, b"tampered-key").await.unwrap();
    assert!(
        validate_recovery_bundle(&root)
            .await
            .unwrap_err()
            .to_string()
            .contains("does not match")
    );
    tokio::fs::write(&key, b"original-key").await.unwrap();
    tokio::fs::write(&dump, b"PGDMPtampered").await.unwrap();
    let error = validate_recovery_bundle(&root).await.unwrap_err();
    assert!(error.to_string().contains("does not match"));
    tokio::fs::remove_dir_all(root).await.unwrap();
}
