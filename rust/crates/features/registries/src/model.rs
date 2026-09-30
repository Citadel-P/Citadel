use super::*;

citadel_primitives::status_enum! {
    pub enum RegistryStatus { Active, Disabled, Deprecated }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistryDetails {
    pub id: Uuid,
    #[serde(flatten)]
    pub audit: citadel_primitives::AuditMetadata,
    pub name: String,
    pub status: RegistryStatus,
    pub description: Option<String>,
    pub registry_host: String,
    #[serde(rename = "type")]
    pub registry_type: String,
    #[serde(skip_serializing)]
    pub configuration: Value,
    pub tags: Vec<TagSummary>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_status_storage_and_wire_names_stay_identical() {
        for status in RegistryStatus::ALL {
            assert_eq!(status.as_str().parse::<RegistryStatus>().unwrap(), *status);
            let wire = serde_json::to_value(status).unwrap();
            assert_eq!(wire, status.as_str());
            assert_eq!(
                serde_json::from_value::<RegistryStatus>(wire).unwrap(),
                *status
            );
        }
        for invalid in ["active", "Active ", "Online", ""] {
            assert!(invalid.parse::<RegistryStatus>().is_err());
        }
    }
}
