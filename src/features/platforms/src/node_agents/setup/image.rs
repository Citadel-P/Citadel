use super::{SetupKind, fail};
use crate::RuntimeCapabilityError;

pub(super) fn selected(
    kind: SetupKind,
    configured: &str,
    approved: Option<&str>,
) -> Result<String, RuntimeCapabilityError> {
    match (kind, approved) {
        (SetupKind::Upgrade, _) | (SetupKind::Install, None) => Ok(configured.into()),
        (SetupKind::Repair | SetupKind::Install, Some(image)) => Ok(image.into()),
        (SetupKind::Repair, None) => Err(fail(
            "No approved Agent image is recorded. Install node agents before repairing coverage.",
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repair_and_repeated_install_keep_the_approved_image() {
        for kind in [SetupKind::Repair, SetupKind::Install] {
            assert_eq!(
                selected(kind, "agent:new", Some("agent@sha256:old")).unwrap(),
                "agent@sha256:old"
            );
        }
        assert_eq!(
            selected(SetupKind::Upgrade, "agent:new", Some("agent@sha256:old")).unwrap(),
            "agent:new"
        );
        assert!(selected(SetupKind::Repair, "agent:new", None).is_err());
    }
}
