use citadel_adapters::connectors::docker::DockerClient;
use citadel_adapters::connectors::routing::stacks::StackRuntimeRouter;
use citadel_adapters::connectors::routing::stacks::StackUpdateRuntime;
use citadel_adapters::persistence::postgres::stacks::PostgresStackRepository;
use citadel_git::GitRepositoryExecutionService;
use citadel_primitives::ActorId;
use citadel_stacks::{
    StackReleaseSource, StackRepository, StackSpec, StackUpdateScanner, StackUpdateState,
};
use serde_json::json;
use std::{sync::Arc, time::Duration};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

pub async fn verify(
    pool: &sqlx::PgPool,
    git: Arc<GitRepositoryExecutionService>,
    actor: ActorId,
    repository: Uuid,
    applied: &str,
    remote: &str,
) {
    let platform = Uuid::now_v7();
    sqlx::query("INSERT INTO platforms(id,address,connectortype,cpucount,imagecount,memtotal,name,networkcount,platformdescriptor,status,volumecount) VALUES($1,'http://localhost.docker','Local',0,0,0,$2,0,'{\"$type\":\"Docker\"}','Online',0)")
        .bind(platform).bind(format!("git-update-{platform}")).execute(pool).await.unwrap();
    let store = PostgresStackRepository::new(pool.clone());
    let input = citadel_stacks::CreateStack {
        name: format!("git-update-{platform}"), platform_id: platform,
        stack_source: citadel_stacks::StackSource::Git,
        spec: serde_json::from_value(json!({"$type":"Git","gitRepoId":repository,"branch":"main","composePaths":["compose.yaml"],"updateBehavior":"Notify"})).unwrap(),
        description: None, drift_policy: None, tag_ids: vec![], duplicate_source: None,
    };
    let stack = store.create(actor, true, &input).await.unwrap();
    let source: StackReleaseSource = serde_json::from_value(json!({"sourceType":"Git","gitRepositoryId":repository,"branch":"main","resolvedCommitSha":applied,"composePaths":["compose.yaml"]})).unwrap();
    sqlx::query("UPDATE stackreleases SET source=$2,status='Healthy' WHERE id=$1")
        .bind(stack.current_stack_release_id)
        .bind(source.to_storage_value().unwrap())
        .execute(pool)
        .await
        .unwrap();
    let mut stack = store.get_authorized(actor, true, stack.id).await.unwrap();
    let scanner = StackUpdateRuntime::new(
        StackRuntimeRouter::new(
            pool.clone(),
            DockerClient::new("/no-docker-needed-for-git", Duration::from_secs(1)).unwrap(),
            None,
        ),
        git.clone(),
    );
    for relevant in [false, true] {
        if let Some(StackSpec::Git { watch_paths, .. }) = &mut stack.resource.spec {
            *watch_paths = if relevant {
                vec!["config/**".into()]
            } else {
                vec![]
            };
        }
        let token = CancellationToken::new();
        // Scheduled checks on multiple consumers of the same repository only
        // read its committed observation: no worker or remote fetch is needed.
        let observed = git.synchronized_ref(repository, "main").await.unwrap();
        for _ in 0..2 {
            let cached =
                tokio::time::timeout(Duration::from_secs(2), scanner.scan_cached(&stack, &token))
                    .await
                    .unwrap()
                    .unwrap();
            let StackUpdateState::Git {
                recreate_stack_on_new_commit_state: cached,
                ..
            } = cached
            else {
                panic!("Expected Git update state");
            };
            assert_eq!(
                cached.remote_commit_sha.as_deref(),
                relevant.then_some(remote)
            );
            assert_eq!(cached.last_checked_at, observed.last_synced_at);
            assert!(
                !git.process_one(&token).await.unwrap(),
                "cached checks must not queue a sync"
            );
        }
        let check = scanner.scan(&stack, &token);
        let worker = async {
            while !git.process_one(&token).await.unwrap() {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        };
        let (state, ()) = tokio::time::timeout(Duration::from_secs(15), async {
            tokio::join!(check, worker)
        })
        .await
        .unwrap();
        let StackUpdateState::Git {
            recreate_stack_on_new_commit_state: state,
            ..
        } = state.unwrap()
        else {
            panic!("Expected Git update state")
        };
        assert_eq!(state.current_commit_sha, applied);
        assert_eq!(
            state.remote_commit_sha.as_deref(),
            relevant.then_some(remote)
        );
    }
    // Unavailable observations must not silently fetch or advertise stale commits.
    for status in ["Pending", "Syncing", "Degraded"] {
        sqlx::query(
            "UPDATE gitrepositoryrefs SET status=$2 WHERE gitrepositoryid=$1 AND branch='main'",
        )
        .bind(repository)
        .bind(status)
        .execute(pool)
        .await
        .unwrap();
        assert!(
            scanner
                .scan_cached(&stack, &CancellationToken::new())
                .await
                .is_err()
        );
        let queued: bool =
            sqlx::query_scalar("SELECT controlstate='Queued' FROM gitrepositories WHERE id=$1")
                .bind(repository)
                .fetch_one(pool)
                .await
                .unwrap();
        assert!(!queued);
    }
    sqlx::query(
        "UPDATE gitrepositoryrefs SET status='Healthy' WHERE gitrepositoryid=$1 AND branch='main'",
    )
    .bind(repository)
    .execute(pool)
    .await
    .unwrap();
    if let Some(StackSpec::Git { commit_sha, .. }) = &mut stack.resource.spec {
        *commit_sha = Some(applied.into());
    }
    assert!(
        scanner
            .scan(&stack, &CancellationToken::new())
            .await
            .is_err()
    );
    // A pinned Stack with a Registry is an image candidate even when its Git
    // observation is unavailable. Discovery uses the applied local tree only.
    let registry = Uuid::from_u128(0x100);
    stack
        .resource
        .spec
        .as_mut()
        .unwrap()
        .common_mut()
        .registry_id = Some(registry);
    sqlx::query("UPDATE stackreleases SET spec=$2 WHERE id=$1")
        .bind(stack.current_stack_release_id)
        .bind(stack.spec.as_ref().unwrap().to_storage_value().unwrap())
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("UPDATE gitrepositoryrefs SET status='Degraded' WHERE gitrepositoryid=$1")
        .bind(repository)
        .execute(pool)
        .await
        .unwrap();
    assert!(
        store
            .update_check_candidates(Uuid::nil(), true, 100)
            .await
            .unwrap()
            .contains(&stack.id)
    );
    assert!(
        !store
            .update_check_candidates(Uuid::nil(), false, 100)
            .await
            .unwrap()
            .contains(&stack.id)
    );
    let cache = Arc::new(
        citadel_adapters::connectors::registries::digest_cache::ImageDigestCache::default(),
    );
    let image_scanner = citadel_adapters::connectors::routing::images::scanner::ImageScanner::new(
        pool.clone(),
        Arc::new(ImageInspector),
        cache.clone(),
    )
    .with_git(git.clone());
    assert_eq!(
        image_scanner
            .run_cycle(&CancellationToken::new())
            .await
            .unwrap(),
        1
    );
    assert_eq!(
        cache.get(registry, "nginx:alpine").unwrap().digest,
        "sha256:remote"
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let owner = stack.id.to_string();
    let api = axum::Router::new().fallback(move |uri: axum::http::Uri| {
        let owner = owner.clone();
        async move {
            let value = if uri.path().ends_with("/version") {
                json!({"ApiVersion":"1.49","MinAPIVersion":"1.24","Version":"28.0.0"})
            } else if uri.path().ends_with("/containers/json") {
                json!([{"Id":"container","Names":["/web"],"Image":"nginx:alpine","ImageID":"sha256:fixture","Created":1,
                    "Labels":{"com.citadel.stack-id":owner,"com.docker.compose.service":"web"},"State":"running","Status":"Up"}])
            } else if uri.path().ends_with("/images/json") {
                json!([{"Id":"sha256:fixture","RepoTags":["nginx:alpine"],"RepoDigests":["nginx@sha256:deployed"],"Created":1,"Size":10,"Labels":{},"Containers":1}])
            } else { panic!("Unexpected Docker request: {uri}") };
            axum::Json(value)
        }
    });
    let server = tokio::spawn(async move { axum::serve(listener, api).await.unwrap() });
    let image_runtime = StackUpdateRuntime::new(
        StackRuntimeRouter::new(
            pool.clone(),
            DockerClient::with_endpoint(
                format!("http://{address}").parse().unwrap(),
                Duration::from_secs(2),
                "/host",
            )
            .unwrap(),
            None,
        )
        .with_image_cache(cache),
        git.clone(),
    );
    let state = image_runtime
        .scan_images_cached(&stack, &CancellationToken::new())
        .await
        .unwrap();
    let StackUpdateState::Git {
        recreate_stack_on_new_image_state,
        recreate_stack_on_new_commit_state,
    } = state
    else {
        panic!("expected Git state")
    };
    let image = &recreate_stack_on_new_image_state.auto_update_states[0];
    assert!(image.update_available);
    assert_eq!(image.current_digest, "sha256:deployed");
    assert_eq!(image.remote_digest.as_deref(), Some("sha256:remote"));
    let StackUpdateState::Git {
        recreate_stack_on_new_commit_state: prior,
        ..
    } = &stack.stack_update_state
    else {
        unreachable!()
    };
    assert_eq!(&recreate_stack_on_new_commit_state, prior);
    server.abort();
    let _ = server.await;
    let queued: bool =
        sqlx::query_scalar("SELECT controlstate='Queued' FROM gitrepositories WHERE id=$1")
            .bind(repository)
            .fetch_one(pool)
            .await
            .unwrap();
    assert!(!queued, "image discovery must not queue a Git sync");
    sqlx::query("UPDATE gitrepositoryrefs SET status='Healthy' WHERE gitrepositoryid=$1")
        .bind(repository)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM stacks WHERE id=$1")
        .bind(stack.id)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM activityevents WHERE resourceid=$1")
        .bind(stack.id)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM platforms WHERE id=$1")
        .bind(platform)
        .execute(pool)
        .await
        .unwrap();
}

struct ImageInspector;
impl citadel_adapters::connectors::routing::images::scanner::ImageScanRuntime for ImageInspector {
    fn inspect<'a>(
        &'a self,
        task: &'a citadel_adapters::connectors::routing::images::scanner::ImageScanTask,
        _: &'a CancellationToken,
    ) -> futures_util::future::BoxFuture<'a, Result<String, String>> {
        Box::pin(async move {
            assert_eq!(task.reference, "nginx:alpine");
            Ok("sha256:remote".into())
        })
    }
}
