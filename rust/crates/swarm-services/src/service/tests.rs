use super::*;

#[test]
fn binding_references_are_unique_and_case_insensitive() {
    let references = referenced_binding_names(&[
        "TOKEN=${API_TOKEN}".to_owned(),
        "URL=https://${HOST}/api/${api_token}".to_owned(),
        "PLAIN".to_owned(),
    ])
    .unwrap();

    assert_eq!(references, ["API_TOKEN", "HOST", "PLAIN"]);
}

#[test]
fn incomplete_binding_reference_is_rejected() {
    assert!(referenced_binding_names(&["TOKEN=${MISSING".to_owned()]).is_err());
}

#[test]
fn secret_values_are_redacted_from_runtime_errors() {
    assert_eq!(
        redact(
            "Docker rejected value very-secret".to_owned(),
            &["very-secret".to_owned()]
        ),
        "Docker rejected value ********"
    );
}
