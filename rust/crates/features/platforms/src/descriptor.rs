//! Routing fields are decoded by persistence; original metadata stays lossless.
use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(transparent)]
pub struct PlatformDescriptor {
    pub metadata: Value,
    #[serde(skip)]
    pub routing: PlatformRoutingMetadata,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct PlatformRoutingMetadata {
    pub node_id: Option<String>,
    pub daemon_id: Option<String>,
    pub control_available: bool,
    pub local_node_state: Option<String>,
    pub error: Option<String>,
}
