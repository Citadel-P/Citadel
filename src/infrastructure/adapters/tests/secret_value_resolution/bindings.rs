use super::*;
use citadel_bindings::SecretProtector;
use citadel_deployments::DeploymentBindingResolverPort;
use citadel_stacks::StackBindingResolverPort;
use citadel_swarm_services::SwarmServiceBindingResolverPort;
use sqlx::PgPool;

async fn resolve(
    pool: &PgPool,
    protector: Arc<dyn SecretProtector>,
    scope: &str,
    resource: Uuid,
    names: &[String],
) -> Result<Vec<(String, String, bool, Option<String>)>, String> {
    use citadel_adapters::persistence::postgres::{deployments, stacks, swarm_services};
    match scope {
        "Stack" => stacks::bindings::PostgresStackBindingResolver::new(pool.clone(), protector)
            .unwrap()
            .resolve(resource, names)
            .await
            .map(|r| {
                r.entries
                    .into_iter()
                    .map(|e| {
                        (
                            e.name,
                            e.value.to_string(),
                            e.secret,
                            Some(e.snapshot.value),
                        )
                    })
                    .collect()
            })
            .map_err(|e| e.to_string()),
        "Deployment" => {
            deployments::bindings::PostgresDeploymentBindingResolver::new(pool.clone(), protector)
                .unwrap()
                .resolve(resource, names)
                .await
                .map(|r| {
                    r.entries
                        .into_iter()
                        .map(|e| {
                            (
                                e.name,
                                e.value.to_string(),
                                e.secret,
                                Some(e.snapshot.value),
                            )
                        })
                        .collect()
                })
                .map_err(|e| e.to_string())
        }
        "SwarmService" => swarm_services::bindings::PostgresSwarmServiceBindingResolver::new(
            pool.clone(),
            protector,
        )
        .unwrap()
        .resolve(resource, names)
        .await
        .map(|r| {
            r.entries
                .into_iter()
                .map(|e| (e.name, e.value.to_string(), e.secret, None))
                .collect()
        })
        .map_err(|e| e.to_string()),
        _ => unreachable!(),
    }
}

#[tokio::test]
#[ignore = "requires CITADEL_EXECUTION_DATABASE_URL"]
async fn only_winning_bindings_resolve_secrets_for_all_workloads() {
    let url = std::env::var("CITADEL_EXECUTION_DATABASE_URL").unwrap();
    MigrationRunner::migrate(&url).await.unwrap();
    let pool = PgPoolOptions::new()
        .max_connections(3)
        .connect(&url)
        .await
        .unwrap();
    let requests = Arc::new(AtomicUsize::new(0));
    let available = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let (count, healthy) = (requests.clone(), available.clone());
    let server = tokio::spawn(async move {
        axum::serve(
            listener,
            axum::Router::new().fallback(move || {
                let (count, healthy) = (count.clone(), healthy.clone());
                async move {
                    count.fetch_add(1, Ordering::SeqCst);
                    (
                        if healthy.load(Ordering::SeqCst) {
                            StatusCode::OK
                        } else {
                            StatusCode::SERVICE_UNAVAILABLE
                        },
                        Json(json!({"data":{"data":{"TOKEN":"resolved-value"}}})),
                    )
                }
            }),
        )
        .await
        .unwrap();
    });
    let protector = Arc::new(AesGcmSecretProtector::new(&[97; 32]).unwrap());
    let provider = Uuid::now_v7();
    sqlx::query("INSERT INTO secretproviders(id,name,providertype,configuration) VALUES($1,$1::text,'VaultCompatibleKvV2',$2)")
        .bind(provider).bind(json!({"Address":format!("http://{address}"),"MountPath":"kv","ProtectedToken":protector.protect(b"fixture-token").unwrap()}))
        .execute(&pool).await.unwrap();
    for scope in ["Stack", "Deployment", "SwarmService"] {
        available.store(false, Ordering::SeqCst);
        requests.store(0, Ordering::SeqCst);
        let resource = Uuid::now_v7();
        let secret = Uuid::now_v7();
        let (global, local, extra) = (Uuid::now_v7(), Uuid::now_v7(), Uuid::now_v7());
        let name = format!("B_{}", resource.simple());
        let first = format!("A_{}", resource.simple());
        let names = [name.clone(), first.clone()];
        sqlx::query("INSERT INTO secretdefinitions(id,name,providertype,providerid,externalpath,externalkey) VALUES($1,$1::text,'VaultCompatibleKvV2',$2,'fixture','TOKEN')")
            .bind(secret).bind(provider).execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO secretusegrants(secretid,scope,resourceid) VALUES($1,$2,$3)")
            .bind(secret)
            .bind(scope)
            .bind(resource)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO resourcebindings(id,kind,name,scope,secretid,secretdeliverymode) VALUES($1,'Secret',$2,'Global',$3,'EnvironmentVariable')")
            .bind(global).bind(&name).bind(secret).execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO resourcebindings(id,kind,name,scope,resourceid,value) VALUES($1,'Variable',$2,$3,$4,'override'),($5,'Variable',$6,$3,$4,'first')")
            .bind(local).bind(name.to_ascii_lowercase()).bind(scope).bind(resource).bind(extra).bind(&first).execute(&pool).await.unwrap();
        let values = resolve(&pool, protector.clone(), scope, resource, &names)
            .await
            .unwrap();
        assert_eq!(values[0].0, first);
        assert_eq!(values[1].0, name.to_ascii_lowercase());
        assert_eq!(values[1].1, "override");
        assert!(!values[1].2);
        if let Some(snapshot) = &values[1].3 {
            assert_eq!(snapshot, "override");
        }
        assert_eq!(
            requests.load(Ordering::SeqCst),
            0,
            "shadowed provider must not be contacted for {scope}"
        );
        sqlx::query("DELETE FROM resourcebindings WHERE id=$1")
            .bind(local)
            .execute(&pool)
            .await
            .unwrap();
        assert!(
            resolve(&pool, protector.clone(), scope, resource, &names)
                .await
                .is_err()
        );
        assert_eq!(
            requests.load(Ordering::SeqCst),
            1,
            "unavailable winning provider must fail"
        );
        sqlx::query("DELETE FROM secretusegrants WHERE secretid=$1")
            .bind(secret)
            .execute(&pool)
            .await
            .unwrap();
        assert!(
            resolve(&pool, protector.clone(), scope, resource, &names)
                .await
                .is_err()
        );
        assert_eq!(
            requests.load(Ordering::SeqCst),
            1,
            "missing grant must fail before network access"
        );
        sqlx::query("INSERT INTO secretusegrants(secretid,scope,resourceid) VALUES($1,$2,$3)")
            .bind(secret)
            .bind(scope)
            .bind(resource)
            .execute(&pool)
            .await
            .unwrap();
        available.store(true, Ordering::SeqCst);
        let values = resolve(&pool, protector.clone(), scope, resource, &names)
            .await
            .unwrap();
        assert_eq!(values[1].1, "resolved-value");
        assert!(values[1].2);
        if let Some(snapshot) = &values[1].3 {
            assert_eq!(snapshot, "********");
        }
        sqlx::query("UPDATE resourcebindings SET secretdeliverymode='File' WHERE id=$1")
            .bind(global)
            .execute(&pool)
            .await
            .unwrap();
        assert!(
            resolve(&pool, protector.clone(), scope, resource, &names)
                .await
                .is_err()
        );
        assert_eq!(
            requests.load(Ordering::SeqCst),
            2,
            "invalid delivery must fail before resolution"
        );
        sqlx::query("DELETE FROM resourcebindings WHERE id=ANY($1)")
            .bind([global, extra])
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM secretdefinitions WHERE id=$1")
            .bind(secret)
            .execute(&pool)
            .await
            .unwrap();
    }
    sqlx::query("DELETE FROM secretproviders WHERE id=$1")
        .bind(provider)
        .execute(&pool)
        .await
        .unwrap();
    server.abort();
    let _ = server.await;
    pool.close().await;
}
