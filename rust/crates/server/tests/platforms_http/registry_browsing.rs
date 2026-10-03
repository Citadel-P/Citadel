use super::*;
use citadel_adapters::connectors::registries::browser::RegistryBrowser;

// Ports RegistryImageQueryAuthorizationTests and ImageEndpointTests for all four
// browse routes: real authorized Registry rows and controlled external HTTP servers.
#[tokio::test]
#[ignore = "requires CITADEL_PLATFORM_DATABASE_URL"]
async fn registry_browsing_authorizes_before_using_credentials_and_maps_existing_contracts() {
    let mut f = fixture().await;
    let calls = Arc::new(Mutex::new(Vec::new()));
    let seen = calls.clone();
    let failing = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let fail = failing.clone();
    let mock=Router::new().fallback(move |request:Request<Body>| {
        let calls=seen.clone();let fail=fail.clone();
        async move {
            let path=request.uri().path().to_owned();
            calls.lock().await.push(path.clone());
            if fail.load(std::sync::atomic::Ordering::SeqCst) {return (StatusCode::UNAUTHORIZED,axum::Json(json!({"message":"provider-leaked-PAT-fixture"})));}
            let response=if path=="/v2/auth/token" {
                let body:Value=serde_json::from_slice(&to_bytes(request.into_body(),4096).await.unwrap()).unwrap();
                assert_eq!(body,json!({"identifier":"tester","secret":"fixture-PAT"}));
                json!({"access_token":"fixture-access-token"})
            } else if path=="/v2/repositories/tester" {
                assert_eq!(request.headers()["authorization"],"Bearer fixture-access-token");
                json!({"results":[{"name":"nginx","namespace":"tester","last_updated":"2026-01-01T00:00:00Z","is_private":true,"is_trusted":false,"is_automated":false,"pull_count":42}]})
            } else if path=="/v2/namespaces/tester/repositories/nginx/tags" {
                assert_eq!(request.headers()["authorization"],"Bearer fixture-access-token");
                json!({"results":[{"id":3,"name":"latest","last_updated":"2026-01-01T00:00:00Z","full_size":123,"status":"active","tag_last_pulled":"2026-01-01T00:00:00Z","images":[{"architecture":"amd64","digest":"sha256:123","os":"linux","size":123,"status":"active","last_pulled":"2026-01-01T00:00:00Z"}]}]})
            } else if path=="/users/tester/packages" {
                assert_eq!(request.headers()["authorization"],"Bearer fixture-PAT");
                json!([{"id":42,"name":"server","created_at":"2026-01-01T00:00:00Z","updated_at":"2026-01-02T00:00:00Z","url":"https://api.github.test/packages/42","html_url":"https://github.test/packages/42"}])
            } else {
                assert_eq!(path,"/user/packages/container/server/versions");
                assert_eq!(request.headers()["authorization"],"Bearer fixture-PAT");
                json!([{"id":43,"name":"sha256:abc","url":"https://api.github.test/version/43","metadata":{"container":{"tags":["latest"]}}}])
            };
            (StatusCode::OK,axum::Json(response))
        }
    });
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move { axum::serve(listener, mock).await.unwrap() });
    f.app = platforms_http::router(f.lookup_state.platforms.clone()).layer(axum::Extension(
        Arc::new(RegistryBrowser::with_endpoints(&address, &address).unwrap())
            as Arc<dyn citadel_registries::registry_images::RegistryBrowsePort>,
    ));
    let docker = Uuid::now_v7();
    let github = Uuid::now_v7();
    let docker_name = format!("hub-{}", docker.simple());
    let github_name = format!("ghcr-{}", github.simple());
    for (id, name, cfg, host) in [
        (
            docker,
            &docker_name,
            json!({"$type":"DockerHub","UserName":"tester","PAT":"fixture-PAT"}),
            "registry-1.docker.io",
        ),
        (
            github,
            &github_name,
            json!({"$type":"GitHub","NameSpace":"tester","PAT":"fixture-PAT"}),
            "ghcr.io",
        ),
    ] {
        sqlx::query("INSERT INTO registries(id,name,registryhost,configuration,status,createdbyactorid) VALUES($1,$2,$3,$4,'Active',$5)").bind(id).bind(name).bind(host).bind(cfg).bind(f.administrator.actor_id.value()).execute(&f.pool).await.unwrap();
    }
    let reader = super::lookup::subject(&f).await;
    let paths = [
        format!("/api/v1/images/{docker_name}/repositories"),
        format!("/api/v1/images/dockerhub/{docker_name}/repositories"),
        format!("/api/v1/images/dockerhub/{docker_name}/nginx/tags"),
        format!("/api/v1/images/ghcr/{github_name}/server/versions"),
    ];
    for path in &paths {
        assert_eq!(
            send(&f, path, None).await.status(),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            send(&f, path, Some(reader.clone())).await.status(),
            StatusCode::FORBIDDEN
        );
    }
    assert!(
        calls.lock().await.is_empty(),
        "Denied access must not authenticate with Registry credentials"
    );
    super::lookup::grant(
        &f,
        reader.actor_id.value(),
        citadel_primitives::ResourceType::Registry,
        docker,
        0,
    )
    .await;
    super::lookup::grant(
        &f,
        reader.actor_id.value(),
        citadel_primitives::ResourceType::Registry,
        github,
        0,
    )
    .await;
    // SQL fixture grants must invalidate the earlier cached denial.
    super::realtime_groups::replace_specific_permissions(&f, &reader, vec![]).await;
    let uppercase = paths[0].replace(&docker_name, &docker_name.to_uppercase());
    assert_eq!(
        send(&f, &uppercase, Some(reader.clone())).await.status(),
        StatusCode::OK
    );
    assert_eq!(
        send(
            &f,
            "/api/v1/images/nonexistent-registry/repositories",
            Some(reader.clone())
        )
        .await
        .status(),
        StatusCode::NOT_FOUND
    );
    sqlx::query("UPDATE registries SET status='Disabled' WHERE id=$1")
        .bind(docker)
        .execute(&f.pool)
        .await
        .unwrap();
    let before = calls.lock().await.len();
    assert_eq!(
        send(&f, &paths[0], Some(reader.clone())).await.status(),
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        calls.lock().await.len(),
        before,
        "Disabled registries must not use credentials"
    );
    sqlx::query("UPDATE registries SET status='Active' WHERE id=$1")
        .bind(docker)
        .execute(&f.pool)
        .await
        .unwrap();
    let external = json_body(send(&f, &paths[0], Some(reader.clone())).await).await;
    assert_eq!(external[0]["$type"], "DockerHub");
    assert_eq!(external[0]["pullCount"], 42);
    assert!(external[0].get("isTrusted").is_none());
    let repos = json_body(send(&f, &paths[1], Some(reader.clone())).await).await;
    assert_eq!(repos[0]["isPrivate"], true);
    let tags = json_body(send(&f, &paths[2], Some(reader.clone())).await).await;
    assert_eq!(tags[0]["status"], "Active");
    assert_eq!(tags[0]["image"]["architecture"], "amd64");
    let versions = json_body(send(&f, &paths[3], Some(reader.clone())).await).await;
    assert_eq!(
        versions[0]["metadata"]["container"]["tags"],
        json!(["latest"])
    );
    let gh = json_body(
        send(
            &f,
            &format!("/api/v1/images/{github_name}/repositories"),
            Some(reader.clone()),
        )
        .await,
    )
    .await;
    assert_eq!(gh[0]["$type"], "GitHub");
    assert_eq!(gh[0]["id"], "42");
    assert_eq!(
        send(
            &f,
            &format!("/api/v1/images/dockerhub/{github_name}/repositories"),
            Some(reader.clone())
        )
        .await
        .status(),
        StatusCode::NOT_FOUND
    );
    failing.store(true, std::sync::atomic::Ordering::SeqCst);
    let error = send(&f, &paths[0], Some(reader)).await;
    assert_eq!(error.status(), StatusCode::BAD_REQUEST);
    let bytes = to_bytes(error.into_body(), 65536).await.unwrap();
    assert!(!String::from_utf8_lossy(&bytes).contains("PAT-fixture"));
    server.abort();
}
