use super::*;

pub(super) async fn swarm_stack(f: &Fixture, platform: &str, profile: &str) -> Option<String> {
    if std::env::var("CITADEL_ACCEPTANCE_BASELINE_COMPONENT").is_ok() {
        return None;
    }
    let name = format!("acceptance-health-{profile}");
    let compose = "version: '3.8'\nservices:\n  checked:\n    image: alpine:3.24\n    command: ['sleep', '1200']\n    healthcheck:\n      test: ['CMD', 'test', '-d', '/tmp']\n      interval: 1s\n      timeout: 1s\n      retries: 1\n    deploy:\n      replicas: 3\n      placement:\n        max_replicas_per_node: 1\n    stop_grace_period: 1s\n";
    let created = f
        .api(
            Method::POST,
            "/api/v1/stacks",
            Some(json!({
                "name":name, "platformId":platform, "stackSource":"WebEditor", "tagIds":[],
                "spec":{"$type":"WebEditor", "composeFile":compose, "updateBehavior":"Disabled",
                    "projectName":name, "destroyBeforeDeploy":false, "buildImageBindings":[]}
            })),
        )
        .await;
    let id = created["id"].as_str().unwrap().to_owned();
    let result = f
        .api(
            Method::POST,
            "/api/v1/stacks/apply",
            Some(json!({"id":id,"recreate":false})),
        )
        .await;
    assert_eq!(
        result.as_array().unwrap().last().unwrap()["exitCode"],
        0,
        "{result}"
    );
    let assessed = f
        .wait(&format!("/api/v1/stacks/{id}"), |v| {
            v["probeHealth"]["status"] == "Healthy" && v["probeHealth"]["counts"]["healthy"] == 3
        })
        .await;
    let nodes: std::collections::HashSet<_> = assessed["probeHealth"]["members"]
        .as_array()
        .unwrap()
        .iter()
        .map(|member| member["nodeId"].as_str().unwrap())
        .collect();
    assert_eq!(
        nodes.len(),
        3,
        "health evidence must come from the three owning nodes"
    );
    println!(
        "Health {profile}: all three owning nodes, including the manager, have fresh passing checks"
    );
    Some(id)
}

/// Exercise persisted parent assessments with real Docker probes over both Agent transports.
pub(super) async fn exercise(f: &Fixture, platform: &str, daemon: &str, profile: &str) {
    if std::env::var("CITADEL_ACCEPTANCE_BASELINE_COMPONENT").is_ok() {
        // Cross-version transport acceptance does not require the newer M2b assessment contract.
        return;
    }
    let name = format!("acceptance-health-{profile}");
    let compose = "services:\n  checked:\n    image: alpine:3.24\n    command: ['sleep', '1200']\n    healthcheck:\n      test: ['CMD-SHELL', 'test ! -e /tmp/unhealthy']\n      interval: 1s\n      timeout: 1s\n      retries: 1\n    stop_grace_period: 1s\n  unchecked:\n    image: alpine:3.24\n    command: ['sleep', '1200']\n    stop_grace_period: 1s\n";
    let created = f
        .api(
            Method::POST,
            "/api/v1/stacks",
            Some(json!({
                "name":name, "platformId":platform, "stackSource":"WebEditor", "tagIds":[],
                "spec":{"$type":"WebEditor", "composeFile":compose, "updateBehavior":"Disabled",
                    "projectName":name, "destroyBeforeDeploy":true, "buildImageBindings":[]}
            })),
        )
        .await;
    let id = created["id"].as_str().unwrap();
    let result = f
        .api(
            Method::POST,
            "/api/v1/stacks/apply",
            Some(json!({"id":id,"recreate":false})),
        )
        .await;
    assert_eq!(
        result.as_array().unwrap().last().unwrap()["exitCode"],
        0,
        "{result}"
    );
    let path = format!("/api/v1/stacks/{id}");
    let stack = f
        .wait(&path, |v| {
            v["probeHealth"]["counts"]["healthy"] == 1 && v["probeHealth"]["counts"]["noCheck"] == 1
        })
        .await;
    assert_ne!(
        stack["probeHealth"]["status"], "Healthy",
        "unchecked members cannot imply all-green"
    );
    let checked = stack["probeHealth"]["members"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["status"] == "Healthy")
        .unwrap();
    let docker_id = checked["dockerContainerId"].as_str().unwrap();
    let container_id = checked["containerId"].as_str().unwrap();
    let initial = docker(&["exec", &f.container(daemon), "docker", "inspect", docker_id]).await;
    let initial: Value = serde_json::from_str(&initial).unwrap();
    docker(&[
        "exec",
        &f.container(daemon),
        "docker",
        "exec",
        docker_id,
        "touch",
        "/tmp/unhealthy",
    ])
    .await;
    let failed = f
        .wait(&path, |v| v["probeHealth"]["status"] == "Unhealthy")
        .await;
    assert!(
        failed["probeHealth"]["attention"]
            .as_array()
            .unwrap()
            .contains(&json!("Unhealthy"))
    );
    assert_eq!(failed["probeHealth"]["counts"]["noCheck"], 1);
    docker(&[
        "exec",
        &f.container(daemon),
        "docker",
        "exec",
        docker_id,
        "rm",
        "/tmp/unhealthy",
    ])
    .await;
    f.wait(&path, |v| {
        v["probeHealth"]["counts"]["healthy"] == 1 && v["probeHealth"]["counts"]["unhealthy"] == 0
    })
    .await;
    let history = f
        .wait(
            &format!("/api/v1/containers/{container_id}/health-history"),
            |v| {
                v.as_array().is_some_and(|entries| {
                    entries.iter().any(|e| e["health"]["status"] == "Unhealthy")
                        && entries.iter().any(|e| e["health"]["status"] == "Healthy")
                })
            },
        )
        .await;
    assert!(
        !history.to_string().contains("CMD-SHELL"),
        "history must omit commands/output"
    );
    let after = docker(&["exec", &f.container(daemon), "docker", "inspect", docker_id]).await;
    let after: Value = serde_json::from_str(&after).unwrap();
    assert_eq!(
        initial[0]["State"]["StartedAt"], after[0]["State"]["StartedAt"],
        "observing unhealthy must not restart"
    );
    assert_eq!(initial[0]["RestartCount"], after[0]["RestartCount"]);
    assert_eq!(after[0]["State"]["Running"], true);
    f.api(Method::POST, "/api/v1/stacks/stop", Some(json!([id])))
        .await;
    f.wait(&path, |v| v["probeHealth"]["status"] == "NotRunning")
        .await;
    f.api(Method::DELETE, "/api/v1/stacks", Some(json!([id])))
        .await;
    println!(
        "Health {profile}: healthy/failing/recovered parent assessment, unchecked coverage, retained history, intentional stop and no automatic restart passed"
    );
}

/// Released Agents without health metadata must not manufacture passing or absent checks.
pub(super) async fn released_agent(f: &Fixture, platform: &str, daemon: &str, profile: &str) {
    // Enable only for a published baseline known to predate the additive health contract.
    // Future released baselines may legitimately support health observations.
    if std::env::var("CITADEL_ACCEPTANCE_BASELINE_HEALTH").as_deref() != Ok("unknown") {
        return;
    }
    let name = format!("acceptance-old-health-{profile}");
    let compose = "services:\n  checked:\n    image: alpine:3.24\n    command: ['sleep', '1200']\n    healthcheck:\n      test: ['CMD', 'test', '-d', '/tmp']\n      interval: 1s\n      timeout: 1s\n      retries: 1\n    stop_grace_period: 1s\n  unchecked:\n    image: alpine:3.24\n    command: ['sleep', '1200']\n    stop_grace_period: 1s\n";
    let created = f
        .api(
            Method::POST,
            "/api/v1/stacks",
            Some(json!({
                "name":name, "platformId":platform, "stackSource":"WebEditor", "tagIds":[],
                "spec":{"$type":"WebEditor", "composeFile":compose, "updateBehavior":"Disabled",
                    "projectName":name, "destroyBeforeDeploy":true, "buildImageBindings":[]}
            })),
        )
        .await;
    let id = created["id"].as_str().unwrap();
    let result = f
        .api(
            Method::POST,
            "/api/v1/stacks/apply",
            Some(json!({"id":id,"recreate":false})),
        )
        .await;
    assert_eq!(
        result.as_array().unwrap().last().unwrap()["exitCode"],
        0,
        "{result}"
    );
    let inventory = f
        .wait(&format!("/api/v1/platforms/{platform}/containers"), |v| {
            v["containers"].as_array().is_some_and(|items| {
                items
                    .iter()
                    .filter(|c| c["stackId"] == id && c["state"] == "Running")
                    .count()
                    == 2
            })
        })
        .await;
    let members: Vec<_> = inventory["containers"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|c| c["stackId"] == id)
        .collect();
    let mut checked = 0;
    for container in members {
        assert_eq!(container["health"]["status"], "Unknown", "{container}");
        assert!(
            container["health"]["observedAt"].is_null(),
            "legacy metadata must not gain a fresh observation timestamp"
        );
        let inspected = docker(&[
            "exec",
            &f.container(daemon),
            "docker",
            "inspect",
            container["containerId"]
                .as_str()
                .expect("inventory Docker container ID"),
        ])
        .await;
        let inspected: Value = serde_json::from_str(&inspected).unwrap();
        if !inspected[0]["Config"]["Healthcheck"].is_null() {
            checked += 1;
        }
        let container_id = container["id"].as_str().unwrap();
        let details = f
            .api(
                Method::GET,
                &format!("/api/v1/containers/{container_id}/inspect"),
                None,
            )
            .await;
        assert_ne!(details["healthMetadataObserved"], true, "{details}");
        let history = f
            .api(
                Method::GET,
                &format!("/api/v1/containers/{container_id}/health-history"),
                None,
            )
            .await;
        assert!(
            history
                .as_array()
                .unwrap()
                .iter()
                .all(|entry| entry["origin"] != "Sample" && entry["health"]["status"] != "Healthy"),
            "{history}"
        );
    }
    assert_eq!(
        checked, 1,
        "fixture must include a configured and an unchecked container"
    );
    let stack = f
        .wait(&format!("/api/v1/stacks/{id}"), |v| {
            v["probeHealth"]["members"]
                .as_array()
                .is_some_and(|m| m.len() == 2)
        })
        .await;
    assert_eq!(stack["probeHealth"]["status"], "Unknown", "{stack}");
    assert_eq!(stack["probeHealth"]["counts"]["unknown"], 2, "{stack}");
    assert_eq!(stack["probeHealth"]["counts"]["healthy"], 0);
    assert_eq!(stack["probeHealth"]["counts"]["noCheck"], 0);
    f.api(Method::DELETE, "/api/v1/stacks", Some(json!([id])))
        .await;
    println!(
        "Released Agent {profile}: checked/unchecked containers and parent remain Unknown; inspection metadata and periodic health samples are absent"
    );
}
