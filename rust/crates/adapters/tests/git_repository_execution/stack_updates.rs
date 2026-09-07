use citadel_adapters::{
    docker::DockerClient,
    stack_runtime::{StackRuntimeRouter, StackUpdateRuntime},
    stack_store::PostgresStackStore,
};
use citadel_domain::ActorId;
use citadel_git::GitRepositoryExecutionService;
use citadel_stacks::{
    StackReleaseSource, StackSpec, StackStore, StackUpdateScanner, StackUpdateState,
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
    let store = PostgresStackStore::new(pool.clone());
    let input = serde_json::from_value(json!({"name":format!("git-update-{platform}"),"platformId":platform,"stackSource":"Git",
        "spec":{"$type":"Git","gitRepoId":repository,"branch":"main","composePaths":["compose.yaml"],"updateBehavior":"Notify"}})).unwrap();
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
        if let Some(StackSpec::Git { watch_paths, .. }) = &mut stack.spec {
            *watch_paths = if relevant {
                vec!["config/**".into()]
            } else {
                vec![]
            };
        }
        let token = CancellationToken::new();
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
    if let Some(StackSpec::Git { commit_sha, .. }) = &mut stack.spec {
        *commit_sha = Some(applied.into());
    }
    assert!(
        scanner
            .scan(&stack, &CancellationToken::new())
            .await
            .is_err()
    );
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
