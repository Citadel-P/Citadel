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
        containers: Default::default(),
        payload: json!({"dockerNodeId":"worker","stats":[{"dockerContainerId":"same-docker-id","cpuUsage":42}]}),
    };
    let values = map_stats(&event, &[manager.clone(), worker.clone(), other.clone()]);
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
        containers: Default::default(),
        payload: json!({"stats":[{"dockerContainerId":"same-docker-id","cpuUsage":12}]}),
    };
    let values = map_stats(&event, &[worker.clone(), manager.clone()]);
    assert_eq!(values.len(), 1);
    assert_eq!(values[0]["containerId"], json!(manager.id));
    assert!(container_data(worker, Some(&event))["containerStat"].is_null());
}
