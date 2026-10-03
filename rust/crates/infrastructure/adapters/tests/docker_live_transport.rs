#![cfg(unix)]
//! Run only against a disposable, initialized Swarm daemon with Alpine preloaded.
use citadel_adapters::connectors::docker::DockerClient;
use futures_util::StreamExt;
use std::time::Duration;
use tokio::net::{TcpStream, UnixListener};

#[tokio::test]
#[ignore = "requires isolated CITADEL_DOCKER_FIXTURE_TCP Swarm daemon and CITADEL_DOCKER_FIXTURE_IMAGE"]
async fn generated_finite_calls_and_handwritten_streams_work_with_a_real_daemon() {
    let address = std::env::var("CITADEL_DOCKER_FIXTURE_TCP").unwrap();
    let image = std::env::var("CITADEL_DOCKER_FIXTURE_IMAGE").unwrap();
    let path = std::env::temp_dir().join(format!("citadel-live-{}.sock", uuid::Uuid::now_v7()));
    let listener = UnixListener::bind(&path).unwrap();
    let proxy = tokio::spawn(async move {
        let mut connections = tokio::task::JoinSet::new();
        loop {
            let (mut local, _) = listener.accept().await.unwrap();
            let address = address.clone();
            connections.spawn(async move {
                let mut remote = TcpStream::connect(address).await.unwrap();
                let _ = tokio::io::copy_bidirectional(&mut local, &mut remote).await;
            });
        }
    });
    let client = DockerClient::new(&path, Duration::from_secs(15)).unwrap();
    client.ping().await.unwrap();
    assert!(!client.info().await.unwrap().id.is_empty());
    assert!(!client.list_images().await.unwrap().is_empty());
    assert!(!client.inspect_image(&image).await.unwrap().id.is_empty());
    assert!(!client.image_history(&image).await.unwrap().is_empty());
    let network = client
        .create_network(
            &serde_json::from_value(
                serde_json::json!({"Name":"citadel-platform-network","Driver":"bridge"}),
            )
            .unwrap(),
        )
        .await
        .unwrap();
    client.inspect_network(&network.id).await.unwrap();
    client.list_networks().await.unwrap();
    let volume = client
        .create_volume(
            &serde_json::from_value(
                serde_json::json!({"Name":"citadel-platform-volume","Driver":"local"}),
            )
            .unwrap(),
        )
        .await
        .unwrap();
    client.inspect_volume(&volume.name).await.unwrap();
    client.list_volumes().await.unwrap();
    let mut events = client.events(None, None).await.unwrap();
    let id=client.create_container("citadel-platform-container",&serde_json::json!({"Image":image,"Cmd":["sh","-c","echo fixture; sleep 60"],"HostConfig":{"Binds":["citadel-platform-volume:/data"]}})).await.unwrap();
    client.start_container(&id).await.unwrap();
    client.inspect_container(&id).await.unwrap();
    client.inspect_container_document(&id).await.unwrap();
    client.list_containers(true).await.unwrap();
    client.image_containers(&image).await.unwrap();
    client.container_stats_once(&id).await.unwrap();
    let mut stats = client.container_stats(&id).await.unwrap();
    tokio::time::timeout(Duration::from_secs(10), stats.next())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    tokio::time::timeout(Duration::from_secs(10), events.next())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    drop(stats);
    drop(events);
    client.delete_container(&id, true, true).await.unwrap();
    client.delete_network(&network.id).await.unwrap();
    client.delete_volume(&volume.name, false).await.unwrap();
    client.inspect_swarm().await.unwrap();
    let nodes = client.list_swarm_nodes().await.unwrap();
    let node = client.inspect_swarm_node(&nodes[0].id).await.unwrap();
    client
        .update_swarm_node(&node.id, node.version.index as i64, &node.spec)
        .await
        .unwrap();
    let secret = client
        .create_swarm_material(
            true,
            &serde_json::json!({"Name":"citadel-platform-secret","Data":"c2VjcmV0"}),
        )
        .await
        .unwrap();
    let config = client
        .create_swarm_material(
            false,
            &serde_json::json!({"Name":"citadel-platform-config","Data":"Y29uZmln"}),
        )
        .await
        .unwrap();
    let secret_id = secret["ID"].as_str().unwrap();
    let config_id = config["ID"].as_str().unwrap();
    let secret = client.inspect_swarm_secret(secret_id).await.unwrap();
    let config = client.inspect_swarm_config(config_id).await.unwrap();
    client
        .update_swarm_material(true, secret_id, secret.version.index as i64, &secret.spec)
        .await
        .unwrap();
    client
        .update_swarm_material(false, config_id, config.version.index as i64, &config.spec)
        .await
        .unwrap();
    client.list_swarm_secrets().await.unwrap();
    client.list_swarm_configs().await.unwrap();
    let created=client.create_swarm_service(&serde_json::json!({"Name":"citadel-platform-service","TaskTemplate":{"ContainerSpec":{"Image":image}},"Mode":{"Replicated":{"Replicas":0}}})).await.unwrap();
    let id = created["ID"].as_str().unwrap();
    let service = client.inspect_swarm_service(id).await.unwrap();
    client
        .update_swarm_service(id, service.version.index as i64, &service.spec)
        .await
        .unwrap();
    client.list_swarm_services().await.unwrap();
    client.list_swarm_tasks().await.unwrap();
    client.list_swarm_service_tasks(id).await.unwrap();
    client.list_swarm_node_tasks(&node.id).await.unwrap();
    client.delete_swarm_service(id).await.unwrap();
    client.delete_swarm_secret(secret_id).await.unwrap();
    client.delete_swarm_config(config_id).await.unwrap();
    proxy.abort();
    let _ = proxy.await;
    std::fs::remove_file(path).unwrap();
}
