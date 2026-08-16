#![forbid(unsafe_code)]

use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ActorId(Uuid);

impl ActorId {
    #[must_use]
    pub const fn new(value: Uuid) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn value(self) -> Uuid {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlatformSummary {
    pub id: Uuid,
    pub name: String,
    pub address: String,
    pub status: String,
    pub connector_type: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimePlatformInfo {
    pub daemon_id: String,
    pub operating_system: String,
    pub os_type: String,
    pub architecture: String,
    pub cpu_count: i64,
    pub memory_total: i64,
    pub container_count: i64,
    pub containers_running: i64,
    pub containers_paused: i64,
    pub containers_stopped: i64,
    pub api_version: String,
    pub minimum_api_version: String,
    pub agent_version: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeContainerSummary {
    pub id: String,
    pub name: String,
    pub image: String,
    pub state: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimePlatformStats {
    pub memory_usage: f64,
    pub cpu_usage: f64,
    pub receive_bytes: f64,
    pub transmit_bytes: f64,
    pub container_count: i64,
    pub containers_running: i64,
    pub containers_paused: i64,
    pub containers_stopped: i64,
}
