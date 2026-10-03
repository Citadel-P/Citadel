use citadel_platforms::volume_content::normalize_path;

// Ports VolumePathNormalizerTests, with control-character and size boundaries.
#[test]
fn volume_paths_preserve_canonical_paths_and_reject_traversal() {
    for input in [None, Some(""), Some("/")] {
        assert_eq!(normalize_path(input).unwrap(), "/");
    }
    for input in [
        "/config",
        "/config/appsettings.json",
        "/目录/with spaces.txt",
    ] {
        assert_eq!(normalize_path(Some(input)).unwrap(), input);
    }
    for input in [
        "config",
        "/config/",
        "/config//appsettings.json",
        "/config/./appsettings.json",
        "/config/../secret",
        "/config\\appsettings.json",
        "/a\0",
        "/a\n",
    ] {
        assert!(normalize_path(Some(input)).is_err(), "{input:?}");
    }
    assert!(normalize_path(Some(&format!("/{}", "x".repeat(4096)))).is_err());
}
