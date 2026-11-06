use super::*;
use serde_json::{Value, json};
fn source() -> Value {
    json!({"Id":"abcdef1234567890","Name":"/citadel-agent","Image":"sha256:old","State":{"Running":true},"Config":{"Hostname":"abcdef123456","Image":"agent:old","Env":["CITADEL_AGENT_MODE=edge","PRIVATE_TOKEN=kept"],"Labels":{"com.citadel.system":"true","com.citadel.system-role":"agent"}},"HostConfig":{"NetworkMode":"bridge","AutoRemove":false,"Binds":[],"PortBindings":{"9000/tcp":[{"HostPort":"9000"}]},"RestartPolicy":{"Name":"always"}},"Mounts":[{"Type":"bind","Source":"/var/run/docker.sock","Destination":"/var/run/docker.sock","RW":true,"Propagation":"rprivate"},{"Type":"volume","Name":"agent-data","Destination":"/app/data","RW":true}]})
}
fn image() -> String {
    format!("ghcr.io/citadel-p/citadel.agent@sha256:{}", "a".repeat(64))
}
#[test]
fn replacement_preserves_identity_storage_environment_and_ports() {
    let source = source();
    let next = replacement_config(&source, &image(), "operation").unwrap();
    assert_eq!(next["Env"], source["Config"]["Env"]);
    assert_eq!(
        next["HostConfig"]["PortBindings"],
        source["HostConfig"]["PortBindings"]
    );
    assert_eq!(
        next["HostConfig"]["RestartPolicy"],
        source["HostConfig"]["RestartPolicy"]
    );
    assert_eq!(next["Hostname"], "");
    assert_eq!(next["Image"], image());
    assert_eq!(next["HostConfig"]["Binds"][1], "agent-data:/app/data:rw");
}
#[test]
fn rejects_other_resources_remote_sockets_and_ephemeral_edge_identity() {
    for change in [0, 1, 2, 3, 4] {
        let mut s = source();
        match change {
            0 => s["Config"]["Labels"]["com.citadel.system-role"] = json!("core"),
            1 => s["HostConfig"]["NetworkMode"] = json!("container:other"),
            2 => s["Mounts"][0]["Source"] = json!("/other.sock"),
            3 => {
                s["Mounts"].as_array_mut().unwrap().pop();
            }
            _ => s["Config"]["Hostname"] = json!("custom"),
        };
        assert!(replacement_config(&s, &image(), "op").is_err());
    }
    for reference in [
        "agent:latest",
        "ghcr.io/citadel-p/citadel@sha256:abc",
        "ghcr.io/citadel-p/citadel.agent:0.1.2",
    ] {
        assert!(validate_image(reference).is_err());
    }
}

#[test]
fn generated_profiles_and_effective_identity_paths_are_checked() {
    for mode in ["edge", "EDGE", "EdGe"] {
        for (role, profile) in [("edge-agent", "edge-agent"), ("agent", "edge-build-agent")] {
            let mut s = source();
            s["Config"]["Labels"]["com.citadel.system-role"] = json!(role);
            s["Config"]["Env"] = json!([
                format!("CITADEL_AGENT_MODE={mode}"),
                format!("CITADEL_EDGE_AGENT_KEY_PATH=/app/data/{profile}.key"),
                format!("CITADEL_EDGE_IDENTITY_PATH=/app/data/{profile}.identity.json")
            ]);
            assert!(replacement_config(&s, &image(), "op").is_ok());
            for key in ["CITADEL_EDGE_AGENT_KEY_PATH", "CITADEL_EDGE_IDENTITY_PATH"] {
                for path in [
                    "/tmp/key",
                    "/app/data/../key",
                    " /app/data/key",
                    "/app/data/sub/key",
                ] {
                    let mut bad = s.clone();
                    bad["Config"]["Env"] = json!([
                        format!("CITADEL_AGENT_MODE={mode}"),
                        format!("{key}={path}")
                    ]);
                    assert!(
                        replacement_config(&bad, &image(), "op").is_err(),
                        "{mode} {key} {path}"
                    );
                }
            }
            s["Mounts"].as_array_mut().unwrap().pop();
            assert!(replacement_config(&s, &image(), "op").is_err());
        }
    }
    let mut direct = source();
    direct["Config"]["Env"] = json!(["CITADEL_AGENT_MODE=direct"]);
    direct["Mounts"].as_array_mut().unwrap().pop();
    assert!(replacement_config(&direct, &image(), "op").is_ok());
    direct["Config"]["Labels"]["com.docker.swarm.service.id"] = json!("service");
    assert!(replacement_config(&direct, &image(), "op").is_err());
}

#[test]
fn standalone_settlement_requires_the_exact_exited_helper_and_replacement() {
    let mut current = source();
    current["Id"] = json!("replacement");
    current["Config"]["Image"] = json!(image());
    current["Config"]["Labels"]["com.citadel.agent-update"] = json!("op");
    let helper = json!({"Name":format!("/{}", helper_name("op", "source")),
        "Config":{"Image":image(), "Entrypoint":["/app/citadel-agent"],
            "Cmd":["replace-agent","source","sha256:old",image(),"op"],
            "Labels":{"com.citadel.system-role":"agent-updater","com.citadel.agent-update":"op"}},
        "State":{"Status":"exited","Running":false,"ExitCode":0}});
    assert!(settled(&current, &helper, "source", "sha256:old", "op").is_ok());
    // Lost start, delayed start, and an unsuccessful helper are never cleanup acknowledgements.
    for state in [
        json!({"Status":"created","Running":false,"ExitCode":0}),
        json!({"Status":"running","Running":true,"ExitCode":0}),
        json!({"Status":"exited","Running":false,"ExitCode":1}),
    ] {
        let mut pending = helper.clone();
        pending["State"] = state;
        assert!(settled(&current, &pending, "source", "sha256:old", "op").is_err());
    }
    assert!(settled(&current, &helper, "source", "sha256:other", "op").is_err());
    assert!(settled(&current, &helper, "source", "sha256:old", "new-op").is_err());
    current["Id"] = json!("source");
    assert!(settled(&current, &helper, "source", "sha256:old", "op").is_err());
}

fn compose_source() -> Value {
    let mut source = source();
    source["HostConfig"]["NetworkMode"] = json!("portal-edge-demo_default");
    source["Mounts"][0]["Source"] = json!("/run/user/1001/docker.sock");
    source["NetworkSettings"] = json!({"Networks": {
        "portal-edge-demo_default": {
            "NetworkID": "a".repeat(64), "Aliases": ["edge-agent", "agent", "abcdef123456"],
            "DNSNames": ["edge-agent", "abcdef123456"], "EndpointID": "old-endpoint",
            "IPAddress": "172.20.0.2", "MacAddress": "02:42:ac:14:00:02",
            "IPAMConfig": null, "GwPriority": 1
        },
        "monitoring": {"NetworkID": "b".repeat(64), "Aliases": ["monitor"], "IPAMConfig": null}
    }});
    source
}

#[test]
fn rootless_helper_uses_only_the_sources_socket_and_preserves_hardening() {
    let source = compose_source();
    let helper = helper_config(&source, &image(), "op").unwrap();
    assert_eq!(
        helper["HostConfig"]["Binds"],
        json!(["/run/user/1001/docker.sock:/var/run/docker.sock:rw"])
    );
    assert_eq!(helper["HostConfig"]["NetworkMode"], "none");
    assert_eq!(helper["HostConfig"]["ReadonlyRootfs"], true);
    assert_eq!(helper["HostConfig"]["CapDrop"], json!(["ALL"]));
    assert_eq!(
        helper["HostConfig"]["SecurityOpt"],
        json!(["no-new-privileges"])
    );
    assert_eq!(
        helper["Cmd"],
        json!([
            "replace-agent",
            source["Id"],
            source["Image"],
            image(),
            "op"
        ])
    );
    assert!(helper.get("Env").is_none());
    let next = replacement_config(&source, &image(), "op").unwrap();
    assert_eq!(next["Env"], source["Config"]["Env"]);
    assert_eq!(
        next["HostConfig"]["Binds"][0],
        "/run/user/1001/docker.sock:/var/run/docker.sock:rw,rprivate"
    );
    assert_eq!(next["HostConfig"]["Binds"][1], "agent-data:/app/data:rw");
}

#[test]
fn compose_networks_pin_ids_keep_aliases_and_do_not_reuse_runtime_addresses() {
    let source = compose_source();
    let next = replacement_config(&source, &image(), "op").unwrap();
    assert_eq!(next["HostConfig"]["NetworkMode"], "a".repeat(64));
    let endpoints = next["NetworkingConfig"]["EndpointsConfig"]
        .as_object()
        .unwrap();
    assert_eq!(endpoints.len(), 2);
    assert_eq!(
        endpoints[&"a".repeat(64)],
        json!({
            "NetworkID": "a".repeat(64), "Aliases": ["edge-agent", "agent"], "GwPriority": 1
        })
    );
    assert_eq!(
        endpoints[&"b".repeat(64)],
        json!({"NetworkID": "b".repeat(64), "Aliases": ["monitor"]})
    );
    let mut by_id = source.clone();
    by_id["HostConfig"]["NetworkMode"] = json!("a".repeat(64));
    assert_eq!(replacement_config(&by_id, &image(), "op").unwrap(), next);
}

#[test]
fn unsupported_socket_and_network_profiles_fail_before_helper_creation() {
    for path in [
        "/run/user/../docker.sock",
        "/run/user//docker.sock",
        "/run/user/1:2/docker.sock",
        "/other.sock",
    ] {
        let mut source = compose_source();
        source["Mounts"][0]["Source"] = json!(path);
        assert!(helper_config(&source, &image(), "op").is_err(), "{path}");
    }
    for change in 0..9 {
        let mut source = compose_source();
        match change {
            0 => source["Mounts"][0]["RW"] = json!(false),
            1 => source["Config"]["Env"]
                .as_array_mut()
                .unwrap()
                .push(json!("DOCKER_HOST=tcp://remote:2375")),
            2 => source["HostConfig"]["NetworkMode"] = json!("container:other"),
            3 => source["HostConfig"]["NetworkMode"] = json!("missing-network"),
            4 => {
                source["NetworkSettings"]["Networks"]["portal-edge-demo_default"]["NetworkID"] =
                    json!("")
            }
            5 => {
                source["NetworkSettings"]["Networks"]["portal-edge-demo_default"]["IPAMConfig"] =
                    json!({"IPv4Address":"172.20.0.2"})
            }
            6 => {
                source["NetworkSettings"]["Networks"]["monitoring"]["Links"] =
                    json!(["legacy:alias"])
            }
            7 => source["Config"]["MacAddress"] = json!("02:42:ac:14:00:02"),
            _ => source["HostConfig"]["NetworkMode"] = json!("bridge"),
        }
        assert!(
            helper_config(&source, &image(), "op").is_err(),
            "change {change}"
        );
    }
}
