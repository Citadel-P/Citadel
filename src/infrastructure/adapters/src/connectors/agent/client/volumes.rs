//! Preserve the cluster-volume fields carried by the Agent contract in Docker shape.
use citadel_contracts::citadel::shared_models::v1::ClusterVolumeMessage;
use serde_json::{Value, json};

pub(super) fn cluster(v: ClusterVolumeMessage) -> Value {
    json!({
        "ID": v.id, "CreatedAt": v.created_at, "UpdatedAt": v.updated_at,
        "Version": v.version.map(|v| json!({"Index": v.index})),
        "Spec": v.spec.map(|v| json!({
            "Group": v.group,
            "AccessMode": v.access_mode.map(|a| json!({
                "Scope": match a.scope { 1 => "multi", _ => "single" },
                "Sharing": match a.sharing { 1 => "readonly", 2 => "onewriter", 3 => "all", _ => "none" },
                "Availability": a.availability,
                "Secrets": a.secrets.into_iter().map(|s| json!({"Key": s.key, "Secret": s.secret})).collect::<Vec<_>>(),
                "CapacityRange": a.capacity_range.map(|c| json!({"RequiredBytes": c.required_bytes, "LimitBytes": c.limit_bytes})),
            })),
        })),
        "Info": v.info.map(|i| json!({
            "CapacityBytes": i.capacity_bytes, "VolumeContext": i.volume_context, "VolumeID": i.volume_id,
            "AccessibleTopology": i.accessible_topology.into_iter().map(|t| t.labels).collect::<Vec<_>>(),
        })),
        "PublishStatus": v.publish_status.into_iter().map(|p| json!({
            "NodeID": p.node_id, "State": p.state, "PublishContext": p.publish_context,
        })).collect::<Vec<_>>(),
    })
}
