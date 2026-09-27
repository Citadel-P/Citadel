use super::*;

#[test]
fn container_actions_have_one_shared_semantic_mapping() {
    for (action, expected) in [
        ("create", Some(ContainerChange::Created)),
        ("start", Some(ContainerChange::Running)),
        ("unpause", Some(ContainerChange::Running)),
        ("pause", Some(ContainerChange::Paused)),
        ("die", Some(ContainerChange::Exited)),
        ("destroy", Some(ContainerChange::Tombstone)),
        ("rename", Some(ContainerChange::Observe)),
        ("health_status: healthy", Some(ContainerChange::Observe)),
        ("exec_create: sh", None),
        ("exec_start: sh", None),
        ("exec_die", None),
        ("attach", None),
        ("top", None),
        ("kill", None),
        ("stop", None),
        ("restart", None),
        ("", None),
    ] {
        assert_eq!(
            classify("container", action, "local"),
            expected.map(RuntimeEventKind::Container),
            "{action}"
        );
    }
}

#[test]
fn resource_events_preserve_deletion_and_swarm_scope() {
    use ResourceChange::*;
    use RuntimeEventKind::*;
    for (resource, action, scope, expected) in [
        ("image", "pull", "local", Some(Image(Observe))),
        ("image", "create", "local", Some(Image(Observe))),
        ("image", "delete", "local", Some(Image(Tombstone))),
        ("image", "tag", "local", None),
        ("volume", "create", "local", Some(Volume(Observe))),
        ("volume", "destroy", "local", Some(Volume(Tombstone))),
        ("volume", "mount", "local", None),
        ("network", "create", "local", Some(Network(Observe))),
        ("network", "destroy", "local", Some(Network(Tombstone))),
        ("network", "connect", "local", None),
        ("network", "disconnect", "local", None),
        (
            "network",
            "update",
            "swarm",
            Some(SwarmDirty(SwarmResource::Network)),
        ),
        (
            "network",
            "disconnect",
            "swarm",
            Some(SwarmDirty(SwarmResource::Network)),
        ),
        ("network", "create", "swarm", Some(Network(Observe))),
        ("network", "destroy", "swarm", Some(Network(Tombstone))),
        (
            "service",
            "update",
            "swarm",
            Some(SwarmDirty(SwarmResource::Service)),
        ),
        (
            "task",
            "update",
            "swarm",
            Some(SwarmDirty(SwarmResource::Service)),
        ),
        (
            "node",
            "update",
            "swarm",
            Some(SwarmDirty(SwarmResource::Node)),
        ),
        (
            "secret",
            "remove",
            "swarm",
            Some(SwarmDirty(SwarmResource::Secret)),
        ),
        (
            "config",
            "create",
            "swarm",
            Some(SwarmDirty(SwarmResource::Config)),
        ),
        ("builder", "prune", "local", None),
        ("plugin", "enable", "local", None),
        ("future-resource", "update", "local", Some(Unknown)),
    ] {
        assert_eq!(
            classify(resource, action, scope),
            expected,
            "{resource}/{action}/{scope}"
        );
    }
}

#[test]
fn stop_emits_only_exited_and_restart_finishes_running_without_duplicate_marker() {
    let states = |actions: &[&str]| {
        actions
            .iter()
            .filter_map(|action| classify("container", action, "local"))
            .collect::<Vec<_>>()
    };
    assert_eq!(
        states(&["kill", "die", "stop"]),
        [RuntimeEventKind::Container(ContainerChange::Exited)]
    );
    assert_eq!(
        states(&["kill", "die", "stop", "start", "restart"]),
        [
            RuntimeEventKind::Container(ContainerChange::Exited),
            RuntimeEventKind::Container(ContainerChange::Running)
        ]
    );
}

#[test]
fn legacy_agent_payloads_are_normalized_at_the_core_boundary() {
    use crate::connectors::agent::client::decode_daemon_event;
    use citadel_contracts::citadel::{platforms::v1::*, shared_models::v1::ContainerMessage};
    use prost::Message;
    for action in [
        "kill", "stop", "attach", "top", "exec_die", "restart", "die", "start", "destroy",
    ] {
        let wire = DaemonEventResponse {
            scope: 1,
            kind: Some(daemon_event_response::Kind::DaemonContainerEventResponse(
                DaemonContainerEventResponse {
                    action: action.to_owned(),
                    container_id: "fixture".into(),
                    container: Some(ContainerMessage {
                        id: "fixture".into(),
                        state: 2,
                        ..Default::default()
                    }),
                },
            )),
        };
        let decoded = decode_daemon_event(&wire.encode_to_vec()).unwrap();
        assert_eq!(
            decoded.as_ref().map(|e| e.kind),
            classify("container", action, "local")
        );
        if let Some(event) = decoded {
            if action == "destroy" {
                assert!(event.container.is_none());
                assert!(event.container_state.is_none());
            } else {
                let expected = if action == "die" { "exited" } else { "running" };
                assert_eq!(event.container_state.as_deref(), Some(expected));
                assert_eq!(event.container.unwrap().state, expected);
            }
        }
    }
    for scope in [1, 2] {
        let wire = DaemonEventResponse {
            scope,
            kind: Some(daemon_event_response::Kind::DaemonResourceEventResponse(
                DaemonResourceEventResponse {
                    r#type: 5,
                    action: "disconnect".into(),
                    resource_id: "network".into(),
                },
            )),
        };
        let decoded = decode_daemon_event(&wire.encode_to_vec()).unwrap();
        assert_eq!(
            decoded.map(|e| e.kind),
            if scope == 2 {
                Some(RuntimeEventKind::SwarmDirty(SwarmResource::Network))
            } else {
                None
            }
        );
    }
}
