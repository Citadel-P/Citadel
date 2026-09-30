citadel_primitives::status_enum! {
    pub enum DeploymentStatus {
        Unknown,
        Created,
        Pending,
        Applying,
        Healthy,
        Degraded,
        Failed,
        Stopped,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deployment_states_roundtrip_and_reject_external_container_states() {
        for status in DeploymentStatus::ALL {
            assert_eq!(
                status.as_str().parse::<DeploymentStatus>().unwrap(),
                *status
            );
            assert_eq!(serde_json::to_value(status).unwrap(), status.as_str());
        }
        for invalid in ["running", "Exited", "healthy", "Healthy ", "Invalid"] {
            assert!(invalid.parse::<DeploymentStatus>().is_err());
        }
    }
}
