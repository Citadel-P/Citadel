use super::*;
use citadel_docker::client::projection::DockerEvent;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use tokio_util::sync::CancellationToken;

#[tokio::test]
async fn lifecycle_bursts_share_fresh_reads_and_recreation_is_a_barrier() {
    let path = temp_socket();
    let listener = UnixListener::bind(&path).unwrap();
    let token = CancellationToken::new();
    let shutdown = token.clone();
    let counts = Arc::new([
        AtomicUsize::new(0),
        AtomicUsize::new(0),
        AtomicUsize::new(0),
    ]);
    let counters = counts.clone();
    let server = tokio::spawn(async move {
        loop {
            let (mut conn, _) =
                tokio::select! {()=shutdown.cancelled()=>break,v=listener.accept()=>v.unwrap()};
            let (request, _) = read_terminal_request(&mut conn).await;
            let uri = request.split_ascii_whitespace().nth(1).unwrap();
            let body=if uri=="/version" {serde_json::json!({"ApiVersion":"1.49","MinAPIVersion":"1.41"})}
            else if let Some(name)=uri.strip_prefix("/v1.49/volumes/"){counters[0].fetch_add(1,Ordering::Relaxed);serde_json::json!({"Name":name})}
            else if uri=="/v1.49/containers/json?all=true&size=false"{counters[1].fetch_add(1,Ordering::Relaxed);serde_json::json!([])}
            else{
                assert_eq!(uri,"/v1.49/system/df?type=volume");
                let size=counters[2].fetch_add(1,Ordering::Relaxed)+1;
                serde_json::json!({"Volumes":(0..100).map(|i|serde_json::json!({"Name":format!("v{i}"),"UsageData":{"Size":size,"RefCount":1}})).collect::<Vec<_>>()})
            }.to_string();
            conn.write_all(
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                )
                .as_bytes(),
            )
            .await
            .unwrap();
        }
    });
    let docker = DockerClient::new(&path, Duration::from_secs(3)).unwrap();
    for n in [1, 20, 100] {
        let events=(0..n).map(|i|("create",i)).chain([("destroy",0),("create",0)]).map(|(action,i)|{
            Ok(serde_json::from_value::<DockerEvent>(serde_json::json!({"Type":"volume","Action":action,"Actor":{"ID":format!("v{i}")}})).unwrap())
        });
        let before = counts.each_ref().map(|c| c.load(Ordering::Relaxed));
        let started = std::time::Instant::now();
        let result = docker
            .volume_event_stream(Box::pin(futures_util::stream::iter(events)), token.clone())
            .collect::<Vec<_>>()
            .await;
        assert_eq!(result.len(), n + 2);
        for (index, item) in result.into_iter().enumerate() {
            let (event, value) = item.unwrap();
            if index == n {
                assert_eq!(event.action, "destroy");
                assert!(value.is_none());
            } else {
                let volume = value.unwrap();
                assert_eq!(volume.name, event.actor.id);
                assert_eq!(
                    volume.usage_data.unwrap()["Size"],
                    before[2] + 1 + usize::from(index > n)
                );
            }
        }
        assert_eq!(
            counts.each_ref().map(|c| c.load(Ordering::Relaxed)),
            [before[0] + n + 1, before[1] + 2, before[2] + 2]
        );
        eprintln!("{n} creates plus recreation: {:?}", started.elapsed());
    }
    token.cancel();
    server.await.unwrap();
    std::fs::remove_file(path).unwrap();
}
