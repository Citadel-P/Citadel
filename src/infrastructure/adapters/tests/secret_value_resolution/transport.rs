use super::*;
use citadel_adapters::persistence::postgres::{
    backups, builds, deployments, stacks, swarm_services,
};
use citadel_backups::BackupSecretResolver;
use citadel_builds::BuildSecretResolver;
use citadel_deployments::DeploymentBindingResolverPort;
use citadel_stacks::StackBindingResolverPort;
use citadel_swarm_services::SwarmServiceBindingResolverPort;

#[tokio::test]
#[ignore = "requires CITADEL_EXECUTION_DATABASE_URL"]
async fn shared_resolvers_reuse_vault_transport_without_caching_tokens_values_or_grants() {
    let url = std::env::var("CITADEL_EXECUTION_DATABASE_URL").unwrap();
    MigrationRunner::migrate(&url).await.unwrap();
    let pool = PgPoolOptions::new()
        .max_connections(3)
        .connect(&url)
        .await
        .unwrap();
    let observations = Arc::new(std::sync::Mutex::new(Vec::new()));
    let observed = observations.clone();
    let barrier = Arc::new(tokio::sync::Barrier::new(2));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        let app = axum::Router::new().fallback(
            move |axum::extract::ConnectInfo(peer): axum::extract::ConnectInfo<
                std::net::SocketAddr,
            >,
                  request: axum::http::Request<axum::body::Body>| {
                let observed = observed.clone();
                let barrier = barrier.clone();
                async move {
                    let token = request
                        .headers()
                        .get("x-vault-token")
                        .unwrap()
                        .to_str()
                        .unwrap()
                        .to_owned();
                    observed.lock().unwrap().push((peer, token.clone()));
                    let path = request.uri().path();
                    if path.ends_with("concurrent") {
                        barrier.wait().await;
                    }
                    if path.ends_with("redirect") {
                        return axum::http::Response::builder()
                            .status(302)
                            .header("Location", "/trap")
                            .body(axum::body::Body::empty())
                            .unwrap();
                    }
                    if path.ends_with("oversize") {
                        return axum::http::Response::new(axum::body::Body::from(vec![
                            b'x';
                            1024 * 1024
                                + 1
                        ]));
                    }
                    assert!(!path.ends_with("trap"), "redirect must not be followed");
                    axum::http::Response::new(axum::body::Body::from(
                        json!({"data":{"data":{"TOKEN":token}}}).to_string(),
                    ))
                }
            },
        );
        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
        )
        .await
        .unwrap();
    });
    let protector = Arc::new(AesGcmSecretProtector::new(&[97; 32]).unwrap());
    let shared = PostgresSecretValueResolver::new(pool.clone(), protector.clone()).unwrap();
    let backup = backups::secrets::PostgresBackupSecretResolver::with_resolver(shared.clone());
    let build = builds::credentials::PostgresBuildSecretResolver::with_resolver(shared.clone());
    let stack =
        stacks::bindings::PostgresStackBindingResolver::with_resolver(pool.clone(), shared.clone());
    let deployment = deployments::bindings::PostgresDeploymentBindingResolver::with_resolver(
        pool.clone(),
        shared.clone(),
    );
    let service = swarm_services::bindings::PostgresSwarmServiceBindingResolver::with_resolver(
        pool.clone(),
        shared.clone(),
    );
    let ids = [Uuid::now_v7(), Uuid::now_v7()];
    let providers = [Uuid::now_v7(), Uuid::now_v7()];
    let name = format!("SHARED_{}", ids[0].simple());
    for ((id, provider), token) in ids
        .into_iter()
        .zip(providers)
        .zip(["token-one", "token-two"])
    {
        sqlx::query("INSERT INTO secretproviders(id,name,providertype,configuration) VALUES($1,$1::text,'VaultCompatibleKvV2',$2)")
            .bind(provider).bind(json!({"Address":format!("http://{address}"),"MountPath":"kv","ProtectedToken":protector.protect(token.as_bytes()).unwrap()})).execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO secretdefinitions(id,name,providertype,providerid,externalpath,externalkey) VALUES($1,$1::text,'VaultCompatibleKvV2',$2,'fixture','TOKEN')")
            .bind(id).bind(provider).execute(&pool).await.unwrap();
        sqlx::query(
            "INSERT INTO secretusegrants(secretid,scope,resourceid) VALUES($1,'Global',$2)",
        )
        .bind(id)
        .bind(Uuid::nil())
        .execute(&pool)
        .await
        .unwrap();
    }
    let binding = Uuid::now_v7();
    sqlx::query("INSERT INTO resourcebindings(id,kind,name,scope,secretid,secretdeliverymode) VALUES($1,'Secret',$2,'Global',$3,'EnvironmentVariable')")
        .bind(binding).bind(&name).bind(ids[0]).execute(&pool).await.unwrap();
    let resource = Uuid::now_v7();
    let names = [name];
    assert_eq!(
        backup.resolve(ids[0], resource).await.unwrap().as_str(),
        "token-one"
    );
    assert_eq!(
        build.resolve(ids[1], resource).await.unwrap().as_str(),
        "token-two"
    );
    assert_eq!(
        stack.resolve(resource, &names).await.unwrap().entries[0]
            .value
            .as_str(),
        "token-one"
    );
    assert_eq!(
        deployment.resolve(resource, &names).await.unwrap().entries[0]
            .value
            .as_str(),
        "token-one"
    );
    assert_eq!(
        service.resolve(resource, &names).await.unwrap().entries[0]
            .value
            .as_str(),
        "token-one"
    );
    {
        let observed = observations.lock().unwrap();
        assert_eq!(observed.len(), 5);
        assert!(
            observed.iter().all(|(peer, _)| peer == &observed[0].0),
            "five feature resolvers must reuse one TCP connection"
        );
        assert_eq!(observed[1].1, "token-two");
    }
    sqlx::query("UPDATE secretproviders SET configuration=$2 WHERE id=$1").bind(providers[0])
        .bind(json!({"Address":format!("http://{address}/changed"),"MountPath":"kv","ProtectedToken":protector.protect(b"new-token").unwrap()})).execute(&pool).await.unwrap();
    assert_eq!(
        backup.resolve(ids[0], resource).await.unwrap().as_str(),
        "new-token"
    );
    sqlx::query("DELETE FROM secretusegrants WHERE secretid=$1")
        .bind(ids[0])
        .execute(&pool)
        .await
        .unwrap();
    assert!(stack.resolve(resource, &names).await.is_err());
    assert_eq!(observations.lock().unwrap().len(), 6);
    sqlx::query("INSERT INTO secretusegrants(secretid,scope,resourceid) VALUES($1,'Global',$2)")
        .bind(ids[0])
        .bind(Uuid::nil())
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE secretdefinitions SET externalpath='concurrent' WHERE id=ANY($1)")
        .bind(ids)
        .execute(&pool)
        .await
        .unwrap();
    let (a, b) = tokio::time::timeout(std::time::Duration::from_secs(3), async {
        tokio::join!(
            backup.resolve(ids[0], resource),
            build.resolve(ids[1], resource)
        )
    })
    .await
    .expect("shared transport must not serialize different requests");
    assert_eq!(a.unwrap().as_str(), "new-token");
    assert_eq!(b.unwrap().as_str(), "token-two");
    for path in ["redirect", "oversize"] {
        sqlx::query("UPDATE secretdefinitions SET externalpath=$2 WHERE id=$1")
            .bind(ids[0])
            .bind(path)
            .execute(&pool)
            .await
            .unwrap();
        assert!(backup.resolve(ids[0], resource).await.is_err());
    }
    assert_eq!(
        observations.lock().unwrap().len(),
        10,
        "redirects must not add requests"
    );
    sqlx::query("DELETE FROM resourcebindings WHERE id=$1")
        .bind(binding)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM secretdefinitions WHERE id=ANY($1)")
        .bind(ids)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM secretproviders WHERE id=ANY($1)")
        .bind(providers)
        .execute(&pool)
        .await
        .unwrap();
    server.abort();
    let _ = server.await;
    pool.close().await;
}
