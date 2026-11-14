use super::*;
use citadel_adapters::connectors::routing::build_pools::BuildPoolRuntimeRegistry;
use citadel_builds::BuildPoolConnections;
use sha2::{Digest, Sha256};
use uuid::Uuid;

#[tokio::test]
#[ignore = "requires CITADEL_EXECUTION_DATABASE_URL"]
async fn registered_build_pools_share_connections_and_fence_changes_without_retargeting_old_clients()
 {
    let url = std::env::var("CITADEL_EXECUTION_DATABASE_URL").unwrap();
    citadel_database::MigrationRunner::migrate(&url)
        .await
        .unwrap();
    let pool = sqlx::PgPool::connect(&url).await.unwrap();
    let directory = tempfile::tempdir().unwrap();
    let signer = AgentRequestSigner::load_or_create(&directory.path().join("signing-key")).unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = format!("http://{}", listener.local_addr().unwrap());
    let accepts = Arc::new(AtomicUsize::new(0));
    let counter = accepts.clone();
    let incoming = async_stream::stream! {
        loop {
            let accepted = listener.accept().await;
            if accepted.is_ok() { counter.fetch_add(1, Ordering::SeqCst); }
            yield accepted.map(|(stream, _)| stream);
        }
    };
    let audiences = Arc::new(Mutex::new(Vec::new()));
    let observed = audiences.clone();
    let checking_signer = signer.clone();
    let service = PlatformServiceServer::with_interceptor(
        FixtureService {
            behavior: HandshakeBehavior::Delay,
            attempts: Arc::new(AtomicUsize::new(0)),
            nonces: Arc::new(Mutex::new(Vec::new())),
        },
        move |request: Request<()>| {
            use base64::Engine;
            use ed25519_dalek::Verifier;
            let binary = |key| request.metadata().get_bin(key).unwrap().to_bytes().unwrap();
            let audience = binary("x-audience-sha256-bin");
            let mut payload = b"citadel-direct-v2\0".to_vec();
            payload.extend_from_slice(&audience);
            payload.extend_from_slice(&binary("x-timestamp-bin"));
            payload.extend_from_slice(&binary("x-nonce-bin"));
            payload.extend_from_slice(b"/citadel.platforms.v1.PlatformService/GetPlatformInfo");
            payload.extend_from_slice(&Sha256::digest([]));
            let key: [u8; 32] = base64::engine::general_purpose::STANDARD
                .decode(checking_signer.public_key_base64())
                .unwrap()
                .try_into()
                .unwrap();
            ed25519_dalek::VerifyingKey::from_bytes(&key)
                .unwrap()
                .verify(
                    &payload,
                    &ed25519_dalek::Signature::from_slice(&binary("x-signature-bin")).unwrap(),
                )
                .map_err(|_| Status::unauthenticated("signature mismatch"))?;
            observed.try_lock().unwrap().push(audience.to_vec());
            Ok(request)
        },
    );
    let stop = CancellationToken::new();
    let shutdown = stop.clone();
    let server = tokio::spawn(async move {
        tonic::transport::Server::builder()
            .add_service(service)
            .serve_with_incoming_shutdown(incoming, shutdown.cancelled())
            .await
            .unwrap();
    });
    let base = AgentClient::lazy(
        "http://unused.invalid",
        signer.clone(),
        Duration::from_secs(2),
        true,
    )
    .unwrap();
    let registry = BuildPoolRuntimeRegistry::new(pool.clone(), Some(base.clone()));
    let actor = Uuid::now_v7();
    sqlx::query("INSERT INTO actors(id,isenabled,type) VALUES($1,true,'System')")
        .bind(actor)
        .execute(&pool)
        .await
        .unwrap();
    let ids = [Uuid::now_v7(), Uuid::now_v7()];
    for id in ids {
        sqlx::query("INSERT INTO buildagentpools(id,name,normalizedname,createdbyactorid,provider,providerspec) VALUES($1,$1::text,$1::text,$2,'SelfManagedVm',$3)")
            .bind(id).bind(actor).bind(serde_json::json!({"$type":"SelfManagedVm","endpoint":address})).execute(&pool).await.unwrap();
    }
    let cancel = CancellationToken::new();
    let concurrent = futures_util::future::join_all((0..8).map(|_| async {
        let client = registry.resolve(ids[0], &address, &cancel).await.unwrap();
        client.get_info(&cancel).await.unwrap();
        client
    }))
    .await;
    let first = concurrent[0].clone();
    for client in concurrent {
        assert!(Arc::ptr_eq(&first, &client));
    }
    assert_eq!(
        accepts.load(Ordering::SeqCst),
        1,
        "unchanged target must reuse its HTTP/2 connection"
    );
    signer.rotate().unwrap();
    first.get_info(&cancel).await.unwrap();
    let second = registry.resolve(ids[1], &address, &cancel).await.unwrap();
    second.get_info(&cancel).await.unwrap();
    assert_eq!(
        accepts.load(Ordering::SeqCst),
        2,
        "distinct resources own distinct signed audiences"
    );
    assert_eq!(
        audiences.lock().await.last().unwrap(),
        Sha256::digest(ids[1].to_string().as_bytes()).as_slice()
    );
    assert_eq!(
        audiences.lock().await[0],
        Sha256::digest(ids[0].to_string().as_bytes()).as_slice()
    );
    let normalized = registry
        .resolve(ids[1], &format!("{address}/"), &cancel)
        .await
        .unwrap();
    assert!(Arc::ptr_eq(&second, &normalized));
    let (new_address, _, _, new_stop) = start_fixture(HandshakeBehavior::FailOnce).await;
    sqlx::query("UPDATE buildagentpools SET providerspec=$2 WHERE id=$1")
        .bind(ids[0])
        .bind(serde_json::json!({"$type":"SelfManagedVm","endpoint":new_address}))
        .execute(&pool)
        .await
        .unwrap();
    assert!(registry.resolve(ids[0], &address, &cancel).await.is_err());
    let changed = registry
        .resolve(ids[0], &new_address, &cancel)
        .await
        .unwrap();
    assert!(!Arc::ptr_eq(&first, &changed));
    changed.get_info(&cancel).await.unwrap();
    // A retained attempt/explicit connection still talks to the original executor.
    first.get_info(&cancel).await.unwrap();
    let pinned = base.for_address(&address, ids[0], &cancel).await.unwrap();
    pinned.get_info(&cancel).await.unwrap();
    assert_eq!(pinned.address(), address);
    sqlx::query("DELETE FROM buildagentpools WHERE id=ANY($1)")
        .bind(ids)
        .execute(&pool)
        .await
        .unwrap();
    assert!(
        registry
            .resolve(ids[0], &new_address, &cancel)
            .await
            .is_err()
    );
    let retired = Arc::downgrade(&second);
    drop(second);
    drop(normalized);
    registry.expire_connections().await.unwrap();
    assert!(
        retired.upgrade().is_none(),
        "deleted registration retained its owner"
    );
    assert!(registry.resolve(ids[1], &address, &cancel).await.is_err());
    let cancelled = CancellationToken::new();
    cancelled.cancel();
    assert_eq!(
        registry
            .resolve(ids[0], &new_address, &cancelled)
            .await
            .err()
            .unwrap()
            .kind,
        RuntimeErrorKind::Cancelled
    );
    assert!(
        base.for_address(&address, ids[0], &cancelled)
            .await
            .is_err()
    );
    sqlx::query("DELETE FROM actors WHERE id=$1")
        .bind(actor)
        .execute(&pool)
        .await
        .unwrap();
    new_stop.cancel();
    stop.cancel();
    drop(first);
    drop(pinned);
    drop(changed);
    server.await.unwrap();
    pool.close().await;
}
