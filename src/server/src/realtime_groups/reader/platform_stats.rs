use serde_json::{Value, json};
use uuid::Uuid;

pub(super) fn live_platform_stats(
    platform: Uuid,
    context: &citadel_platforms::PlatformTelemetryContext,
    sample: &Value,
    node_id: Option<&str>,
) -> Option<Value> {
    let created = sample["created"].as_i64()?;
    if let Some(node_id) = node_id {
        // The ordinary Platform stream already samples its connected host.
        if context.descriptor["nodeID"].as_str() == Some(node_id) {
            return None;
        }
        let mut counters = sample["containerIo"].as_array()?.clone();
        for counter in &mut counters {
            counter["nodeId"] = json!(node_id);
            counter["created"] = json!(created);
        }
        return Some(json!({
            "platformId":platform, "ioOnly":true,
            "stat":{"created":created,"containerIo":counters,
                "cpuUsage":null,"memoryUsage":null,"rxBytes":null,"txBytes":null,
                "diskUsedBytes":null,"diskTotalBytes":null,"diskUsage":null},
            "ioNodeId":node_id,
        }));
    }
    let metadata = &sample["metadata"];
    let mem_total = metadata["memTotal"].as_i64().unwrap_or(context.mem_total);
    let cpu = sample["cpuUsage"].as_f64().unwrap_or_default();
    let memory = sample["memoryActive"].as_f64().unwrap_or_default();
    Some(json!({
        "platformId":platform,
        "stat": { "created":created, "cpuUsage":if context.cpu_count > 0 {cpu/context.cpu_count as f64} else {cpu},
            "memoryUsage":if mem_total > 0 {memory/mem_total as f64*100.0} else {0.0},
            "rxBytes":sample["rxBytes"], "txBytes":sample["txBytes"], "containerIo":sample["containerIo"],
            "diskUsedBytes":metadata["diskUsedBytes"], "diskTotalBytes":metadata["diskTotalBytes"], "diskUsage":metadata["diskUsage"],
        },
        "memTotal":mem_total,
        "networkCount":metadata["networkCount"].as_i64().unwrap_or(i64::from(context.network_count)),
        "imageCount":metadata["imageCount"].as_i64().unwrap_or(context.image_count),
        "volumeCount":metadata["volumeCount"].as_i64().unwrap_or(i64::from(context.volume_count)),
        "containerCount":metadata.get("containerCount").unwrap_or(&context.descriptor["containerCount"]),
        "containersRunning":metadata.get("containersRunning").unwrap_or(&context.descriptor["containersRunning"]),
        "containersPaused":metadata.get("containersPaused").unwrap_or(&context.descriptor["containersPaused"]),
        "containersStopped":metadata.get("containersStopped").unwrap_or(&context.descriptor["containersStopped"]),
    }))
}
