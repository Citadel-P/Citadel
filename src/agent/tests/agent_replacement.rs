//! Real Docker replacement only; transport authentication is covered separately.
use serde_json::Value;
use std::process::Command;
fn docker(args: &[&str]) -> String {
    let output = Command::new("docker").args(args).output().unwrap();
    assert!(
        output.status.success(),
        "Docker fixture failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap().trim().into()
}
struct Fixture {
    name: String,
    volume: String,
    operation: String,
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let output = Command::new("docker")
            .args([
                "ps",
                "-aq",
                "--filter",
                &format!("label=citadel.test.agent-update={}", self.operation),
            ])
            .output()
            .unwrap();
        for id in String::from_utf8_lossy(&output.stdout).lines() {
            let _ = Command::new("docker").args(["rm", "-f", id]).output();
        }
        let _ = Command::new("docker")
            .args(["volume", "rm", &self.volume])
            .output();
    }
}
#[tokio::test]
#[ignore = "requires disposable Docker fixtures and pre-pulled CITADEL_AGENT_TEST_IMAGE official digest"]
async fn replaces_only_selected_container_preserving_identity_data_and_port() {
    let image = std::env::var("CITADEL_AGENT_TEST_IMAGE").unwrap();
    assert!(image.starts_with("ghcr.io/citadel-p/citadel.agent@sha256:"));
    let operation = uuid::Uuid::now_v7().to_string();
    let fixture = Fixture {
        name: format!("citadel-agent-update-test-{operation}"),
        volume: format!("citadel-agent-update-test-{operation}"),
        operation,
    };
    docker(&["volume", "create", &fixture.volume]);
    docker(&[
        "run",
        "-d",
        "--name",
        &fixture.name,
        "--label",
        &format!("citadel.test.agent-update={}", fixture.operation),
        "--label",
        "com.citadel.system=true",
        "--label",
        "com.citadel.system-role=agent",
        "-e",
        "CITADEL_AGENT_MODE=edge",
        "-e",
        "TEST_SECRET=fixture-only",
        "-v",
        "/var/run/docker.sock:/var/run/docker.sock",
        "-v",
        &format!("{}:/app/data", fixture.volume),
        "-p",
        "127.0.0.1::9000",
        "--restart",
        "always",
        "--entrypoint",
        "/bin/sh",
        &image,
        "-c",
        "exec sleep 600",
    ]);
    let inspect = |name: &str| -> Value {
        serde_json::from_str::<Vec<Value>>(&docker(&["inspect", name]))
            .unwrap()
            .remove(0)
    };
    docker(&[
        "exec",
        &fixture.name,
        "sh",
        "-c",
        "echo preserved >/app/data/identity",
    ]);
    let untouched = docker(&[
        "run",
        "-d",
        "--label",
        &format!("citadel.test.agent-update={}", fixture.operation),
        "--entrypoint",
        "/bin/sh",
        &image,
        "-c",
        "exec sleep 600",
    ]);
    let source = inspect(&fixture.name);
    citadel_agent::updates::replace(
        source["Id"].as_str().unwrap(),
        source["Image"].as_str().unwrap(),
        &image,
        &fixture.operation,
    )
    .await
    .unwrap();
    let target = inspect(&fixture.name);
    assert_ne!(source["Id"], target["Id"]);
    assert_eq!(inspect(&untouched)["State"]["Running"], true);
    assert_eq!(
        target["HostConfig"]["RestartPolicy"],
        source["HostConfig"]["RestartPolicy"]
    );
    assert_eq!(source["Config"]["Env"], target["Config"]["Env"]);
    assert_eq!(target["Config"]["Image"], image);
    assert_eq!(target["State"]["Running"], true);
    assert_eq!(
        source["NetworkSettings"]["Ports"],
        target["NetworkSettings"]["Ports"]
    );
    for mount in source["Mounts"].as_array().unwrap() {
        let replacement = target["Mounts"]
            .as_array()
            .unwrap()
            .iter()
            .find(|m| m["Destination"] == mount["Destination"])
            .unwrap();
        for key in ["Destination", "Source", "Name", "Type", "RW", "Propagation"] {
            assert_eq!(mount[key], replacement[key], "{key}");
        }
        for option in mount["Mode"]
            .as_str()
            .unwrap()
            .split(',')
            .filter(|v| !v.is_empty())
        {
            assert!(
                replacement["Mode"]
                    .as_str()
                    .unwrap()
                    .split(',')
                    .any(|v| v == option)
            );
        }
    }
    assert_eq!(
        inspect(source["Id"].as_str().unwrap())["HostConfig"]["RestartPolicy"]["Name"],
        "no"
    );
    assert_eq!(
        docker(&["exec", &fixture.name, "cat", "/app/data/identity"]),
        "preserved"
    );
    assert_eq!(
        inspect(source["Id"].as_str().unwrap())["State"]["Running"],
        false
    );
}
