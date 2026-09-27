use super::*;
use crate::api::resources::platforms::views::ContainerView;

#[test]
fn build_log_events_only_target_the_matching_run_group() {
    let run = Uuid::now_v7();
    let event = PublishedRuntimeEvent {
        platform_id: None,
        resource_type: "Build",
        resource_id: run,
        event_kind: "buildLogs",
        resource_revision: 1,
        reads: Default::default(),
        containers: Default::default(),
        payload: json!({"entries":[]}),
    };
    assert!(
        Group::parse(&format!("build-run:{run}"))
            .unwrap()
            .affected_by(&event)
    );
    for group in [
        format!("build-run:{}", Uuid::now_v7()),
        format!("build-runs:{run}"),
        "build-projects".into(),
        format!("activity:Build:{run}"),
    ] {
        assert!(
            !Group::parse(&group).unwrap().affected_by(&event),
            "{group}"
        );
    }
}

fn container(node: Option<&str>) -> ContainerView {
    ContainerView {
        image_view: None,
        deployment_view: None,
        id: Uuid::now_v7(),
        platform_id: Uuid::now_v7(),
        container_id: "same-docker-id".into(),
        name: "fixture".into(),
        docker_image_id: "image".into(),
        created: 1,
        state: "Running".into(),
        control_state: "Idle".into(),
        updated: 1,
        stack: None,
        is_system: false,
        system_role: None,
        has_citadel_ownership_labels: false,
        is_swarm_task: true,
        docker_node_id: node.map(str::to_owned),
        node_hostname: None,
        projection_observed_at: None,
        projection_stale_since: None,
        projection_stale_reason: None,
        last_stats: None,
        ports: json!([]),
        deployment_id: None,
        stack_id: None,
        capabilities: None,
    }
}

#[test]
fn statistics_match_both_node_and_docker_identity() {
    let manager = container(None);
    let worker = container(Some("worker"));
    let other = container(Some("other-worker"));
    let event = PublishedRuntimeEvent {
        platform_id: Some(worker.platform_id),
        resource_type: "Platform",
        resource_id: worker.platform_id,
        event_kind: "stats",
        resource_revision: 1,
        reads: Default::default(),
        containers: Default::default(),
        payload: json!({"dockerNodeId":"worker","stats":[{"dockerContainerId":"same-docker-id","cpuUsage":42}]}),
    };
    let values = map_stats(
        &event,
        &[identity(&manager), identity(&worker), identity(&other)],
    );
    assert_eq!(values.len(), 1);
    assert_eq!(values[0]["containerId"], json!(worker.id));
    assert_eq!(
        container_data(worker, Some(&event))["containerStat"]["cpuUsage"],
        42
    );
    assert!(container_data(manager, Some(&event))["containerStat"].is_null());
    assert!(container_data(other, Some(&event))["containerStat"].is_null());
}

#[test]
fn ordinary_manager_statistics_do_not_update_worker_rows() {
    let manager = container(None);
    let worker = container(Some("worker"));
    let event = PublishedRuntimeEvent {
        platform_id: Some(manager.platform_id),
        resource_type: "Platform",
        resource_id: manager.platform_id,
        event_kind: "stats",
        resource_revision: 1,
        reads: Default::default(),
        containers: Default::default(),
        payload: json!({"stats":[{"dockerContainerId":"same-docker-id","cpuUsage":12}]}),
    };
    let values = map_stats(&event, &[identity(&worker), identity(&manager)]);
    assert_eq!(values.len(), 1);
    assert_eq!(values[0]["containerId"], json!(manager.id));
    assert!(container_data(worker, Some(&event))["containerStat"].is_null());
}

#[test]
fn committed_container_state_patches_skip_full_reads_for_container_groups() {
    let platform = Uuid::now_v7();
    let patch = citadel_platforms::containers::ContainerStatePatch {
        id: Uuid::now_v7(),
        platform_id: platform,
        container_id: "docker-123".into(),
        state: Some("Exited".into()),
        control_state: Some("Idle".into()),
        updated: Some(42),
        docker_node_id: None,
    };
    let hub = crate::realtime::RealtimeHub::new(8, Arc::new(crate::metrics::Metrics::default()));
    let mut receiver = hub.subscribe();
    hub.publish_container_state_patches(platform, &[patch.clone()]);
    let event = receiver.try_recv().unwrap();

    for name in [
        format!("containers:{platform}"),
        format!("docker-daemon:{platform}"),
    ] {
        let group = crate::realtime_groups::Group::parse(&name).unwrap();
        assert!(group.affected_by(&event));
        let snapshot =
            ApplicationGroupReader::container_patch_snapshot(&group, Some(&event), Some(platform))
                .unwrap();
        assert!(snapshot.rows.is_empty());
        assert_eq!(snapshot.events.len(), 1);
        assert!(
            snapshot.events[0].arguments[0]
                .to_string()
                .contains("docker-123")
        );
    }

    let group =
        crate::realtime_groups::Group::parse(&format!("deployment:{}", Uuid::now_v7())).unwrap();
    assert!(
        ApplicationGroupReader::container_patch_snapshot(&group, Some(&event), Some(platform))
            .is_none()
    );
}

fn identity(c: &ContainerView) -> citadel_platforms::ContainerIdentity {
    citadel_platforms::ContainerIdentity {
        id: c.id,
        platform_id: c.platform_id,
        deployment_id: c.deployment_id,
        stack_id: c.stack_id,
        container_id: c.container_id.clone(),
        docker_node_id: c.docker_node_id.clone(),
    }
}

#[test]
fn state_patches_do_not_reload_platform_summaries_or_unrelated_details() {
    let platform = Uuid::now_v7();
    let id = Uuid::now_v7();
    let event = PublishedRuntimeEvent {
        platform_id: Some(platform),
        resource_type: "Platform",
        resource_id: platform,
        event_kind: "runtimeChanged",
        resource_revision: 1,
        reads: Default::default(),
        containers: Default::default(),
        payload: json!({"dockerResourceType":"container","containerPatches":[{"id":id}]}),
    };
    assert!(
        Group::parse(&format!("container-info:{id}"))
            .unwrap()
            .affected_by(&event)
    );
    for group in [
        "platforms".into(),
        "stacks".into(),
        "deployments".into(),
        format!("container-info:{}", Uuid::now_v7()),
        format!("activity:Stack:{}", Uuid::now_v7()),
    ] {
        assert!(
            !Group::parse(&group).unwrap().affected_by(&event),
            "{group}"
        );
    }
}

#[test]
fn platform_stats_use_current_samples_and_preserve_normalization() {
    let context = citadel_platforms::PlatformTelemetryContext {
        cpu_count: 4,
        mem_total: 1024,
        network_count: 1,
        volume_count: 2,
        image_count: 3,
        descriptor: json!({"containerCount":6}),
    };
    let sample = json!({"created":42,"memoryActive":512.0,"cpuUsage":200.0,"rxBytes":100,"txBytes":200,
        "metadata":{"memTotal":2048,"containersRunning":3,"containerCount":6}});
    let value = live_platform_stats(Uuid::nil(), &context, &sample).unwrap();
    assert_eq!(value["stat"]["created"], 42);
    assert_eq!(value["stat"]["cpuUsage"], 50.0);
    assert_eq!(value["stat"]["memoryUsage"], 25.0);
    assert_eq!(value["containersRunning"], 3);
    assert!(live_platform_stats(Uuid::nil(), &context, &Value::Null).is_none());
}
