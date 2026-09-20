//! Runtime labels are ownership hints. Callers must resolve their owner in the database.
use std::collections::BTreeMap;

use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Owner {
    Deployment(Uuid),
    Stack(Uuid),
}

/// Reject ambiguous/unsupported ownership rather than treating it as unowned.
pub(crate) fn owner(labels: &BTreeMap<String, String>) -> Result<Option<Owner>, &'static str> {
    let invalid = "Workload has invalid or unsupported Citadel ownership labels.";
    let mut normalized = BTreeMap::new();
    let mut insert = |key: &str, value: &str| -> Result<(), &'static str> {
        let key = key.to_ascii_lowercase();
        let Some(key) = key
            .strip_prefix("com.citadel.")
            .or_else(|| key.strip_prefix("x-citadel."))
        else {
            return Ok(());
        };
        if normalized
            .insert(key.to_owned(), value.to_owned())
            .is_some_and(|old| old != value)
        {
            return Err(invalid);
        }
        Ok(())
    };
    for (key, value) in labels {
        insert(key, value)?;
        if key == "#extensions" {
            if let Some(extensions) = value.strip_prefix("map[").and_then(|s| s.strip_suffix(']')) {
                for entry in extensions.split_whitespace() {
                    if let Some((key, value)) = entry.split_once(':') {
                        insert(key, value)?;
                    }
                }
            } else if value.to_ascii_lowercase().contains("citadel.") {
                return Err(invalid);
            }
        }
    }
    if normalized.is_empty() {
        return Ok(None);
    }
    if !normalized
        .get("managed")
        .is_some_and(|s| s.eq_ignore_ascii_case("true"))
        || normalized.contains_key("system")
        || normalized.contains_key("service-id")
    {
        return Err(invalid);
    }
    match (normalized.get("deployment-id"), normalized.get("stack-id")) {
        (Some(id), None) => Uuid::parse_str(id)
            .map(Owner::Deployment)
            .map(Some)
            .map_err(|_| invalid),
        (None, Some(id)) => Uuid::parse_str(id)
            .map(Owner::Stack)
            .map(Some)
            .map_err(|_| invalid),
        _ => Err(invalid),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_current_and_legacy_owners_but_rejects_ambiguous_labels() {
        let id = Uuid::now_v7();
        for prefix in ["com.citadel.", "x-citadel."] {
            let mut labels = BTreeMap::from([
                (format!("{prefix}managed"), "true".into()),
                (format!("{prefix}deployment-id"), id.to_string()),
            ]);
            assert_eq!(owner(&labels), Ok(Some(Owner::Deployment(id))));
            labels.insert(format!("{prefix}stack-id"), id.to_string());
            assert!(owner(&labels).is_err());
        }
        assert_eq!(owner(&BTreeMap::new()), Ok(None));
        assert!(
            owner(&BTreeMap::from([(
                "com.citadel.managed".into(),
                "true".into()
            )]))
            .is_err()
        );
        assert_eq!(
            owner(&BTreeMap::from([(
                "#extensions".into(),
                format!("map[x-citadel.managed:true x-citadel.stack-id:{id}]")
            )])),
            Ok(Some(Owner::Stack(id)))
        );
    }
}
