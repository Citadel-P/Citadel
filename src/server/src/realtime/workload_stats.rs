//! A physical sample index is shared for one collector event. Membership belongs
//! to the authorized subscription and is replaced on workload/inventory changes.
use super::{PublishedRuntimeEvent, RealtimeReadError};
use crate::realtime_groups::ClientEvent;
use citadel_platforms::MetricMembership;
use serde_json::{Value, json};

pub(super) async fn samples(
    group: &str,
    membership: &MetricMembership,
    event: &PublishedRuntimeEvent,
) -> Result<Option<ClientEvent>, RealtimeReadError> {
    let Some(batch) = event.payload["stats"].as_array() else {
        return Ok(None);
    };
    if membership.platform != event.platform_id || membership.members.is_empty() {
        return Ok(None);
    }
    let positions = event
        .reads
        .sample_positions
        .get_or_try_init(|| async {
            Ok(batch
                .iter()
                .enumerate()
                .filter_map(|(i, s)| Some((s["dockerContainerId"].as_str()?.to_owned(), i)))
                .collect())
        })
        .await?;
    let node = event.payload["dockerNodeId"].as_str();
    let rows: Vec<Value> = membership
        .members
        .iter()
        .filter_map(|((member_node, docker), id)| {
            if member_node.as_deref() != node {
                return None;
            }
            let mut sample = batch[*positions.get(docker)?].as_object()?.clone();
            sample.remove("dockerContainerId");
            sample.insert("containerId".into(), json!(id));
            Some(Value::Object(sample))
        })
        .collect();
    Ok((!rows.is_empty()).then(|| {
        ClientEvent::new(
            "WorkloadStatsUpdated",
            vec![json!({"group":group,"stats":rows})],
        )
    }))
}
