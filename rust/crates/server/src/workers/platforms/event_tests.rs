use super::*;
use axum::{Json, Router, body::Body, response::IntoResponse};

#[tokio::test]
async fn local_stop_enqueues_only_exited_before_any_observation() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let docker = DockerClient::with_endpoint(
        format!("http://{}", listener.local_addr().unwrap())
            .parse()
            .unwrap(),
        Duration::from_secs(2),
        "/host",
    )
    .unwrap();
    let router = Router::new().fallback(|request: axum::extract::Request| async move {
        match request.uri().path() {
            "/version" => Json(serde_json::json!({"ApiVersion":"1.49","MinAPIVersion":"1.41"})).into_response(),
            "/v1.49/events" => Body::from_stream(async_stream::stream! {
                for (resource, action) in [("container", "kill"), ("container", "die"), ("container", "stop"), ("network", "disconnect"), ("container", "destroy")] {
                    yield Ok::<_, std::io::Error>(bytes::Bytes::from(format!("{}\n", serde_json::json!({
                        "Type":resource, "Action":action, "Actor":{"ID":"fixture"}, "scope":"local", "time":123
                    }))));
                }
                std::future::pending::<()>().await;
            }).into_response(),
            _ => panic!("event classification must not observe Docker: {}", request.uri()),
        }
    });
    let cancel = CancellationToken::new();
    let server_cancel = cancel.clone();
    let server = tokio::spawn(async move {
        axum::serve(listener, router)
            .with_graceful_shutdown(server_cancel.cancelled_owned())
            .await
            .unwrap();
    });
    let (sender, mut receiver) = bounded_channel(1, QueueOverflowPolicy::Wait);
    let source = tokio::spawn(event_source(
        cancel.clone(),
        docker,
        sender,
        Arc::new(Metrics::default()),
    ));
    for expected in [ContainerChange::Exited, ContainerChange::Tombstone] {
        let event = tokio::time::timeout(Duration::from_secs(3), receiver.recv(&cancel))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(event.kind, RuntimeEventKind::Container(expected));
        assert_eq!(event.container_id.as_deref(), Some("fixture"));
        assert_eq!(event.event_time_millis, Some(123_000));
    }
    assert!(receiver.try_recv().is_none());
    cancel.cancel();
    source.await.unwrap().unwrap();
    server.await.unwrap();
}

#[tokio::test]
async fn successful_empty_or_broken_streams_back_off_and_shutdown_interrupts_wait() {
    for body in ["", "invalid-json\n"] {
        let attempts = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let docker = DockerClient::with_endpoint(
            format!("http://{}", listener.local_addr().unwrap())
                .parse()
                .unwrap(),
            Duration::from_secs(2),
            "/host",
        )
        .unwrap();
        let router = Router::new().fallback({
            let attempts = attempts.clone();
            move |req: axum::extract::Request| {
                let attempts = attempts.clone();
                async move {
                    if req.uri().path() == "/version" {
                        Json(serde_json::json!({"ApiVersion":"1.49","MinAPIVersion":"1.41"}))
                            .into_response()
                    } else {
                        assert_eq!(req.uri().path(), "/v1.49/events");
                        attempts.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                        body.into_response()
                    }
                }
            }
        });
        let cancel = CancellationToken::new();
        let stop = cancel.clone();
        let server = tokio::spawn(async move {
            axum::serve(listener, router)
                .with_graceful_shutdown(stop.cancelled_owned())
                .await
                .unwrap();
        });
        let (sender, mut receiver) = bounded_channel(64, QueueOverflowPolicy::Wait);
        let source = tokio::spawn(event_source(
            cancel.clone(),
            docker,
            sender,
            Arc::new(Metrics::default()),
        ));
        tokio::time::sleep(Duration::from_millis(650)).await;
        let count = attempts.load(std::sync::atomic::Ordering::SeqCst);
        assert!((1..=3).contains(&count), "unbounded retries: {count}");
        let mut recoveries = 0;
        while receiver.try_recv().is_some() {
            recoveries += 1;
        }
        assert_eq!(recoveries, count - 1);
        cancel.cancel();
        tokio::time::timeout(Duration::from_millis(100), source)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        server.await.unwrap();
    }
}
