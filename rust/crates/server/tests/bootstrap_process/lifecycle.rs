use super::*;
use citadel_contracts::citadel::edge::v1::{
    AgentEnvelope, core_envelope, edge_agent_service_client::EdgeAgentServiceClient,
};

#[tokio::test]
#[ignore = "requires CITADEL_TEST_DATABASE_URL with CREATE DATABASE permission; starts isolated Core processes"]
async fn startup_registers_routes_starts_workers_and_shuts_down_both_listeners() {
    let fixture = Fixture::new().await;
    tokio::fs::write(fixture.directory.join("password"), PASSWORD)
        .await
        .unwrap();

    // Exercise both realtime compositions, SIGTERM, Ctrl+C, and a migrated restart.
    for (realtime, signal) in [(true, "TERM"), (false, "INT")] {
        let mut server = fixture.start_with_realtime(true, false, realtime);
        assert_eq!(fixture.ready(&mut server).await["requiresSetup"], false);
        fixture.login(&server).await;
        assert!(
            tokio::net::TcpStream::connect(("127.0.0.1", server.edge_port))
                .await
                .is_ok()
        );

        let health = fixture
            .client
            .get(format!("{}/health", server.url))
            .send()
            .await
            .unwrap();
        assert_eq!(health.status(), 200);
        assert!(health.headers().contains_key("x-request-id"));
        let ready: Value = fixture
            .client
            .get(format!("{}/ready", server.url))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        assert_eq!(ready["database"], true);
        assert_eq!(ready["setup"], true);
        assert_eq!(
            ready["docker"], false,
            "the fixture never connects to the host Docker daemon"
        );

        // Workers, rather than the request handler, maintain these counters.
        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                let metrics = fixture
                    .client
                    .get(format!("{}/metrics", server.url))
                    .send()
                    .await
                    .unwrap()
                    .text()
                    .await
                    .unwrap();
                let value = |name: &str| {
                    metrics
                        .lines()
                        .find_map(|line| line.strip_prefix(name))
                        .unwrap()
                        .trim()
                        .parse::<u64>()
                        .unwrap()
                };
                if value("citadel_active_tasks ") > 0
                    && value("citadel_readiness_failures_total ") > 0
                {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        .expect("supervised workers must start");

        let spec: Value = fixture
            .client
            .get(format!("{}/openapi/v1.json", server.url))
            .send()
            .await
            .unwrap()
            .error_for_status()
            .unwrap()
            .json()
            .await
            .unwrap();
        for (template, item) in spec["paths"].as_object().unwrap() {
            let path = template
                .split('/')
                .map(|segment| {
                    if segment.starts_with('{') {
                        "00000000-0000-0000-0000-000000000001"
                    } else {
                        segment
                    }
                })
                .collect::<Vec<_>>()
                .join("/");
            // TRACE has no handlers: method rejection and Allow verify registration
            // without executing any business mutations or opening streams.
            let response = fixture
                .client
                .request(reqwest::Method::TRACE, format!("{}{path}", server.url))
                .send()
                .await
                .unwrap();
            assert_eq!(response.status(), 405, "route missing: {template}");
            let allow = response.headers().get("allow").unwrap().to_str().unwrap();
            for method in item
                .as_object()
                .unwrap()
                .keys()
                .filter(|m| matches!(m.as_str(), "get" | "post" | "put" | "patch" | "delete"))
            {
                assert!(
                    allow
                        .split(',')
                        .any(|allowed| allowed.trim() == method.to_uppercase()),
                    "{method} {template} absent from Allow: {allow}"
                );
            }
        }
        assert_eq!(
            fixture
                .client
                .get(format!("{}/swagger/", server.url))
                .send()
                .await
                .unwrap()
                .status(),
            200
        );
        assert_eq!(
            fixture
                .client
                .get(format!("{}/api/v1/not-a-route", server.url))
                .send()
                .await
                .unwrap()
                .status(),
            404
        );
        assert_eq!(
            fixture
                .client
                .get(format!("{}/api/v1/stacks", server.url))
                .send()
                .await
                .unwrap()
                .status(),
            401
        );

        let (mut websocket, response) = tokio_tungstenite::connect_async(format!(
            "{}/api/v1/realtime",
            server.url.replacen("http://", "ws://", 1)
        ))
        .await
        .unwrap();
        assert_eq!(response.status(), 101);
        websocket.close(None).await.unwrap();

        // An unauthenticated handshake reaches the gRPC service and is rejected
        // without enrolling an agent or touching Docker.
        tokio::time::timeout(Duration::from_secs(3), async {
            let channel = tonic::transport::Endpoint::from_shared(format!(
                "http://127.0.0.1:{}",
                server.edge_port
            ))
            .unwrap()
            .connect()
            .await
            .unwrap();
            let mut client = EdgeAgentServiceClient::new(channel);
            let mut stream = client
                .connect(futures_util::stream::iter([AgentEnvelope::default()]))
                .await
                .unwrap()
                .into_inner();
            assert!(matches!(
                stream.message().await.unwrap().unwrap().body,
                Some(core_envelope::Body::SessionRejected(_))
            ));
        })
        .await
        .expect("edge gRPC route must respond");

        let pid = server.child.id().unwrap();
        assert!(
            Command::new("kill")
                .args([format!("-{signal}"), pid.to_string()])
                .status()
                .await
                .unwrap()
                .success()
        );
        let status = tokio::time::timeout(Duration::from_secs(15), server.child.wait())
            .await
            .expect("graceful shutdown timed out")
            .unwrap();
        let output = format!(
            "{}{}",
            server.output.await.unwrap(),
            server.errors.await.unwrap()
        );
        assert!(
            status.success(),
            "Core must join workers and close its pool: {status}\n{output}"
        );
        let migration = output
            .lines()
            .filter_map(|line| serde_json::from_str::<Value>(line).ok())
            .find(|line| line["fields"]["message"] == "database migrations completed")
            .expect("migrations must run on every serve");
        if realtime {
            assert!(migration["fields"]["applied"].as_u64().unwrap() > 0);
        } else {
            assert_eq!(migration["fields"]["applied"], 0);
            assert!(migration["fields"]["already_applied"].as_u64().unwrap() > 0);
        }
        assert!(
            fixture
                .client
                .get(format!("{}/health", server.url))
                .send()
                .await
                .is_err()
        );
        assert!(
            tokio::net::TcpStream::connect(("127.0.0.1", server.edge_port))
                .await
                .is_err()
        );
    }
    fixture.close().await;
}
