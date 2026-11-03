use super::*;
use crate::CoreIdentity;

fn identities() -> (CoreIdentity, CoreIdentity, UpgradeDescriptor) {
    let source = CoreIdentity {
        version: "0.1.1".into(),
        image: format!("{CORE_IMAGE}@sha256:{}", "a".repeat(64)),
        catalog_sha256: "b".repeat(64),
        architecture: "amd64".into(),
    };
    let target = CoreIdentity {
        version: "0.1.2".into(),
        image: format!("{CORE_IMAGE}@sha256:{}", "c".repeat(64)),
        catalog_sha256: "d".repeat(64),
        architecture: "amd64".into(),
    };
    let descriptor = UpgradeDescriptor {
        schema_version: 1,
        product: "citadel".into(),
        target_version: target.version.clone(),
        target_catalog_sha256: target.catalog_sha256.clone(),
        installer_format: 1,
        update_protocol: 1,
        sources: vec![UpgradeSource {
            version: source.version.clone(),
            catalog_sha256: source.catalog_sha256.clone(),
            architectures: vec!["amd64".into()],
            agent_versions: vec![],
        }],
    };
    (source, target, descriptor)
}

#[test]
fn an_exact_source_catalog_and_architecture_are_required() {
    let (source, target, descriptor) = identities();
    assert!(
        descriptor
            .validate_transition(&source, &target, &[])
            .is_ok()
    );
    for altered in [
        CoreIdentity {
            catalog_sha256: "e".repeat(64),
            ..source.clone()
        },
        CoreIdentity {
            version: "0.1.0".into(),
            ..source.clone()
        },
        CoreIdentity {
            architecture: "arm64".into(),
            ..source.clone()
        },
    ] {
        assert!(
            descriptor
                .validate_transition(&altered, &target, &[])
                .is_err()
        );
    }
    let mut duplicated = descriptor.clone();
    duplicated.sources.push(duplicated.sources[0].clone());
    assert!(
        duplicated
            .validate_transition(&source, &target, &[])
            .is_err()
    );
}

#[test]
fn unknown_agent_transitions_remain_unavailable() {
    let (source, target, mut descriptor) = identities();
    assert!(
        descriptor
            .validate_transition(&source, &target, &["0.1.1".into()])
            .is_err()
    );
    descriptor.sources[0].agent_versions.push("0.1.1".into());
    assert!(
        descriptor
            .validate_transition(&source, &target, &["0.1.1".into()])
            .is_ok()
    );
    assert!(
        descriptor
            .validate_transition(&source, &target, &["0.1.1".into(), "unknown".into()])
            .is_err()
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
            schema_version: 2,
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
        assert!(changed.validate_transition(&source, &target, &[]).is_err());
    }
}
