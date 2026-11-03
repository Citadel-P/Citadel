use super::*;
use crate::CORE_IMAGE;
use serde_json::{Value, json};

const STABLE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../test/release/fixtures/update-stable.json"
));
const DEVELOPMENT: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../test/release/fixtures/update-development.json"
));

#[test]
fn official_publisher_fixtures_resolve_exact_core_digests() {
    for (text, version) in [(STABLE, "1.2.3"), (DEVELOPMENT, "1.2.4-dev.1")] {
        let manifest: PublishedReleaseManifest = serde_json::from_str(text).unwrap();
        assert_eq!(
            manifest.core_image(version).unwrap(),
            format!("{CORE_IMAGE}@sha256:{}", "b".repeat(64))
        );
        assert!(manifest.core_image("1.2.5").is_err());
    }
}

#[test]
fn untrusted_publication_fields_cannot_change_the_expected_identity() {
    let fixture: Value = serde_json::from_str(STABLE).unwrap();
    for (pointer, value) in [
        ("/schemaVersion", json!(2)),
        ("/tag", json!("v1.2.4")),
        ("/metadata/displayVersion", json!("1.2.4")),
        ("/metadata/dirty", json!(true)),
        ("/metadata/identityStatus", json!("fallback")),
        ("/metadata/sourceRevision", json!("invalid")),
        ("/metadata/releaseEligible", json!(false)),
        ("/metadata/developmentReleaseEligible", json!(true)),
        ("/architectures", json!(["amd64"])),
        ("/signatures/tool", json!("unchecked")),
        (
            "/signatures/certificateIdentity",
            json!("https://github.com/attacker/repo/.github/workflows/ci.yml@refs/tags/v1.2.3"),
        ),
        (
            "/signatures/certificateOidcIssuer",
            json!("https://attacker.invalid"),
        ),
        (
            "/images/core/0/tag",
            json!("ghcr.io/citadel-p/citadel:latest"),
        ),
        (
            "/images/core/0/digest",
            json!(format!("ghcr.io/attacker/core@sha256:{}", "b".repeat(64))),
        ),
        (
            "/images/core/1/digest",
            json!(format!(
                "docker.io/citadelplane/citadel@sha256:{}",
                "c".repeat(64)
            )),
        ),
        ("/verifiedSignatures", json!({})),
    ] {
        let mut document = fixture.clone();
        *document.pointer_mut(pointer).unwrap() = value;
        let manifest: PublishedReleaseManifest = serde_json::from_value(document).unwrap();
        assert!(manifest.core_image("1.2.3").is_err(), "{pointer}");
    }
    for version in [
        "latest",
        "v1.2.3",
        "1.2.3+dirty",
        "1.2.3/../../other",
        "1.2.3-dev.0",
        "1.2.3-rc.1",
    ] {
        assert!(release_signing_identity(version).is_err());
    }
}
