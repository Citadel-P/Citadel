use super::*;

#[test]
fn permission_decoding_rejects_invalid_hierarchy_and_preserves_admin() {
    assert!(decode_permission(false, 7, 63).is_err());
    assert!(decode_permission(false, 3, 4).is_err());
    assert_eq!(
        decode_permission(true, 0, 0).unwrap(),
        EffectivePermission::Administrator
    );
}

#[test]
fn duplicate_names_remain_bounded() {
    let name = duplicate_name(&"x".repeat(80), "copy-100");
    assert!(name.len() <= 64);
    assert!(name.ends_with("-copy-100"));
}

#[test]
fn only_host_bind_mounts_emit_warning() {
    assert!(has_likely_host_bind(Some(&["./data:/data".to_owned()])));
    assert!(has_likely_host_bind(Some(&["/srv/data:/data".to_owned()])));
    assert!(!has_likely_host_bind(Some(&["named:/data".to_owned()])));
}
