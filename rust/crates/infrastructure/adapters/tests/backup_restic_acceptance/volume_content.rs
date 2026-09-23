use super::*;
use citadel_adapters::connectors::docker::DockerClient;
use citadel_adapters::connectors::routing::volumes::content::VolumeContentAdapter;
use futures_util::StreamExt;

pub(super) async fn verify(
    cluster: &Cluster,
    pool: &sqlx::PgPool,
    agent: &AgentClient,
    platform: Uuid,
    nodes: &[String],
    registry: &EdgeRegistry,
    volume: &str,
) {
    verify_local(pool).await;
    let adapter = VolumeContentAdapter::new(
        pool.clone(),
        DockerClient::new("/unusable-core-daemon.sock", Duration::from_secs(2)).unwrap(),
        Some(agent.clone()),
        registry.clone(),
        std::env::var("CITADEL_PHASE7_AGENT_IMAGE").unwrap(),
        citadel_runtime::DynamicTasks::new(tokio_util::sync::CancellationToken::new()),
    );
    let cancel = CancellationToken::new();
    // A same-named Volume on the connected manager must not replace a worker's data.
    for node in [0usize, 1] {
        let listing = adapter
            .list(platform, volume, "/", Some(&nodes[node]), &cancel)
            .await
            .unwrap();
        assert!(listing.entries.iter().any(|e| e.name == "payload.txt"));
        let mut download = adapter
            .download(
                platform,
                volume,
                "/payload.txt",
                Some(&nodes[node]),
                &cancel,
            )
            .await
            .unwrap();
        let mut data = Vec::new();
        while let Some(chunk) = download.stream.next().await {
            data.extend(chunk.unwrap());
        }
        assert_eq!(data, if node == 1 { PAYLOAD } else { b"wrong-node" });
    }
    cluster.node(1,&["run","--rm","--volume",&format!("{volume}:/fixture"),"--entrypoint","sh",IMAGE,"-c","mkdir -p /fixture/config; cp /fixture/payload.txt /fixture/config/item; ln -s /etc/passwd /fixture/escape"],None).await;
    assert!(
        adapter
            .download(platform, volume, "/escape", Some(&nodes[1]), &cancel)
            .await
            .is_err()
    );
    assert!(
        adapter
            .list(platform, volume, "/escape", Some(&nodes[1]), &cancel)
            .await
            .is_err()
    );
    assert!(
        adapter
            .download(platform, volume, "/", Some(&nodes[1]), &cancel)
            .await
            .is_err()
    );
    assert!(
        adapter
            .list(platform, "does-not-exist", "/", Some(&nodes[1]), &cancel)
            .await
            .is_err()
    );
    let mut directory = adapter
        .download(platform, volume, "/config", Some(&nodes[1]), &cancel)
        .await
        .unwrap();
    assert!(directory.directory);
    assert_eq!(directory.filename, "config.tar");
    let mut archive = Vec::new();
    while let Some(chunk) = directory.stream.next().await {
        archive.extend(chunk.unwrap());
    }
    // Validate the actual archive with the existing fixture's tar, not a fake payload.
    let extracted = cluster
        .node(
            1,
            &[
                "run",
                "--rm",
                "-i",
                "--entrypoint",
                "tar",
                IMAGE,
                "-xOf",
                "-",
                "config/item",
            ],
            Some(&archive),
        )
        .await;
    assert_eq!(extracted, PAYLOAD);
    // Dropping an unconsumed download also tears down its helper.
    drop(
        adapter
            .download(platform, volume, "/payload.txt", Some(&nodes[1]), &cancel)
            .await
            .unwrap(),
    );
    for node in 0..3 {
        tokio::time::timeout(Duration::from_secs(15), async {
            loop {
                if cluster
                    .node(
                        node,
                        &[
                            "ps",
                            "--all",
                            "--filter",
                            "label=citadel.volume-browser=true",
                            "--quiet",
                        ],
                        None,
                    )
                    .await
                    .is_empty()
                {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        })
        .await
        .expect("Volume helper must be cleaned after completion, failure or disconnect");
    }
    verify_expired_helpers(cluster, pool, platform, nodes, &adapter).await;
}

async fn verify_expired_helpers(
    cluster: &Cluster,
    pool: &sqlx::PgPool,
    platform: Uuid,
    nodes: &[String],
    adapter: &VolumeContentAdapter,
) {
    let now = Utc::now().timestamp();
    let mut created = vec![];
    for (node, expired, owned) in [
        (0, true, true),
        (1, true, true),
        (1, false, true),
        (1, true, false),
    ] {
        let id = Uuid::now_v7();
        let name = format!("citadel-volume-helper-{}", id.simple());
        let owner = if owned { platform } else { Uuid::now_v7() };
        let docker_id = String::from_utf8(
            cluster
                .node(
                    node,
                    &[
                        "create",
                        "--name",
                        &name,
                        "--label",
                        "com.citadel.system=true",
                        "--label",
                        "com.citadel.system-role=volume-helper",
                        "--label",
                        &format!("com.citadel.platform-id={owner}"),
                        "--label",
                        &format!(
                            "com.citadel.helper-expires-at={}",
                            if expired { now - 1 } else { now + 3600 }
                        ),
                        IMAGE,
                    ],
                    None,
                )
                .await,
        )
        .unwrap()
        .trim()
        .to_owned();
        sqlx::query("INSERT INTO containers(id,platformid,dockercontainerid,dockernodeid,dockerimageid,name,created,updated,ports,state,issystem,systemrole) VALUES($1,$2,$3,$4,'fixture',$5,$6,$6,'[]','Created',true,'volume-helper')")
            .bind(id).bind(platform).bind(&docker_id).bind(&nodes[node]).bind(&name).bind(now-3600).execute(pool).await.unwrap();
        created.push((id, node, docker_id, expired && owned));
    }
    let mut cursor = Uuid::nil();
    for _ in 0..10 {
        cursor = adapter
            .reap_expired(cursor, &CancellationToken::new())
            .await
            .unwrap();
        if cursor.is_nil() {
            break;
        }
    }
    for (id, node, docker_id, removed) in created {
        let result = cluster
            .node(
                node,
                &[
                    "ps",
                    "--all",
                    "--quiet",
                    "--no-trunc",
                    "--filter",
                    &format!("id={docker_id}"),
                ],
                None,
            )
            .await;
        assert_eq!(
            result.is_empty(),
            removed,
            "Expired Created helpers are deleted only on the exact node with exact ownership"
        );
        if !removed {
            cluster.node(node, &["rm", &docker_id], None).await;
        }
        sqlx::query("DELETE FROM containers WHERE id=$1")
            .bind(id)
            .execute(pool)
            .await
            .unwrap();
    }
}

async fn verify_local(pool: &sqlx::PgPool) {
    let platform = Uuid::now_v7();
    let name = format!("citadel-local-browser-{}", platform.simple());
    sqlx::query("INSERT INTO platforms(id,address,connectortype,cpucount,imagecount,memtotal,name,networkcount,platformdescriptor,status,volumecount) VALUES($1,'http://localhost.docker','Local',0,0,0,$2,0,'{\"$type\":\"DockerStandalone\"}','Online',0)")
        .bind(platform).bind(&name).execute(pool).await.unwrap();
    let result = AssertUnwindSafe(async {
        docker(&["volume", "create", &name], None).await;
        docker(
            &[
                "run",
                "--rm",
                "-i",
                "--volume",
                &format!("{name}:/data"),
                "--entrypoint",
                "sh",
                IMAGE,
                "-c",
                "cat > /data/file.bin",
            ],
            Some(b"local binary\0\xff"),
        )
        .await;
        let client = VolumeContentAdapter::new(
            pool.clone(),
            DockerClient::new("/var/run/docker-host.sock", Duration::from_secs(10)).unwrap(),
            None,
            EdgeRegistry::default(),
            std::env::var("CITADEL_PHASE7_AGENT_IMAGE").unwrap(),
            citadel_runtime::DynamicTasks::new(tokio_util::sync::CancellationToken::new()),
        );
        let cancel = CancellationToken::new();
        let listing = client
            .list(platform, &name, "/", None, &cancel)
            .await
            .unwrap();
        assert_eq!(listing.entries.len(), 1);
        let mut download = client
            .download(platform, &name, "/file.bin", None, &cancel)
            .await
            .unwrap();
        let mut bytes = Vec::new();
        while let Some(chunk) = download.stream.next().await {
            bytes.extend(chunk.unwrap());
        }
        assert_eq!(bytes, b"local binary\0\xff");
    })
    .catch_unwind()
    .await;
    let _ = run(
        ProcessRequest::new("docker").args(["volume", "rm", &name]),
        &CancellationToken::new(),
    )
    .await;
    if let Err(panic) = result {
        std::panic::resume_unwind(panic);
    }
}
