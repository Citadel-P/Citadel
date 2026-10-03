//! An observation is authoritative only for its own runtime identity.
use super::{ProjectionKind, ResourceChange, RuntimeEventKind};
use crate::{RuntimeImageSummary, RuntimeNetworkSummary, RuntimeVolumeSummary};
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ResourceDelta {
    Image {
        id: String,
        value: Option<RuntimeImageSummary>,
    },
    Network {
        id: String,
        value: Option<RuntimeNetworkSummary>,
    },
    Volume {
        id: String,
        value: Option<RuntimeVolumeSummary>,
    },
}
impl ResourceDelta {
    pub fn id(&self) -> &str {
        match self {
            Self::Image { id, .. } | Self::Network { id, .. } | Self::Volume { id, .. } => id,
        }
    }
    pub fn resource_type(&self) -> &'static str {
        match self {
            Self::Image { .. } => "image",
            Self::Network { .. } => "network",
            Self::Volume { .. } => "volume",
        }
    }
    pub fn projection_kind(&self) -> ProjectionKind {
        match self {
            Self::Image { .. } => ProjectionKind::Images,
            Self::Network { .. } => ProjectionKind::Networks,
            Self::Volume { .. } => ProjectionKind::Volumes,
        }
    }
    pub fn valid_for(&self, kind: RuntimeEventKind) -> bool {
        if self.id().is_empty() {
            return false;
        }
        match (self, kind) {
            (Self::Image { id, value }, RuntimeEventKind::Image(change)) => match value {
                Some(v) => change == ResourceChange::Observe && v.id == *id,
                None => change == ResourceChange::Tombstone,
            },
            (Self::Network { id, value }, RuntimeEventKind::Network(change)) => match value {
                Some(v) => change == ResourceChange::Observe && v.id == *id,
                None => change == ResourceChange::Tombstone,
            },
            (Self::Volume { id, value }, RuntimeEventKind::Volume(change)) => match value {
                Some(v) => change == ResourceChange::Observe && v.name == *id,
                None => change == ResourceChange::Tombstone,
            },
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn absent_observations_are_deletions_only_for_explicit_tombstones() {
        let missing = ResourceDelta::Volume {
            id: "data".into(),
            value: None,
        };
        assert!(!missing.valid_for(RuntimeEventKind::Volume(ResourceChange::Observe)));
        assert!(missing.valid_for(RuntimeEventKind::Volume(ResourceChange::Tombstone)));
        let wrong = ResourceDelta::Volume {
            id: "data".into(),
            value: Some(RuntimeVolumeSummary {
                name: "another".into(),
                ..Default::default()
            }),
        };
        assert!(!wrong.valid_for(RuntimeEventKind::Volume(ResourceChange::Observe)));
        assert!(!missing.valid_for(RuntimeEventKind::Network(ResourceChange::Tombstone)));
    }
}
