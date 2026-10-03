citadel_primitives::status_enum! {
    pub enum AlertEventStatus {
        Active,
        Acknowledged,
        Resolved,
    }
}

impl AlertEventStatus {
    /// Resolution wins even when an event was previously acknowledged.
    pub const fn from_lifecycle(acknowledged: bool, resolved: bool) -> Self {
        match (acknowledged, resolved) {
            (_, true) => Self::Resolved,
            (true, false) => Self::Acknowledged,
            (false, false) => Self::Active,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolution_takes_precedence_over_acknowledgement() {
        for (acknowledged, resolved, expected) in [
            (false, false, AlertEventStatus::Active),
            (true, false, AlertEventStatus::Acknowledged),
            (false, true, AlertEventStatus::Resolved),
            (true, true, AlertEventStatus::Resolved),
        ] {
            assert_eq!(
                AlertEventStatus::from_lifecycle(acknowledged, resolved),
                expected
            );
        }
    }

    #[test]
    fn lifecycle_status_roundtrips_without_normalization() {
        for status in AlertEventStatus::ALL {
            assert_eq!(
                status.as_str().parse::<AlertEventStatus>().unwrap(),
                *status
            );
            let json = serde_json::to_value(status).unwrap();
            assert_eq!(json, status.as_str());
            assert_eq!(
                serde_json::from_value::<AlertEventStatus>(json).unwrap(),
                *status
            );
        }
        for invalid in ["", "active", "Resolved ", "Enabled"] {
            assert!(invalid.parse::<AlertEventStatus>().is_err());
        }
    }
}

citadel_primitives::status_enum! {
    pub enum AlertRuleStatus {
        Enabled,
        Disabled,
    }
}
