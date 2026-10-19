use citadel_docker::client::events::{RuntimeEventKind, SwarmResource, classify};
#[test]
fn legacy_agent_payloads_are_normalized_at_the_core_boundary() {
    use citadel_adapters::connectors::agent::client::decode_daemon_event;
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
