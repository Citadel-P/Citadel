use super::*;
use crate::CoreIdentity;

fn catalog(count: usize) -> String {
    serde_json::json!({"migrations": (1..=count).map(|i|
        serde_json::json!({"id": format!("{i:04}"), "sha256": format!("{i:064x}")}))
        .collect::<Vec<_>>()})
    .to_string()
}

fn digest(catalog: &str) -> String {
    use sha2::{Digest, Sha256};
    Sha256::digest(catalog.as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>()
}

fn identities() -> (CoreIdentity, CoreIdentity, UpgradeDescriptor) {
    let source = CoreIdentity {
        version: "0.1.1".into(),
        image: format!("{CORE_IMAGE}@sha256:{}", "a".repeat(64)),
        catalog_sha256: digest(&catalog(1)),
        architecture: "amd64".into(),
    };
    let target = CoreIdentity {
        version: "0.1.2".into(),
        image: format!("{CORE_IMAGE}@sha256:{}", "c".repeat(64)),
        catalog_sha256: digest(&catalog(2)),
        architecture: "amd64".into(),
    };
    let descriptor = UpgradeDescriptor {
        schema_version: 2,
        product: "citadel".into(),
        target_version: target.version.clone(),
        target_catalog_sha256: target.catalog_sha256.clone(),
        installer_format: 1,
        update_protocol: 1,
        migration_catalog: catalog(2),
        agent_protocol: 2,
    };
    (source, target, descriptor)
}

#[test]
fn newer_versions_use_migration_history_without_a_version_allowlist() {
    let (mut source, target, descriptor) = identities();
    for version in ["0.1.0", "0.1.1"] {
        source.version = version.into();
        assert!(
            descriptor
                .validate_transition(&source, &target, &catalog(1), 2, &[])
                .is_ok()
        );
    }
    source.catalog_sha256 = digest(&catalog(2));
    assert!(
        descriptor
            .validate_transition(&source, &target, &catalog(2), 2, &[])
            .is_ok()
    );
}

#[test]
fn altered_removed_reordered_or_unbound_migrations_are_rejected() {
    let (mut source, target, descriptor) = identities();
    let mut changed: serde_json::Value = serde_json::from_str(&catalog(1)).unwrap();
    changed["migrations"][0]["sha256"] = "f".repeat(64).into();
    for catalog in [
        changed.to_string(),
        catalog(3),
        r#"{"migrations":[]}"#.into(),
    ] {
        source.catalog_sha256 = digest(&catalog);
        assert!(
            descriptor
                .validate_transition(&source, &target, &catalog, 2, &[])
                .is_err()
        );
    }
    let mut reordered: serde_json::Value = serde_json::from_str(&catalog(2)).unwrap();
    reordered["migrations"].as_array_mut().unwrap().reverse();
    let catalog = reordered.to_string();
    source.catalog_sha256 = digest(&catalog);
    assert!(
        descriptor
            .validate_transition(&source, &target, &catalog, 2, &[])
            .is_err()
    );
    source.catalog_sha256 = "f".repeat(64);
    assert!(
        descriptor
            .validate_transition(&source, &target, &self::catalog(1), 2, &[])
            .is_err()
    );
    source.catalog_sha256 = digest(&self::catalog(1));
    source.architecture = "arm64".into();
    assert!(
        descriptor
            .validate_transition(&source, &target, &self::catalog(1), 2, &[])
            .is_err()
    );
}

#[test]
fn registered_agents_require_the_same_protocol_not_a_release_version_list() {
    let (source, target, descriptor) = identities();
    for agents in [vec![], vec!["0.1.0".into(), "0.1.1".into()]] {
        assert!(
            descriptor
                .validate_transition(&source, &target, &catalog(1), 2, &agents)
                .is_ok()
        );
    }
    assert!(
        descriptor
            .validate_transition(&source, &target, &catalog(1), 1, &["0.1.1".into()])
            .is_err()
    );
    assert!(
        descriptor
            .validate_transition(&source, &target, &catalog(1), 2, &["unknown".into()])
            .is_err()
    );
    assert!(
        descriptor
            .validate_transition(&source, &target, &catalog(1), 1, &[])
            .is_ok()
    );
}

#[test]
fn moving_tags_foreign_images_channel_switches_and_downgrades_are_rejected() {
    let (source, target, descriptor) = identities();
    for image in [
        "ghcr.io/citadel-p/citadel:latest".to_owned(),
        format!("evil.invalid/core@sha256:{}", "c".repeat(64)),
        format!("{CORE_IMAGE}@sha256:bad"),
    ] {
        assert!(
            descriptor
                .validate_transition(
                    &source,
                    &CoreIdentity {
                        image,
                        ..target.clone()
                    },
                    &catalog(1),
                    2,
                    &[]
                )
                .is_err()
        );
    }
    for version in ["0.1.0", "0.1.1", "0.1.2-dev.1", "0.1.2+unknown"] {
        let mut changed = descriptor.clone();
        changed.target_version = version.into();
        assert!(
            changed
                .validate_transition(
                    &source,
                    &CoreIdentity {
                        version: version.into(),
                        ..target.clone()
                    },
                    &catalog(1),
                    2,
                    &[]
                )
                .is_err()
        );
    }
}

#[test]
fn unknown_descriptor_versions_and_unbound_catalogs_are_rejected() {
    let (source, target, descriptor) = identities();
    for changed in [
        UpgradeDescriptor {
            schema_version: 3,
            ..descriptor.clone()
        },
        UpgradeDescriptor {
            update_protocol: 2,
            ..descriptor.clone()
        },
        UpgradeDescriptor {
            target_catalog_sha256: "e".repeat(64),
            ..descriptor.clone()
        },
        UpgradeDescriptor {
            installer_format: 2,
            ..descriptor.clone()
        },
    ] {
        assert!(
            changed
                .validate_transition(&source, &target, &catalog(1), 2, &[])
                .is_err()
        );
    }
}
