use super::*;
use futures_util::FutureExt;
use std::sync::Mutex;

#[derive(Default)]
struct Observations(Mutex<Vec<AlertObservation>>);

impl AlertEventSink for Observations {
    fn observe<'a>(
        &'a self,
        observation: &'a AlertObservation,
    ) -> BoxFuture<'a, Result<Option<crate::AlertEvent>, AlertError>> {
        Box::pin(async move {
            self.0.lock().unwrap().push(observation.clone());
            Ok(None)
        })
    }
}

fn sample(version: Option<&str>) -> PlatformMetricSample {
    PlatformMetricSample {
        name: "edge-01".into(),
        agent_version: version.map(str::to_owned),
        cpu_usage: None,
        memory_usage: None,
        disk_usage: None,
        disk_used_bytes: None,
        disk_total_bytes: None,
    }
}

#[test]
fn version_observations_compare_full_releases_and_preserve_display_versions() {
    for (core, agent, mismatch) in [
        ("0.1.1", "0.1.0", true),
        ("0.1.0", "0.1.1", true),
        ("0.1.1", "0.1.10", true),
        ("0.1.1", "0.2.1", true),
        ("0.1.1", "1.1.1", true),
        ("0.1.1", "0.1.1-dev.1", true),
        ("0.1.1-dev.1", "0.1.1", true),
        ("0.1.1-dev.1", "0.1.1-dev.2", true),
        ("0.1.1", "0.1.1", false),
        ("0.1.1-dev.1", "0.1.1-dev.1", false),
        ("0.1.1", "0.1.1+source.abc", false),
        ("0.1.1+source.abc", "0.1.1", false),
        ("0.1.1-dev.1+abc", "0.1.1-dev.1+def", false),
        ("0.1.1+abc", "0.1.0+abc", true),
    ] {
        let sink = Observations::default();
        let platform = uuid::Uuid::now_v7();
        observe_platform_metrics(&sink, platform, &sample(Some(agent)), core)
            .now_or_never()
            .unwrap()
            .unwrap();
        let observations = sink.0.lock().unwrap();
        assert_eq!(observations.len(), 1);
        let observation = &observations[0];
        assert_eq!(observation.matched, mismatch, "{core} / {agent}");
        assert_eq!(observation.alert_type, "PlatformVersionMismatch");
        assert_eq!(observation.resource_id, platform);
        assert_eq!(observation.resource_type, "Platform");
        assert_eq!(observation.info["CoreVersion"], core);
        assert_eq!(observation.info["AgentVersion"], agent);
        assert_eq!(
            observation.info["HumanMessage"],
            format!("Platform 'edge-01' Agent version {agent} differs from Core version {core}.")
        );
    }
}

#[test]
fn matching_upgrade_emits_recovery_for_the_same_condition() {
    let sink = Observations::default();
    let platform = uuid::Uuid::now_v7();
    for agent in ["0.1.0", "0.1.1+source.abc"] {
        observe_platform_metrics(&sink, platform, &sample(Some(agent)), "0.1.1")
            .now_or_never()
            .unwrap()
            .unwrap();
    }
    let observations = sink.0.lock().unwrap();
    assert!(observations[0].matched);
    assert!(!observations[1].matched);
    assert_eq!(observations[0].resource_id, observations[1].resource_id);
    assert_eq!(observations[0].deduplication_component, "version");
    assert_eq!(observations[1].deduplication_component, "version");
}

#[test]
fn missing_agent_version_does_not_emit_mismatch_or_recovery() {
    for version in [None, Some("")] {
        let sink = Observations::default();
        let mut sample = sample(version);
        sample.cpu_usage = Some(95.0);
        observe_platform_metrics(&sink, uuid::Uuid::now_v7(), &sample, "0.1.1")
            .now_or_never()
            .unwrap()
            .unwrap();
        let observations = sink.0.lock().unwrap();
        assert_eq!(observations.len(), 1);
        assert_eq!(observations[0].alert_type, "PlatformCpuHigh");
        assert_eq!(observations[0].value, Some(95.0));
    }
}
