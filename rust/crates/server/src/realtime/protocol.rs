use super::RealtimeError;
use crate::api::resources::platforms::views::{ContainerView, PlatformView};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ClientMessage {
    pub(super) protocol_version: u16,
    pub(super) kind: String,
    pub(super) access_token: Option<String>,
    pub(super) resource_type: Option<String>,
    pub(super) resource_id: Option<Uuid>,
    pub(super) last_sequence: Option<u64>,
    pub(super) last_resource_revision: Option<u64>,
    pub(super) client_mode: Option<String>,
    pub(super) invocation_id: Option<String>,
    pub(super) target: Option<String>,
    pub(super) arguments: Option<Vec<Value>>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct RealtimeEnvelope<'a> {
    pub(super) protocol_version: u16,
    pub(super) connection_id: Uuid,
    pub(super) sequence: u64,
    pub(super) resource_type: &'a str,
    pub(super) resource_id: Uuid,
    pub(super) resource_revision: u64,
    pub(super) event_kind: &'a str,
    pub(super) payload_schema_version: u16,
    pub(super) payload: &'a Value,
}

pub(super) struct RealtimeEventRef<'a> {
    pub(super) resource_revision: u64,
    pub(super) resource_type: &'a str,
    pub(super) resource_id: Uuid,
    pub(super) event_kind: &'a str,
    pub(super) payload: &'a Value,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct PlatformSnapshot {
    pub(super) platform: PlatformView,
    pub(super) containers: Vec<ContainerView>,
}

pub(super) fn parse_client_message(text: &str) -> Result<ClientMessage, RealtimeError> {
    serde_json::from_str(text).map_err(|error| RealtimeError::InvalidMessage(error.to_string()))
}

pub(super) struct Invocation {
    pub(super) id: String,
    pub(super) target: Option<String>,
    pub(super) arguments: Option<Vec<Value>>,
}

pub(super) fn parse_invocation(text: &str) -> Result<Invocation, RealtimeError> {
    let request = parse_client_message(text)?;
    if request.protocol_version != super::REALTIME_PROTOCOL_VERSION || request.kind != "invoke" {
        return Err(RealtimeError::InvalidMessage(
            "Expected a versioned invocation".into(),
        ));
    }
    let id = request
        .invocation_id
        .filter(|id| !id.is_empty() && id.len() <= 64)
        .ok_or_else(|| RealtimeError::InvalidMessage("Invalid invocation ID".into()))?;
    Ok(Invocation {
        id,
        target: request.target,
        arguments: request.arguments,
    })
}
