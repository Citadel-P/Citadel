use super::*;
fn release(tag: &str) -> Release {
    Release {
        tag_name: tag.into(),
        draft: false,
        prerelease: false,
    }
}

#[test]
fn compares_semantic_versions_and_ignores_build_metadata() {
    for (current, tag, expected) in [
        ("1.0.0", "v1.1.0", true),
        ("1.9.0", "v1.10.0", true),
        ("1.1.0", "v1.1.0", false),
        ("1.2.0", "v1.1.0", false),
        ("1.1.0+local", "v1.1.0+release", false),
        ("1.1.0-dev.1", "v1.1.0", false),
        ("1.2.0-dev.1", "v1.1.0", false),
        ("1.0.0", "v1.1.0-dev.1", false),
    ] {
        assert_eq!(
            available_update(current, release(tag)).unwrap().is_some(),
            expected,
            "{current} -> {tag}"
        );
    }
    let update = available_update("1.0.0", release("v1.1.0"))
        .unwrap()
        .unwrap();
    assert_eq!(update.version, "1.1.0");
    assert_eq!(
        update.release_url,
        "https://github.com/Citadel-P/Citadel/releases/tag/v1.1.0"
    );
}

fn development_release(tag: &str) -> Release {
    Release {
        prerelease: true,
        ..release(tag)
    }
}

#[test]
fn development_and_stable_installations_follow_their_own_channels() {
    for (current, tag, expected) in [
        ("0.1.4-dev.1", "v0.1.4-dev.1", false),
        ("0.1.4-dev.1+local", "v0.1.4-dev.1+release", false),
        ("0.1.4-dev.1", "v0.1.4-dev.2", true),
        ("0.1.4-dev.9", "v0.1.4-dev.10", true),
        ("0.1.4-dev.2", "v0.1.4-dev.1", false),
        ("0.1.4-dev.1", "v0.1.5-dev.1", true),
        ("0.1.4-dev.1", "v0.1.5-rc.1", false),
        ("0.1.4", "v0.1.5-dev.1", false),
    ] {
        assert_eq!(
            available_update(current, development_release(tag))
                .unwrap()
                .is_some(),
            expected,
            "{current} -> {tag}"
        );
    }
    assert!(
        available_update("0.1.4-dev.1", release("v0.1.5"))
            .unwrap()
            .is_none()
    );
    assert!(
        available_update("0.1.4", release("v0.1.5"))
            .unwrap()
            .is_some()
    );
}

#[test]
fn development_list_selects_highest_version_and_excludes_other_releases() {
    let mut draft = development_release("v9.0.0-dev.1");
    draft.draft = true;
    let releases = vec![
        release("v9.0.0"),
        draft,
        development_release("v2.0.0-rc.1"),
        release("v3.0.0-dev.1"), // A dev tag must also be published as a prerelease.
        release("not-a-version"),
        development_release("v0.1.4-dev.10"),
        development_release("v0.1.4-dev.2"),
    ];
    let update = development_update("0.1.4-dev.1", releases)
        .unwrap()
        .unwrap();
    assert_eq!(update.version, "0.1.4-dev.10");
    assert_eq!(
        update.release_url,
        "https://github.com/Citadel-P/Citadel/releases/tag/v0.1.4-dev.10"
    );
    assert!(
        development_update(
            "0.1.4-dev.1",
            vec![release("v0.1.4"), development_release("v0.1.4-dev.1")]
        )
        .unwrap()
        .is_none()
    );
    assert!(development_update("unknown", vec![]).is_err());
}

#[test]
fn rejects_drafts_prereleases_and_invalid_tags() {
    let mut draft = release("v2.0.0");
    draft.draft = true;
    assert!(available_update("1.0.0", draft).unwrap().is_none());
    let mut preview = release("v2.0.0");
    preview.prerelease = true;
    assert!(available_update("1.0.0", preview).unwrap().is_none());
    for tag in ["latest", "v2.0.0/../../evil", "2.0", "<script>"] {
        assert!(available_update("1.0.0", release(tag)).is_err());
    }
    assert!(available_update("unknown", release("v2.0.0")).is_err());
}
