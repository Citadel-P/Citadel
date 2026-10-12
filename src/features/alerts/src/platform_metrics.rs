//! Threshold/version observations are independent of the persistence transport.
use crate::{AlertError, AlertEventSink, AlertObservation};
use futures_util::future::BoxFuture;

pub struct PlatformMetricSample {
    pub name: String,
    pub agent_version: Option<String>,
    pub cpu_usage: Option<f64>,
    pub memory_usage: Option<f64>,
    pub disk_usage: Option<f64>,
    pub disk_used_bytes: Option<i64>,
    pub disk_total_bytes: Option<i64>,
}

/// A flush serializes consumers and acknowledges only the exact sample revisions
/// successfully observed. Cancellation/failure must leave unacknowledged rows retryable.
pub trait PlatformAlertBatchStore: Send + Sync {
    fn flush_pending<'a>(
        &'a self,
        alerts: &'a dyn AlertEventSink,
        batch_size: i64,
    ) -> BoxFuture<'a, Result<usize, AlertError>>;
}

pub async fn observe_platform_metrics(
    alerts: &dyn AlertEventSink,
    platform_id: uuid::Uuid,
    sample: &PlatformMetricSample,
    compatibility: &str,
) -> Result<(), AlertError> {
    let name = &sample.name;
    let version = sample.agent_version.as_deref().unwrap_or_default();
    if !version.is_empty() {
        let compatible = version
            .strip_prefix(compatibility)
            .is_some_and(|rest| rest.is_empty() || rest.starts_with(['.', '-', '+']));
        let observation = AlertObservation {
            alert_type: "PlatformVersionMismatch".into(),
            info: serde_json::json!({"PlatformName":name,"AgentVersion":version,"CoreVersion":compatibility,"HumanMessage":format!("Platform '{name}' Agent version {version} differs from Core compatibility version {compatibility}.")}),
            resource_id: platform_id,
            resource_name: name.to_owned(),
            resource_type: "Platform".into(),
            deduplication_component: "version".into(),
            observed_at: chrono::Utc::now(),
            value: None,
            matched: !compatible,
        };
        alerts.observe(&observation).await?;
    }
    let values = [
        ("PlatformCpuHigh", "CPU", sample.cpu_usage),
        ("PlatformRamHigh", "RAM", sample.memory_usage),
        ("PlatformDiskHigh", "disk", sample.disk_usage),
    ];
    for (alert_type, label, value) in values {
        let Some(value) = value.filter(|value| value.is_finite() && (0.0..=100.0).contains(value))
        else {
            continue;
        };
        let observation = AlertObservation {
            alert_type: alert_type.to_owned(),
            info: serde_json::json!({
                "PlatformName": name,
                "Value": value,
                "DiskUsagePercent": if alert_type == "PlatformDiskHigh" { Some(value) } else { None },
                "DiskUsedBytes": sample.disk_used_bytes,
                "DiskTotalBytes": sample.disk_total_bytes,
                "HumanMessage": format!("Platform '{name}' {label} usage is {value:.1}%."),
            }),
            resource_id: platform_id,
            resource_name: name.to_owned(),
            resource_type: "Platform".to_owned(),
            deduplication_component: "utilization".to_owned(),
            observed_at: chrono::Utc::now(),
            value: Some(value),
            matched: true,
        };
        alerts.observe(&observation).await?;
    }
    Ok(())
}
