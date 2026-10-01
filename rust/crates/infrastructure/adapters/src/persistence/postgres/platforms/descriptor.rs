use citadel_platforms::{PlatformDescriptor, PlatformRoutingMetadata};
use serde::Deserialize;
use serde_json::Value;

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StoredRouting {
    #[serde(default, rename = "nodeID", alias = "NodeID")]
    node_id: Option<String>,
    #[serde(default, alias = "DaemonId")]
    daemon_id: Option<String>,
    #[serde(default, alias = "ControlAvailable")]
    control_available: Option<bool>,
    #[serde(default, alias = "LocalNodeState")]
    local_node_state: Option<String>,
    #[serde(default, alias = "Error")]
    error: Option<String>,
}

pub fn decode(metadata: Value) -> Result<PlatformDescriptor, sqlx::Error> {
    let fields: StoredRouting = if metadata.is_null() {
        StoredRouting::default()
    } else {
        if !metadata.is_object() {
            return Err(sqlx::Error::Decode(
                "Platform descriptor must be an object or null.".into(),
            ));
        }
        serde_json::from_value(metadata.clone())
            .map_err(|error| sqlx::Error::Decode(Box::new(error)))?
    };
    Ok(PlatformDescriptor {
        metadata,
        routing: PlatformRoutingMetadata {
            node_id: fields.node_id,
            daemon_id: fields.daemon_id,
            control_available: fields.control_available.unwrap_or(false),
            local_node_state: fields.local_node_state,
            error: fields.error,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn routing_decodes_existing_casing_without_losing_extension_metadata() {
        for raw in [
            json!({"nodeID":"node","daemonId":"daemon","controlAvailable":true,"localNodeState":"active","extension":{"Mixed.Key":1}}),
            json!({"NodeID":"node","DaemonId":"daemon","ControlAvailable":true,"LocalNodeState":"active","extension":{"Mixed.Key":1}}),
        ] {
            let parsed = decode(raw.clone()).unwrap();
            assert_eq!(parsed.routing.node_id.as_deref(), Some("node"));
            assert_eq!(parsed.routing.daemon_id.as_deref(), Some("daemon"));
            assert!(parsed.routing.control_available);
            assert_eq!(serde_json::to_value(parsed).unwrap(), raw);
        }
    }
    #[test]
    fn missing_metadata_is_safe_but_invalid_identity_is_rejected() {
        for raw in [Value::Null, json!({"$type":"Docker"})] {
            let parsed = decode(raw.clone()).unwrap();
            assert_eq!(parsed.routing, PlatformRoutingMetadata::default());
            assert_eq!(serde_json::to_value(parsed).unwrap(), raw);
        }
        for raw in [
            json!({"nodeID":12}),
            json!({"controlAvailable":"yes"}),
            json!({"nodeID":"a","NodeID":"b"}),
            json!([]),
        ] {
            assert!(decode(raw).is_err());
        }
    }
}
