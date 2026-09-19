use std::time::Duration;

use citadel_adapters::docker::DockerClient;
use citadel_adapters::postgres::stacks::PostgresStackRepository;
use citadel_adapters::stack_runtime::StackRuntimeRouter;
use citadel_database::MigrationRunner;
use citadel_domain::ActorId;
use citadel_identity::SYSTEM_ACTOR_ID;
use citadel_stacks::{
    ComposeProjectRuntimeService, CreateStack, ImportComposeProject, StackDriftPolicy, StackFilter,
    StackImportClaim, StackImportKind, StackReleaseSource, StackReleaseStatus, StackRepository,
    StackRuntime, StackRuntimeResult, StackSource, StackSpec, StackSpecCommon, StackUpdateBehavior,
    UpdateStack,
};
use sqlx::postgres::PgPoolOptions;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

#[tokio::test]
#[ignore = "requires CITADEL_PHASE6_DATABASE_URL"]
async fn stack_crud_releases_rollback_import_and_delete_are_transactional() {
    let database_url = std::env::var("CITADEL_PHASE6_DATABASE_URL")
        .expect("CITADEL_PHASE6_DATABASE_URL is required for this fixture");
    MigrationRunner::migrate(&database_url).await.unwrap();
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .connect(&database_url)
        .await
        .unwrap();
    let store = PostgresStackRepository::new(pool.clone());
    let actor = ActorId::new(SYSTEM_ACTOR_ID);
    let suffix = Uuid::now_v7().simple().to_string();
    let platform_id = Uuid::now_v7();
    sqlx::query(
        r#"INSERT INTO platforms(id,address,connectortype,cpucount,imagecount,memtotal,name,networkcount,platformdescriptor,status,volumecount)
           VALUES($1,'unix:///var/run/docker.sock','Local',1,0,1048576,$2,0,'{"$type":"Docker"}'::json,'Online',0)"#,
    )
    .bind(platform_id)
    .bind(format!("stack-platform-{suffix}"))
    .execute(&pool)
    .await
    .unwrap();

    let created = store
        .create(
            actor,
            true,
            &CreateStack {
                name: format!("stack-{suffix}"),
                platform_id,
                description: Some("created".to_owned()),
                stack_source: StackSource::WebEditor,
                spec: spec("nginx:alpine", None),
                drift_policy: Some(StackDriftPolicy::default()),
                tag_ids: Vec::new(),
                duplicate_source: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(created.status, StackReleaseStatus::Created);
    assert_eq!(created.platform_id, Some(platform_id));
    assert_eq!(
        store
            .list_authorized(
                actor,
                true,
                &StackFilter {
                    platform_id: Some(platform_id),
                    ..Default::default()
                },
            )
            .await
            .unwrap()
            .len(),
        1
    );

    let cleared = store
        .update_metadata(actor, true, created.id, None)
        .await
        .unwrap();
    assert_eq!(cleared.description, None, "explicit null clears metadata");

    let reader = Uuid::now_v7();
    sqlx::query("INSERT INTO actors(id,isenabled,type) VALUES($1,TRUE,'User')")
        .bind(reader)
        .execute(&pool)
        .await
        .unwrap();
    assert!(
        store
            .get_authorized(ActorId::new(reader), false, created.id)
            .await
            .is_err()
    );
    sqlx::query(
        "INSERT INTO resourceaccesses(id,actorid,permissionlevel,resourceid,resourcetype,specificpermissions) VALUES($1,$2,1,$3,2,0)",
    )
    .bind(Uuid::now_v7())
    .bind(reader)
    .bind(created.id)
    .execute(&pool)
    .await
    .unwrap();
    let readable = store
        .get_authorized(ActorId::new(reader), false, created.id)
        .await
        .unwrap();
    use citadel_domain::PermissionPolicy;
    assert!(
        readable
            .effective_permission
            .allows(citadel_stacks::permissions::ReadStack::REQUIREMENT)
    );
    assert!(
        !readable
            .effective_permission
            .allows(citadel_stacks::permissions::WriteStack::REQUIREMENT)
    );

    let first_claim = store
        .claim_apply(actor, true, created.id, None, None)
        .await
        .unwrap();
    assert!(matches!(
        store.claim_apply(actor, true, created.id, None, None).await,
        Err(citadel_stacks::StackError::Conflict(_))
    ));
    store
        .complete_apply(
            actor,
            &first_claim,
            &StackRuntimeResult {
                status: StackReleaseStatus::Healthy,
                messages: Vec::new(),
            },
            &[],
            Some(&StackReleaseSource {
                source_type: StackSource::WebEditor,
                git_repository_id: None,
                git_repository_name: None,
                branch: None,
                requested_commit_sha: None,
                resolved_commit_sha: String::new(),
                compose_paths: vec!["compose.yml".to_owned()],
                env_file_paths: Vec::new(),
                git_repository_url: None,
                working_directory: None,
                watch_paths: None,
                compose_env_files_from_repo: None,
                compose_digest: Some("sha256:fixture".to_owned()),
            }),
        )
        .await
        .unwrap();
    let rolled_back = store.get_authorized(actor, true, created.id).await.unwrap();
    assert!(
        rolled_back
            .spec
            .as_ref()
            .and_then(StackSpec::compose_file)
            .is_some_and(|compose| compose.contains("nginx:alpine"))
    );
    let current = store.get_authorized(actor, true, created.id).await.unwrap();
    assert_eq!(
        store
            .drift_monitor_candidates(None, 25)
            .await
            .unwrap()
            .iter()
            .filter(|candidate| candidate.id == created.id)
            .count(),
        1,
        "an idle healthy Stack with drift detection enabled must be monitored"
    );
    assert_eq!(
        current
            .source
            .as_ref()
            .and_then(|source| source.compose_digest.as_deref()),
        Some("sha256:fixture")
    );
    let updated = store
        .update(
            actor,
            true,
            created.id,
            current.row_version,
            &UpdateStack {
                name: None,
                platform_id: None,
                description: None,
                stack_source: None,
                spec: None,
                drift_policy: None,
                row_version: Some(current.row_version),
            },
            &spec("nginx:1.27-alpine", Some("stable-project")),
        )
        .await
        .unwrap();
    assert_eq!(updated.status, StackReleaseStatus::Healthy);
    assert!(
        store
            .releases(actor, true, created.id)
            .await
            .unwrap()
            .is_empty(),
        "editing preserves the current version until the next apply"
    );
    let second_claim = store
        .claim_apply(actor, true, created.id, None, None)
        .await
        .unwrap();
    store
        .complete_apply(
            actor,
            &second_claim,
            &StackRuntimeResult {
                status: StackReleaseStatus::Healthy,
                messages: Vec::new(),
            },
            &[],
            None,
        )
        .await
        .unwrap();
    let releases = store.releases(actor, true, created.id).await.unwrap();
    assert_eq!(releases.len(), 1);
    assert!(
        releases[0]
            .spec
            .compose_file()
            .unwrap()
            .contains("nginx:alpine")
    );

    let rollback = store
        .claim_apply(actor, true, created.id, Some(releases[0].id), None)
        .await
        .unwrap();
    store
        .complete_apply(
            actor,
            &rollback,
            &StackRuntimeResult {
                status: StackReleaseStatus::Healthy,
                messages: Vec::new(),
            },
            &[],
            None,
        )
        .await
        .unwrap();

    let state_claims = store.claim_state(actor, true, &[created.id]).await.unwrap();
    assert_eq!(state_claims[0].previous_status, StackReleaseStatus::Healthy);
    assert!(matches!(
        store.claim_state(actor, true, &[created.id]).await,
        Err(citadel_stacks::StackError::Conflict(_))
    ));
    assert_eq!(
        sqlx::query_scalar::<_, String>("SELECT status FROM stackreleases WHERE id=$1")
            .bind(state_claims[0].release_id)
            .fetch_one(&pool)
            .await
            .unwrap(),
        "Pending"
    );
    store.release_state(&state_claims).await.unwrap();
    assert_eq!(
        store
            .get_authorized(actor, true, created.id)
            .await
            .unwrap()
            .status,
        StackReleaseStatus::Healthy
    );

    let state_claims = store.claim_state(actor, true, &[created.id]).await.unwrap();
    store
        .complete_state(
            actor,
            &state_claims[0],
            StackReleaseStatus::Stopped,
            &["container-one".to_owned()],
        )
        .await
        .unwrap();
    let stopped = store.get_authorized(actor, true, created.id).await.unwrap();
    assert_eq!(stopped.status, StackReleaseStatus::Stopped);
    assert_eq!(stopped.control_state, "Idle");
    assert!(
        store
            .drift_monitor_candidates(None, 25)
            .await
            .unwrap()
            .iter()
            .all(|candidate| candidate.id != created.id),
        "an intentionally stopped Stack must not be treated as drift"
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM activityevents WHERE resourceid=$1 AND eventtype='StackStopped'"
        )
        .bind(created.id)
        .fetch_one(&pool)
        .await
        .unwrap(),
        1
    );

    let failed_import_name = format!("failed-import-{suffix}");
    let failed = store
        .import(
            actor,
            true,
            &ImportComposeProject {
                name: failed_import_name.clone(),
                platform_id,
                project_name: format!("missing-project-{suffix}"),
                description: None,
                spec: spec("redis:alpine", Some("missing-project")),
                tag_ids: Vec::new(),
                import_kind: StackImportKind::ComposeProject,
                preview_fingerprint: "sha256:fixture".to_owned(),
                detected_secret_values: Default::default(),
            },
            &StackImportClaim {
                platform_id,
                platform_name: format!("stack-platform-{suffix}"),
                project_name: format!("missing-project-{suffix}"),
                import_kind: StackImportKind::ComposeProject,
                runtime_fingerprint: "sha256:fixture".to_owned(),
                service_names: vec!["redis".to_owned()],
                container_ids: vec!["missing-container".to_owned()],
                container_names: Vec::new(),
                services: Vec::new(),
            },
        )
        .await;
    assert!(failed.is_err());
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM stacks WHERE name=$1")
            .bind(failed_import_name)
            .fetch_one(&pool)
            .await
            .unwrap(),
        0,
        "failed ownership linking must roll back the Stack and initial release"
    );

    let swarm_platform_id = Uuid::now_v7();
    let namespace = format!("import-{suffix}");
    sqlx::query(
        r#"INSERT INTO platforms(id,address,connectortype,cpucount,imagecount,memtotal,name,networkcount,platformdescriptor,status,volumecount)
           VALUES($1,$2,'Local',1,0,1048576,$3,0,'{"$type":"DockerSwarm"}'::json,'Online',0)"#,
    )
    .bind(swarm_platform_id)
    .bind(format!("swarm-{suffix}"))
    .bind(format!("swarm-platform-{suffix}"))
    .execute(&pool)
    .await
    .unwrap();
    for service in ["api", "worker"] {
        sqlx::query(
            r#"INSERT INTO swarmserviceprojections(
                   platformid,dockerserviceid,configids,desiredtaskcount,dockerstacknamespace,
                   forceupdate,image,isstale,labels,mode,name,networkids,observedat,ownership,
                   ports,runningtaskcount,secretids,updatestate,versionindex)
               VALUES($1,$2,'[]',1,$3,0,'nginx:alpine',FALSE,'{}','Replicated',$4,'[]',CURRENT_TIMESTAMP,
                      'DockerStackExternal','[]',1,'[]','completed',1)"#,
        )
        .bind(swarm_platform_id)
        .bind(format!("docker-{service}-{suffix}"))
        .bind(&namespace)
        .bind(format!("{namespace}_{service}"))
        .execute(&pool)
        .await
        .unwrap();
    }
    let compose_container_id = format!("compose-container-{suffix}");
    sqlx::query(
        r#"INSERT INTO containers(
               id,created,dockercontainerid,dockerimageid,hascitadelownershiplabels,
               isswarmtask,issystem,name,platformid,ports,stack,state,updated)
           VALUES($1,1,$2,'nginx:alpine',FALSE,FALSE,FALSE,$3,$4,'[]',$5,'running',1)"#,
    )
    .bind(Uuid::now_v7())
    .bind(&compose_container_id)
    .bind(format!("/{namespace}-compose-1"))
    .bind(swarm_platform_id)
    .bind(&namespace)
    .execute(&pool)
    .await
    .unwrap();
    let runtime = StackRuntimeRouter::new(
        pool.clone(),
        DockerClient::new("unused", Duration::from_secs(1)).unwrap(),
        None,
    );
    let compose_claim = runtime
        .import_claim(
            swarm_platform_id,
            &namespace,
            Some(StackImportKind::ComposeProject),
            &CancellationToken::new(),
        )
        .await
        .unwrap();
    assert_eq!(compose_claim.import_kind, StackImportKind::ComposeProject);
    assert_eq!(compose_claim.container_ids, [compose_container_id]);
    let swarm_claim = runtime
        .import_claim(
            swarm_platform_id,
            &namespace,
            Some(StackImportKind::SwarmStack),
            &CancellationToken::new(),
        )
        .await
        .unwrap();
    assert_eq!(swarm_claim.import_kind, StackImportKind::SwarmStack);
    assert_eq!(swarm_claim.services.len(), 2);
    let imported = store
        .import(
            actor,
            true,
            &ImportComposeProject {
                name: format!("imported-{suffix}"),
                platform_id: swarm_platform_id,
                project_name: namespace.clone(),
                description: Some("Imported fixture".to_owned()),
                spec: spec("nginx:alpine", Some(&namespace)),
                tag_ids: Vec::new(),
                import_kind: StackImportKind::SwarmStack,
                preview_fingerprint: "sha256:fixture".to_owned(),
                detected_secret_values: Default::default(),
            },
            &StackImportClaim {
                platform_id: swarm_platform_id,
                platform_name: format!("swarm-platform-{suffix}"),
                project_name: namespace.clone(),
                import_kind: StackImportKind::SwarmStack,
                runtime_fingerprint: "sha256:fixture".to_owned(),
                service_names: vec!["api".to_owned(), "worker".to_owned()],
                container_ids: Vec::new(),
                container_names: Vec::new(),
                services: vec![
                    ComposeProjectRuntimeService {
                        name: "api".to_owned(),
                        image: Some("nginx:alpine".to_owned()),
                        container_count: 1,
                        states: vec!["completed".to_owned()],
                    },
                    ComposeProjectRuntimeService {
                        name: "worker".to_owned(),
                        image: Some("nginx:alpine".to_owned()),
                        container_count: 1,
                        states: vec!["completed".to_owned()],
                    },
                ],
            },
        )
        .await
        .unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM swarmserviceprojections WHERE platformid=$1 AND dockerstacknamespace=$2 AND stackid=$3"
        )
        .bind(swarm_platform_id)
        .bind(&namespace)
        .bind(imported.id)
        .fetch_one(&pool)
        .await
        .unwrap(),
        2
    );
    assert_eq!(
        store
            .find_import_owner(swarm_platform_id, &namespace)
            .await
            .unwrap(),
        Some(imported.id)
    );
    let imported_claim = store
        .claim_delete(actor, true, &[imported.id])
        .await
        .unwrap();
    store.complete_delete(actor, &imported_claim).await.unwrap();

    let git_repository_id = Uuid::now_v7();
    sqlx::query("INSERT INTO gitrepositories(id,createdbyactorid,defaultbranch,name,status,syncmode,url,controlstate) VALUES($1,$2,'main',$3,'Healthy','Manual','https://example.test/repository.git','Idle')")
        .bind(git_repository_id)
        .bind(actor.value())
        .bind(format!("stack-git-{suffix}"))
        .execute(&pool)
        .await
        .unwrap();
    let git_stack = store
        .create(
            actor,
            true,
            &CreateStack {
                name: format!("git-stack-{suffix}"),
                platform_id,
                description: None,
                stack_source: StackSource::Git,
                spec: StackSpec::Git {
                    git_repo_id: git_repository_id,
                    branch: "main".to_owned(),
                    commit_sha: None,
                    update_behavior: StackUpdateBehavior::Notify,
                    webhook: None,
                    compose_paths: vec!["compose.yml".to_owned()],
                    working_directory: None,
                    compose_env_files_from_repo: Vec::new(),
                    watch_paths: Vec::new(),
                    additional_env_file_from_repo: Vec::new(),
                    common: StackSpecCommon::default(),
                },
                drift_policy: Some(StackDriftPolicy::default()),
                tag_ids: Vec::new(),
                duplicate_source: None,
            },
        )
        .await
        .unwrap();
    let git_claim = store
        .claim_apply(actor, true, git_stack.id, None, None)
        .await
        .unwrap();
    let resolved_commit = "a".repeat(40);
    store
        .complete_apply(
            actor,
            &git_claim,
            &StackRuntimeResult {
                status: StackReleaseStatus::Healthy,
                messages: Vec::new(),
            },
            &[],
            Some(&StackReleaseSource {
                source_type: StackSource::Git,
                git_repository_id: Some(git_repository_id),
                git_repository_name: Some("repository".to_owned()),
                branch: Some("main".to_owned()),
                requested_commit_sha: None,
                resolved_commit_sha: resolved_commit.clone(),
                compose_paths: vec!["compose.yml".to_owned()],
                env_file_paths: Vec::new(),
                git_repository_url: Some("https://example.test/repository.git".to_owned()),
                working_directory: Some(".".to_owned()),
                watch_paths: Some(Vec::new()),
                compose_env_files_from_repo: Some(Vec::new()),
                compose_digest: Some("sha256:git-fixture".to_owned()),
            }),
        )
        .await
        .unwrap();
    let applied_git_stack = store
        .get_authorized(actor, true, git_stack.id)
        .await
        .unwrap();
    let citadel_stacks::StackUpdateState::Git {
        recreate_stack_on_new_commit_state,
        ..
    } = applied_git_stack.stack.stack_update_state
    else {
        panic!("Git Stack must retain Git update state");
    };
    assert_eq!(
        recreate_stack_on_new_commit_state.current_commit_sha,
        resolved_commit
    );
    assert!(
        recreate_stack_on_new_commit_state
            .remote_commit_sha
            .is_none()
    );
    let git_delete = store
        .claim_delete(actor, true, &[git_stack.id])
        .await
        .unwrap();
    store.complete_delete(actor, &git_delete).await.unwrap();
    sqlx::query("DELETE FROM gitrepositories WHERE id=$1")
        .bind(git_repository_id)
        .execute(&pool)
        .await
        .unwrap();

    let claims = store
        .claim_delete(actor, true, &[created.id])
        .await
        .unwrap();
    store.complete_delete(actor, &claims).await.unwrap();
    assert!(store.get_authorized(actor, true, created.id).await.is_err());
    sqlx::query("DELETE FROM actors WHERE id=$1")
        .bind(reader)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM platforms WHERE id=$1")
        .bind(platform_id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM platforms WHERE id=$1")
        .bind(swarm_platform_id)
        .execute(&pool)
        .await
        .unwrap();
}

fn spec(image: &str, project_name: Option<&str>) -> StackSpec {
    StackSpec::WebEditor {
        compose_file: format!("services:\n  web:\n    image: {image}\n"),
        update_behavior: StackUpdateBehavior::Disabled,
        common: StackSpecCommon {
            project_name: project_name.map(str::to_owned),
            destroy_before_deploy: false,
            ..Default::default()
        },
    }
}
