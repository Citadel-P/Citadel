use std::collections::HashSet;

use uuid::Uuid;

/// Trim optional text and treat whitespace-only values as absent.
/// Length limits and character restrictions belong to the calling feature.
#[must_use]
pub fn optional_text(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

/// Remove repeated IDs while preserving their first occurrence, including nil IDs.
/// Callers remain responsible for rejecting invalid identifiers.
#[must_use]
pub fn unique_ids(ids: &[Uuid]) -> Vec<Uuid> {
    let mut seen = HashSet::with_capacity(ids.len());
    ids.iter().copied().filter(|id| seen.insert(*id)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn optional_text_preserves_content_and_normalizes_absence() {
        assert_eq!(optional_text(None), None);
        assert_eq!(optional_text(Some(" \t\n\u{2003}".into())), None);
        assert_eq!(
            optional_text(Some("  Équipe  OPS \n".into())),
            Some("Équipe  OPS".into())
        );
    }

    #[test]
    fn unique_ids_preserves_order_and_leaves_validation_to_the_caller() {
        let first = Uuid::from_u128(2);
        let second = Uuid::from_u128(1);
        assert_eq!(
            unique_ids(&[first, Uuid::nil(), second, first, Uuid::nil()]),
            vec![first, Uuid::nil(), second]
        );
        assert!(unique_ids(&[]).is_empty());
    }
}
