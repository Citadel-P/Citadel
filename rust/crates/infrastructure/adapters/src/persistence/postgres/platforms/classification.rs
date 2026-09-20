//! Decode the persisted vocabulary once, before selecting a runtime.
use citadel_platforms::{ConnectorKind, PlatformKind};

pub fn connector_kind(value: &str) -> Result<ConnectorKind, sqlx::Error> {
    match value {
        "Local" => Ok(ConnectorKind::Local),
        "Agent" => Ok(ConnectorKind::Agent),
        "EdgeAgent" => Ok(ConnectorKind::EdgeAgent),
        _ => Err(sqlx::Error::Decode(
            format!("Unknown Platform connector: {value}").into(),
        )),
    }
}

pub fn platform_kind(value: &str) -> Result<PlatformKind, sqlx::Error> {
    match value {
        // Both spellings exist in persisted standalone descriptors.
        "Docker" | "DockerStandalone" => Ok(PlatformKind::Docker),
        "DockerSwarm" => Ok(PlatformKind::DockerSwarm),
        _ => Err(sqlx::Error::Decode(
            format!("Unknown Platform kind: {value}").into(),
        )),
    }
}

pub const fn connector_value(kind: ConnectorKind) -> &'static str {
    match kind {
        ConnectorKind::Local => "Local",
        ConnectorKind::Agent => "Agent",
        ConnectorKind::EdgeAgent => "EdgeAgent",
    }
}

pub const fn platform_value(kind: PlatformKind) -> &'static str {
    match kind {
        PlatformKind::Docker => "Docker",
        PlatformKind::DockerSwarm => "DockerSwarm",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn persisted_classifications_round_trip_and_reject_unknown_values() {
        for kind in [
            ConnectorKind::Local,
            ConnectorKind::Agent,
            ConnectorKind::EdgeAgent,
        ] {
            assert_eq!(connector_kind(connector_value(kind)).unwrap(), kind);
        }
        for kind in [PlatformKind::Docker, PlatformKind::DockerSwarm] {
            assert_eq!(platform_kind(platform_value(kind)).unwrap(), kind);
        }
        assert_eq!(
            platform_kind("DockerStandalone").unwrap(),
            PlatformKind::Docker
        );
        for value in ["", "Unknown", "agent", " Agent"] {
            assert!(connector_kind(value).is_err());
        }
        for value in ["", "Kubernetes", "dockerswarm", " Docker"] {
            assert!(platform_kind(value).is_err());
        }
    }
}
