//! Closed classifications used by inventory collection and runtime routing.

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum ConnectorKind {
    Local,
    Agent,
    EdgeAgent,
}

impl std::fmt::Display for ConnectorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Local => "Local",
            Self::Agent => "Agent",
            Self::EdgeAgent => "EdgeAgent",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum PlatformKind {
    Docker,
    DockerSwarm,
}

impl std::fmt::Display for PlatformKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Docker => "Docker",
            Self::DockerSwarm => "DockerSwarm",
        })
    }
}
