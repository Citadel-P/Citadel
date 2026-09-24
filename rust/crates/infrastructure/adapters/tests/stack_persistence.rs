use serde_json::json;
use std::time::Duration;

use citadel_adapters::connectors::docker::DockerClient;
use citadel_adapters::connectors::routing::stacks::StackRuntimeRouter;
use citadel_adapters::persistence::postgres::stacks::PostgresStackRepository;
use citadel_database::MigrationRunner;
use citadel_identity::SYSTEM_ACTOR_ID;
use citadel_primitives::ActorId;
use citadel_stacks::{
    CreateStack, ImportComposeProject, StackDriftPolicy, StackFilter, StackImportClaim,
    StackImportKind, StackReleaseSource, StackReleaseStatus, StackRepository, StackRuntime,
    StackRuntimeResult, StackSource, StackSpec, StackSpecCommon, StackUpdateBehavior, UpdateStack,
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
    // Existing standalone platforms must remain valid for Stack CRUD.
    sqlx::query(
        r#"INSERT INTO platforms(id,address,connectortype,cpucount,imagecount,memtotal,name,networkcount,platformdescriptor,status,volumecount)
           VALUES($1,'unix:///var/run/docker.sock','Local',1,0,1048576,$2,0,'{"$type":"DockerStandalone"}'::json,'Online',0)"#,
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
    use citadel_primitives::PermissionPolicy;
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
    sqlx::query("INSERT INTO containers(id,created,dockercontainerid,dockerimageid,name,platformid,ports,stackid,state,updated) VALUES($1,0,'container-one','image','web',$2,'[]',$3,'Running',0)")
        .bind(Uuid::now_v7()).bind(platform_id).bind(created.id).execute(&pool).await.unwrap();
    // A stop event can arrive before the Agent sends the RPC response.
    citadel_adapters::persistence::postgres::platforms::status::container_event(
        &pool,
        platform_id,
        None,
        "container-one",
        Some("exited"),
        None,
        chrono::Utc::now().timestamp(),
    )
    .await
    .unwrap();
    let pending = store.get_authorized(actor, true, created.id).await.unwrap();
    assert_eq!(pending.status, StackReleaseStatus::Pending);
    assert_eq!(pending.control_state, "Processing");
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
                orphaned_owner_id: None,
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
    let old_owner = Uuid::now_v7();
    let docker = ImportDocker::new(json!([{
        "Id": compose_container_id, "Names": [format!("/{namespace}-compose-1")],
        "Image":"nginx:alpine", "ImageID":"sha256:image", "State":"running",
        "Labels": {"com.docker.compose.project":namespace, "com.docker.compose.service":"web",
            "com.citadel.managed":"true", "com.citadel.stack-id":old_owner}
    }]))
    .await;
    let runtime = StackRuntimeRouter::new(pool.clone(), docker.client.clone(), None);
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
    assert_eq!(
        compose_claim.container_ids.as_slice(),
        std::slice::from_ref(&compose_container_id)
    );
    assert_eq!(compose_claim.service_names, ["web"]);
    assert_eq!(compose_claim.orphaned_owner_id, Some(old_owner));
    for labels in [
        json!({"com.citadel.managed":"true", "com.citadel.stack-id": created.id}),
        json!({"com.citadel.managed":"true", "com.citadel.stack-id": "invalid"}),
        json!({"com.citadel.managed":"true", "com.citadel.deployment-id": old_owner}),
    ] {
        let mut document = docker.document.write().await;
        document[0]["Labels"] = labels;
        document[0]["Labels"]["com.docker.compose.project"] = json!(namespace);
        document[0]["Labels"]["com.docker.compose.service"] = json!("web");
        drop(document);
        assert!(
            runtime
                .import_claim(
                    swarm_platform_id,
                    &namespace,
                    Some(StackImportKind::ComposeProject),
                    &CancellationToken::new()
                )
                .await
                .is_err()
        );
    }
    let compose_input = ImportComposeProject {
        name: format!("compose-import-{suffix}"),
        platform_id: swarm_platform_id,
        project_name: namespace.clone(),
        description: None,
        spec: spec("nginx:alpine", Some(&namespace)),
        tag_ids: Vec::new(),
        import_kind: StackImportKind::ComposeProject,
        preview_fingerprint: "fixture".into(),
        detected_secret_values: Default::default(),
    };
    // Commit must recheck a claim's owner and the container's current eligibility.
    let mut active_owner_claim = compose_claim.clone();
    active_owner_claim.orphaned_owner_id = Some(created.id);
    assert!(
        store
            .import(actor, true, &compose_input, &active_owner_claim)
            .await
            .is_err()
    );
    for (state, system) in [("Processing", false), ("Idle", true)] {
        sqlx::query("UPDATE containers SET controlstate=$2,issystem=$3 WHERE dockercontainerid=$1")
            .bind(&compose_container_id)
            .bind(state)
            .bind(system)
            .execute(&pool)
            .await
            .unwrap();
        assert!(
            store
                .import(actor, true, &compose_input, &compose_claim)
                .await
                .is_err()
        );
    }
    sqlx::query(
        "UPDATE containers SET controlstate='Idle',issystem=false WHERE dockercontainerid=$1",
    )
    .bind(&compose_container_id)
    .execute(&pool)
    .await
    .unwrap();
    let compose_imported = store
        .import(actor, true, &compose_input, &compose_claim)
        .await
        .unwrap();
    assert!(
        store
            .import(actor, true, &compose_input, &compose_claim)
            .await
            .is_err()
    );
    let compose_delete = store
        .claim_delete(actor, true, &[compose_imported.id])
        .await
        .unwrap();
    store.complete_delete(actor, &compose_delete).await.unwrap();
    // Native stacks accept one orphaned owner, but reject mixed and existing owners.
    for owner in [created.id, old_owner] {
        let labels = json!({"com.citadel.managed":"true", "com.citadel.stack-id":owner});
        sqlx::query("UPDATE swarmserviceprojections SET labels=$2 WHERE platformid=$1")
            .bind(swarm_platform_id)
            .bind(labels)
            .execute(&pool)
            .await
            .unwrap();
        let claim = runtime
            .import_claim(
                swarm_platform_id,
                &namespace,
                Some(StackImportKind::SwarmStack),
                &CancellationToken::new(),
            )
            .await;
        if owner == created.id {
            assert!(claim.is_err());
        } else {
            assert_eq!(claim.unwrap().orphaned_owner_id, Some(old_owner));
        }
    }
    sqlx::query("UPDATE swarmserviceprojections SET labels='{}' WHERE platformid=$1 AND name=$2")
        .bind(swarm_platform_id)
        .bind(format!("{namespace}_worker"))
        .execute(&pool)
        .await
        .unwrap();
    assert!(
        runtime
            .import_claim(
                swarm_platform_id,
                &namespace,
                Some(StackImportKind::SwarmStack),
                &CancellationToken::new()
            )
            .await
            .is_err()
    );
    sqlx::query("UPDATE swarmserviceprojections SET labels=$2 WHERE platformid=$1")
        .bind(swarm_platform_id)
        .bind(json!({"com.citadel.managed":"true","com.citadel.stack-id":old_owner}))
        .execute(&pool)
        .await
        .unwrap();
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
            &swarm_claim,
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

struct ImportDocker {
    client: DockerClient,
    document: std::sync::Arc<tokio::sync::RwLock<serde_json::Value>>,
    task: tokio::task::JoinHandle<()>,
    path: std::path::PathBuf,
}
impl ImportDocker {
    async fn new(document: serde_json::Value) -> Self {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let path = std::env::temp_dir().join(format!("stack-import-{}.sock", Uuid::now_v7()));
        let listener = tokio::net::UnixListener::bind(&path).unwrap();
        let document = std::sync::Arc::new(tokio::sync::RwLock::new(document));
        let response = document.clone();
        let task = tokio::spawn(async move {
            loop {
                let (mut stream, _) = listener.accept().await.unwrap();
                let mut bytes = Vec::new();
                let mut buffer = [0; 2048];
                while !bytes.windows(4).any(|window| window == b"\r\n\r\n") {
                    let n = stream.read(&mut buffer).await.unwrap();
                    assert!(n > 0 && bytes.len() < 8192);
                    bytes.extend_from_slice(&buffer[..n]);
                }
                let request = String::from_utf8(bytes).unwrap();
                assert!(request.starts_with("GET "), "Import must not mutate Docker");
                let body = if request.starts_with("GET /version ") {
                    json!({"ApiVersion":"1.49","MinAPIVersion":"1.41"})
                } else {
                    assert!(request.starts_with("GET /v1.49/containers/json?"));
                    response.read().await.clone()
                }
                .to_string();
                stream.write_all(format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).as_bytes()).await.unwrap();
            }
        });
        Self {
            client: DockerClient::new(&path, Duration::from_secs(2)).unwrap(),
            document,
            task,
            path,
        }
    }
}
impl Drop for ImportDocker {
    fn drop(&mut self) {
        self.task.abort();
        let _ = std::fs::remove_file(&self.path);
    }
}

async fn concurrency_fixture() -> (
    sqlx::PgPool,
    PostgresStackRepository,
    ActorId,
    citadel_stacks::StackDetails,
) {
    let url = std::env::var("CITADEL_PHASE6_DATABASE_URL").unwrap();
    MigrationRunner::migrate(&url).await.unwrap();
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .connect(&url)
        .await
        .unwrap();
    let store = PostgresStackRepository::new(pool.clone());
    let actor = ActorId::new(SYSTEM_ACTOR_ID);
    let platform = Uuid::now_v7();
    sqlx::query("INSERT INTO platforms(id,address,connectortype,cpucount,imagecount,memtotal,name,networkcount,platformdescriptor,status,volumecount) VALUES($1,$2,'Local',1,0,1024,$2,0,'{\"$type\":\"Docker\"}','Online',0)")
        .bind(platform).bind(format!("stack-concurrency-{platform}")).execute(&pool).await.unwrap();
    let stack = store
        .create(
            actor,
            true,
            &CreateStack {
                name: format!("stack-{}", Uuid::now_v7()),
                platform_id: platform,
                description: None,
                stack_source: StackSource::WebEditor,
                spec: spec("nginx:alpine", None),
                drift_policy: None,
                tag_ids: vec![],
                duplicate_source: None,
            },
        )
        .await
        .unwrap();
    (pool, store, actor, stack)
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE6_DATABASE_URL"]
async fn stack_completion_allows_inventory_and_is_idempotent() {
    let (pool, store, actor, stack) = concurrency_fixture().await;
    let claim = store
        .claim_apply(actor, true, stack.id, None, None)
        .await
        .unwrap();
    let result = StackRuntimeResult {
        status: StackReleaseStatus::Healthy,
        messages: vec![],
    };
    let mut inventory = pool.begin().await.unwrap();
    sqlx::query("SELECT id FROM platforms WHERE id=$1 FOR NO KEY UPDATE")
        .bind(claim.platform_id)
        .fetch_one(&mut *inventory)
        .await
        .unwrap();
    // An uncommitted observation takes a foreign-key KEY SHARE lock on Stack.
    sqlx::query("INSERT INTO containers(id,created,dockercontainerid,dockerimageid,name,platformid,ports,stackid,state,updated) VALUES($1,0,$2,'image','web',$3,'[]',$4,'Running',0)")
        .bind(Uuid::now_v7()).bind(format!("container-{}", stack.id)).bind(claim.platform_id).bind(stack.id).execute(&mut *inventory).await.unwrap();
    tokio::time::timeout(
        Duration::from_secs(5),
        store.complete_apply(actor, &claim, &result, &[], None),
    )
    .await
    .unwrap()
    .unwrap();
    inventory.commit().await.unwrap();
    store
        .complete_apply(actor, &claim, &result, &[], None)
        .await
        .unwrap();
    let count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM activityevents WHERE resourceid=$1 AND eventtype='StackApplied'",
    )
    .bind(stack.id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(count, 1);
    let current = store.get_authorized(actor, true, stack.id).await.unwrap();
    assert_eq!(current.status, StackReleaseStatus::Healthy);
    assert_eq!(current.control_state, "Idle");
    let next = store
        .claim_apply(actor, true, stack.id, None, None)
        .await
        .unwrap();
    assert!(matches!(
        store
            .complete_apply(actor, &claim, &result, &[], None)
            .await,
        Err(citadel_stacks::StackError::Conflict(_))
    ));
    store
        .complete_apply(actor, &next, &result, &[], None)
        .await
        .unwrap();
    pool.close().await;
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE6_DATABASE_URL"]
async fn stack_deletion_waits_for_inventory_without_locking_its_parent() {
    let (pool, store, actor, stack) = concurrency_fixture().await;
    let claims = store.claim_delete(actor, true, &[stack.id]).await.unwrap();
    let container = Uuid::now_v7();
    sqlx::query("INSERT INTO containers(id,created,dockercontainerid,dockerimageid,name,platformid,ports,stackid,state,updated) VALUES($1,0,$2,'image','web',$3,'[]',$4,'Running',0)")
        .bind(container).bind(format!("delete-{}", stack.id)).bind(claims[0].platform_id).bind(stack.id).execute(&pool).await.unwrap();
    let mut inventory = pool.begin().await.unwrap();
    let pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(&mut *inventory)
        .await
        .unwrap();
    sqlx::query("SELECT id FROM platforms WHERE id=$1 FOR NO KEY UPDATE")
        .bind(claims[0].platform_id)
        .fetch_one(&mut *inventory)
        .await
        .unwrap();
    // Deletion must wait before owning Stack: its FK cleanup needs the
    // existing container row that inventory is already updating.
    sqlx::query("UPDATE containers SET state='Exited' WHERE id=$1")
        .bind(container)
        .execute(&mut *inventory)
        .await
        .unwrap();
    let completion = store.complete_delete(actor, &claims);
    tokio::pin!(completion);
    tokio::select! {
        result = &mut completion => panic!("deletion bypassed inventory: {result:?}"),
        () = async {
            tokio::time::timeout(Duration::from_secs(5), async {
                loop {
                    let waiting: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE datname=current_database() AND $1=ANY(pg_blocking_pids(pid)))").bind(pid).fetch_one(&pool).await.unwrap();
                    if waiting { break; }
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
            }).await.unwrap();
        } => {}
    }
    sqlx::query("SELECT id FROM stacks WHERE id=$1 FOR NO KEY UPDATE NOWAIT")
        .bind(stack.id)
        .fetch_one(&mut *inventory)
        .await
        .unwrap();
    inventory.commit().await.unwrap();
    tokio::time::timeout(Duration::from_secs(5), completion)
        .await
        .unwrap()
        .unwrap();
    let linked: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM containers WHERE stackid=$1)")
            .bind(stack.id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(!linked);
    assert!(matches!(
        store.get_authorized(actor, true, stack.id).await,
        Err(citadel_stacks::StackError::NotFound)
    ));
    pool.close().await;
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE6_DATABASE_URL"]
async fn container_completion_locks_stack_before_release() {
    use citadel_adapters::persistence::postgres::platforms::containers::repository::PostgresContainerRepository;
    use citadel_platforms::containers::ContainerRepository;
    let (pool, _, _, stack) = concurrency_fixture().await;
    let store = PostgresContainerRepository::new(pool.clone());
    for abandon in [false, true] {
        let operation = Uuid::now_v7();
        sqlx::query(
            "UPDATE stacks SET containeroperationid=$2,controlstate='Processing' WHERE id=$1",
        )
        .bind(stack.id)
        .bind(operation)
        .execute(&pool)
        .await
        .unwrap();
        let mut request = pool.begin().await.unwrap();
        let pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
            .fetch_one(&mut *request)
            .await
            .unwrap();
        sqlx::query("SELECT id FROM stacks WHERE id=$1 FOR NO KEY UPDATE")
            .bind(stack.id)
            .fetch_one(&mut *request)
            .await
            .unwrap();
        let completion = if abandon {
            store.abandon(operation)
        } else {
            store.finish(operation)
        };
        tokio::pin!(completion);
        tokio::select! {
            result = &mut completion => panic!("completion bypassed Stack: {result:?}"),
            () = async {
                tokio::time::timeout(Duration::from_secs(5), async {
                    loop {
                        let waiting: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE datname=current_database() AND $1=ANY(pg_blocking_pids(pid)))")
                            .bind(pid).fetch_one(&pool).await.unwrap();
                        if waiting { break; }
                        tokio::time::sleep(Duration::from_millis(10)).await;
                    }
                }).await.unwrap();
            } => {}
        }
        // Completion must not own the child while waiting for its parent.
        sqlx::query("SELECT id FROM stackreleases WHERE stackid=$1 FOR NO KEY UPDATE NOWAIT")
            .bind(stack.id)
            .fetch_all(&mut *request)
            .await
            .unwrap();
        request.commit().await.unwrap();
        tokio::time::timeout(Duration::from_secs(5), completion)
            .await
            .unwrap()
            .unwrap();
        let state: String = sqlx::query_scalar("SELECT controlstate FROM stacks WHERE id=$1")
            .bind(stack.id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(state, "Idle");
    }
    pool.close().await;
}
