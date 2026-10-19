use super::*;

/// Network explorer evidence travels through the same signed Direct/Edge inspect
/// capability as ordinary container details. All Docker writes target fixture DIND.
pub(super) async fn exercise(f: &Fixture, platform: &str, daemon: &str, profile: &str) {
    let daemon = f.container(daemon);
    let proxy = docker(&[
        "exec",
        &daemon,
        "docker",
        "network",
        "create",
        "explorer-proxy",
    ])
    .await;
    let backend = docker(&[
        "exec",
        &daemon,
        "docker",
        "network",
        "create",
        "explorer-backend",
    ])
    .await;
    for (name, mode) in [
        ("explorer-app", "explorer-proxy"),
        ("explorer-db", "explorer-backend"),
        ("explorer-host", "host"),
        ("explorer-none", "none"),
    ] {
        docker(&[
            "exec",
            &daemon,
            "docker",
            "run",
            "-d",
            "--name",
            name,
            "--network",
            mode,
            "alpine:3.24",
            "sleep",
            "1200",
        ])
        .await;
    }
    docker(&[
        "exec",
        &daemon,
        "docker",
        "network",
        "connect",
        "--alias",
        "application",
        "explorer-backend",
        "explorer-app",
    ])
    .await;
    docker(&[
        "exec",
        &daemon,
        "docker",
        "run",
        "-d",
        "--name",
        "explorer-shared",
        "--network",
        "container:explorer-app",
        "alpine:3.24",
        "sleep",
        "1200",
    ])
    .await;
    let catalog_path = format!("/api/v1/platforms/{platform}/network-containers");
    let catalog = f
        .wait(&catalog_path, |v| {
            v["containers"].as_array().is_some_and(|items| {
                items
                    .iter()
                    .filter(|item| {
                        item["containerName"]
                            .as_str()
                            .is_some_and(|name| name.contains("explorer-"))
                    })
                    .count()
                    == 5
            })
        })
        .await;
    let find = |name: &str| {
        catalog["containers"]
            .as_array()
            .unwrap()
            .iter()
            .find(|item| {
                item["containerName"]
                    .as_str()
                    .is_some_and(|value| value.trim_start_matches('/') == name)
            })
            .unwrap()["containerId"]
            .as_str()
            .unwrap()
            .to_owned()
    };
    let app = find("explorer-app");
    let listed = f
        .api(Method::GET, &format!("/api/v1/networks/{platform}"), None)
        .await;
    for id in [&proxy, &backend] {
        let network = listed["networks"]
            .as_array()
            .unwrap()
            .iter()
            .find(|network| network["id"] == *id)
            .unwrap();
        assert_list_evidence(network, &app);
    }
    let path = format!("/api/v1/containers/{app}/networking");
    let details = f.api(Method::GET, &path, None).await;
    let endpoints = details["endpoints"].as_array().unwrap();
    assert_eq!(endpoints.len(), 2, "{details}");
    assert!(endpoints.iter().any(|e| e["networkId"] == proxy));
    assert!(endpoints.iter().any(|e| {
        e["networkId"] == backend
            && e["aliases"]
                .as_array()
                .unwrap()
                .contains(&json!("application"))
    }));
    assert!(details.get("config").is_none());
    for (name, mode) in [
        ("explorer-host", "host"),
        ("explorer-none", "none"),
        ("explorer-shared", "shared"),
    ] {
        let details = f
            .api(
                Method::GET,
                &format!("/api/v1/containers/{}/networking", find(name)),
                None,
            )
            .await;
        assert_eq!(details["mode"], mode, "{details}");
    }
    let observed = f
        .api(
            Method::GET,
            &format!("/api/v1/networks/{platform}/{backend}"),
            None,
        )
        .await;
    assert_eq!(
        observed["usage"]["entries"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|entry| entry["relationship"] == "Attachment")
            .count(),
        2,
        "{observed}"
    );
    docker(&[
        "exec",
        &daemon,
        "docker",
        "network",
        "disconnect",
        "explorer-backend",
        "explorer-app",
    ])
    .await;
    let detached = f.api(Method::GET, &path, None).await;
    assert_eq!(detached["endpoints"].as_array().unwrap().len(), 1);
    for name in [
        "explorer-shared",
        "explorer-app",
        "explorer-db",
        "explorer-host",
        "explorer-none",
    ] {
        docker(&["exec", &daemon, "docker", "rm", "-f", name]).await;
    }
    docker(&[
        "exec",
        &daemon,
        "docker",
        "network",
        "rm",
        "explorer-proxy",
        "explorer-backend",
    ])
    .await;
    println!(
        "Networks {profile}: aliases, two-network app, backend-only database, host/none/shared modes, sanitized details and detach passed"
    );
}

fn assert_list_evidence(network: &Value, app: &str) {
    let entries = network["usage"]["entries"].as_array().unwrap();
    assert!(
        entries
            .iter()
            .any(|entry| { entry["containerId"] == app && entry["relationship"] == "Attachment" }),
        "network list lost endpoint evidence: {network}"
    );
}

#[test]
fn list_evidence_requires_current_endpoint_metadata() {
    let incomplete = json!({"containers":{}, "inUse":true,
        "usage":{"entries":[], "isPartial":true}});
    assert!(std::panic::catch_unwind(|| assert_list_evidence(&incomplete, "app")).is_err());
    let current = json!({"containers":{"app":{}}, "inUse":true,
        "usage":{"entries":[{"containerId":"app", "relationship":"Attachment"}], "isPartial":false}});
    assert_list_evidence(&current, "app");
    assert!(std::panic::catch_unwind(|| assert_list_evidence(&current, "other")).is_err());
}
