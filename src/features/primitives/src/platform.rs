//! Platform connectivity shared by workload read models.
crate::status_enum! {
    pub enum PlatformStatus {
        Offline,
        Online,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn platform_connectivity_has_exact_storage_and_wire_values() {
        for status in PlatformStatus::ALL {
            assert_eq!(status.as_str().parse::<PlatformStatus>().unwrap(), *status);
            let json = serde_json::to_value(status).unwrap();
            assert_eq!(json, status.as_str());
            assert_eq!(
                serde_json::from_value::<PlatformStatus>(json).unwrap(),
                *status
            );
        }
        for invalid in ["online", "Online ", "Unknown", "running", ""] {
            assert!(invalid.parse::<PlatformStatus>().is_err());
        }
    }
}
