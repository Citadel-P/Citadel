use super::*;
use citadel_stacks::{GitCheckInputs, StackError};

async fn candidate(store: &PostgresStackRepository, id: Uuid) -> Option<GitCheckInputs> {
    store
        .update_check_candidates(Uuid::nil(), false, 100, None)
        .await
        .unwrap()
        .into_iter()
        .find(|row| row.id == id)
        .and_then(|row| match row.work {
            citadel_stacks::StackCheckWork::CompareGit(inputs) => Some(inputs),
            _ => None,
        })
}

pub(super) async fn verify(
    pool: &sqlx::PgPool,
    store: &PostgresStackRepository,
    scanner: &StackUpdateRuntime,
    git: &GitRepositoryExecutionService,
    actor: ActorId,
    id: Uuid,
) {
    let stack = store.get_authorized(actor, true, id).await.unwrap();
    let Some(StackSpec::Git {
        git_repo_id,
        branch,
        ..
    }) = &stack.spec
    else {
        panic!("expected Git fixture")
    };
    let cancel = CancellationToken::new();
    let initial = candidate(store, id).await.unwrap();
    let reference: Uuid = sqlx::query_scalar(
        "SELECT id FROM gitrepositoryrefs WHERE gitrepositoryid=$1 AND branch=$2",
    )
    .bind(git_repo_id)
    .bind(branch)
    .fetch_one(pool)
    .await
    .unwrap();
    for (scope, expected) in [(reference, true), (Uuid::now_v7(), false)] {
        let scoped = store
            .update_check_candidates(Uuid::nil(), false, 100, Some(scope))
            .await
            .unwrap();
        assert_eq!(scoped.iter().any(|candidate| candidate.id == id), expected);
    }
    let result = scanner.scan_cached(&stack, &cancel).await.unwrap();
    let StackUpdateState::Git {
        recreate_stack_on_new_commit_state,
        ..
    } = &result
    else {
        panic!("expected Git observation")
    };
    assert!(
        recreate_stack_on_new_commit_state
            .remote_commit_sha
            .is_none()
    );
    store
        .save_update_check(actor, true, &stack, &result, Some(initial))
        .await
        .unwrap();
    for _ in 0..2 {
        git.request_sync(actor, *git_repo_id, Some(branch))
            .await
            .unwrap();
        assert!(git.process_one(&cancel).await.unwrap());
        assert!(candidate(store, id).await.is_none());
    }

    // A ref can fail and recover at the same SHA while a check is in flight.
    let stack = store.get_authorized(actor, true, id).await.unwrap();
    sqlx::query(
        "UPDATE gitrepositoryrefs SET status='Degraded' WHERE gitrepositoryid=$1 AND branch=$2",
    )
    .bind(git_repo_id)
    .bind(branch)
    .execute(pool)
    .await
    .unwrap();
    assert!(candidate(store, id).await.is_none());
    git.request_sync(actor, *git_repo_id, Some(branch))
        .await
        .unwrap();
    assert!(git.process_one(&cancel).await.unwrap());
    let recovered = candidate(store, id).await.unwrap();
    assert!(matches!(
        store
            .save_update_check(actor, true, &stack, &result, Some(initial))
            .await,
        Err(StackError::Conflict(_))
    ));
    let result = scanner.scan_cached(&stack, &cancel).await.unwrap();
    store
        .save_update_check(actor, true, &stack, &result, Some(recovered))
        .await
        .unwrap();
    assert!(candidate(store, id).await.is_none());

    // The earlier commit now affects the configured paths on this same release.
    let stack = store.get_authorized(actor, true, id).await.unwrap();
    let spec = stack.spec.as_ref().unwrap().to_storage_value().unwrap();
    sqlx::query("UPDATE stackreleases SET spec=jsonb_set(spec,'{WatchPaths}','[\"config/**\"]') WHERE id=$1")
        .bind(stack.current_stack_release_id).execute(pool).await.unwrap();
    let edited = candidate(store, id).await.unwrap();
    assert!(matches!(
        store
            .save_update_check(actor, true, &stack, &result, Some(recovered))
            .await,
        Err(StackError::Conflict(_))
    ));
    let stack = store.get_authorized(actor, true, id).await.unwrap();
    let result = scanner.scan_cached(&stack, &cancel).await.unwrap();
    let StackUpdateState::Git {
        recreate_stack_on_new_commit_state,
        ..
    } = &result
    else {
        panic!("expected Git observation")
    };
    assert!(
        recreate_stack_on_new_commit_state
            .remote_commit_sha
            .is_some()
    );
    store
        .save_update_check(actor, true, &stack, &result, Some(edited))
        .await
        .unwrap();
    assert!(candidate(store, id).await.is_none());
    let version = stack.row_version + 1;
    for _ in 0..2 {
        git.request_sync(actor, *git_repo_id, Some(branch))
            .await
            .unwrap();
        assert!(git.process_one(&cancel).await.unwrap());
        assert!(
            candidate(store, id).await.is_none(),
            "unchanged pending observation needs no comparison"
        );
        let pending = store
            .update_check_candidates(Uuid::nil(), false, 100, Some(reference))
            .await
            .unwrap()
            .into_iter()
            .find(|row| row.id == id)
            .unwrap();
        assert!(
            matches!(pending.work, citadel_stacks::StackCheckWork::PendingGit(_, observed_version) if observed_version == version)
        );
        assert!(
            !store
                .claim_git_apply_retry(id, version, edited)
                .await
                .unwrap(),
            "Notify cannot claim an Apply"
        );
    }
    // Switching to automatic deployment changes the semantic inputs. A later
    // sync at the same SHA can admit one version-fenced retry using that result.
    sqlx::query("UPDATE stackreleases SET spec=jsonb_set(spec,'{UpdateBehavior}','\"StackAutoDeploy\"') WHERE id=$1")
        .bind(stack.current_stack_release_id).execute(pool).await.unwrap();
    assert!(
        store
            .pending_git_action(id, version, edited)
            .await
            .unwrap()
            .is_none(),
        "stale pending alert is discarded after an edit"
    );
    let auto = candidate(store, id).await.unwrap();
    let stack = store.get_authorized(actor, true, id).await.unwrap();
    let next = scanner.scan_cached(&stack, &cancel).await.unwrap();
    store
        .save_update_check(actor, true, &stack, &next, Some(auto))
        .await
        .unwrap();
    git.request_sync(actor, *git_repo_id, Some(branch))
        .await
        .unwrap();
    assert!(git.process_one(&cancel).await.unwrap());
    let version = stack.row_version + 1;
    assert!(
        !store
            .claim_git_apply_retry(id, version - 1, auto)
            .await
            .unwrap()
    );
    assert!(
        !store
            .claim_git_apply_retry(id, version, edited)
            .await
            .unwrap()
    );
    assert!(
        store
            .claim_git_apply_retry(id, version, auto)
            .await
            .unwrap()
    );
    assert!(
        !store
            .claim_git_apply_retry(id, version, auto)
            .await
            .unwrap()
    );
    assert!(
        !store
            .update_check_candidates(Uuid::nil(), false, 100, Some(reference))
            .await
            .unwrap()
            .iter()
            .any(|row| row.id == id)
    );
    sqlx::query("UPDATE stackreleases SET spec=$2 WHERE id=$1")
        .bind(stack.current_stack_release_id)
        .bind(spec)
        .execute(pool)
        .await
        .unwrap();
}
