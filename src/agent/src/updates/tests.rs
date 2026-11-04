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
