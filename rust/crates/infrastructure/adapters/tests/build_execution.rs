use chrono::{Duration, Utc};
use citadel_adapters::external::builds::executor::LocalDockerBuildExecutor;
use citadel_adapters::persistence::postgres::builds::PostgresBuildRepository;
use citadel_adapters::persistence::postgres::builds::credentials::PostgresBuildRegistryCredentialResolver;
use citadel_adapters::persistence::postgres::builds::credentials::PostgresBuildSecretResolver;
use citadel_adapters::persistence::postgres::stacks::build_images::PostgresStackBuildImageResolver;
use citadel_adapters::security::identity::crypto::AesGcmSecretProtector;
use citadel_bindings::SecretProtector;
use citadel_builds::{
    BuildExecutionResult, BuildExecutor, BuildProjectConfiguration,
    BuildRegistryCredentialResolver, BuildRepository, BuildSecretResolver,
};
use citadel_database::MigrationRunner;
use citadel_primitives::ActorId;
use citadel_stacks::{StackBuildImageBinding, StackBuildImageResolverPort};
use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

#[tokio::test]
#[ignore = "requires CITADEL_PHASE7_DATABASE_URL"]
async fn build_runs_claim_once_persist_results_cancel_and_recover() {
    let url = std::env::var("CITADEL_PHASE7_DATABASE_URL").unwrap();
    MigrationRunner::migrate(&url).await.unwrap();
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .connect(&url)
        .await
        .unwrap();
    let actor = ActorId::new(Uuid::now_v7());
    let platform = Uuid::now_v7();
    let registry = Uuid::now_v7();
    let repository = Uuid::now_v7();
    let tag = Uuid::now_v7();
    sqlx::query("INSERT INTO actors(id,isenabled,type) VALUES($1,true,'System')")
        .bind(actor.value())
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO tags(id,name,normalizedname,color,createdbyactorid) VALUES($1,$2,lower($2),'#112233',$3)")
        .bind(tag)
        .bind(format!("build-tag-{}", tag.simple()))
        .bind(actor.value())
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO platforms(id,address,connectortype,cpucount,imagecount,memtotal,name,networkcount,platformdescriptor,status,volumecount) VALUES($1,'http://localhost/' || $1::text,'Local',0,0,0,$2,0,'{\"$type\":\"DockerStandalone\"}','Online',0)").bind(platform).bind(format!("build-platform-{}",platform.simple())).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO registries(id,configuration,createdbyactorid,name,registryhost,status) VALUES($1,'{}'::json,$2,$3,'docker.io','Enabled')").bind(registry).bind(actor.value()).bind(format!("build-registry-{}",registry.simple())).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO gitrepositories(id,createdbyactorid,defaultbranch,name,status,syncmode,url,controlstate) VALUES($1,$2,'main',$3,'Healthy','Manual','https://example.test/repo.git','Idle')").bind(repository).bind(actor.value()).bind(format!("build-repo-{}",repository.simple())).execute(&pool).await.unwrap();
    let store = PostgresBuildRepository::new(pool.clone());
    let mut input = BuildProjectConfiguration {
        name: format!("build-{}", Uuid::now_v7().simple()),
        description: None,
        enabled: true,
        git_repository_id: repository,
        branch: Some("main".into()),
        context_path: None,
        dockerfile_path: None,
        target: None,
        build_args: None,
        build_secrets: None,
        platform_id: Some(platform),
        registry_id: registry,
        image_repository: "citadel/test".into(),
        tag_templates: None,
        webhook: None,
        timeout_seconds: Some(60),
        retention_run_count: Some(2),
        tag_ids: vec![tag],
        builder_kind: "Platform".into(),
        build_agent_pool_id: None,
    };
    input.validate().unwrap();
    let project = store.create(actor, &input).await.unwrap();
    let persisted_tags: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM resourcetags WHERE resourcetype='Build' AND resourceid=$1 AND tagid=$2",
    )
    .bind(project.id)
    .bind(tag)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(persisted_tags, 1);
    input.name = format!("invalid-tag-build-{}", Uuid::now_v7().simple());
    input.tag_ids = vec![Uuid::now_v7()];
    assert!(matches!(
        store.create(actor, &input).await,
        Err(citadel_builds::BuildError::Validation(_))
    ));
    let rolled_back: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM buildprojects WHERE normalizedname=$1)")
            .bind(input.name.to_uppercase())
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(!rolled_back);
    let queued = store.enqueue(actor, project.id, "Manual").await.unwrap();
    let claim = store
        .claim_next(Utc::now() - Duration::minutes(5))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(claim.run.id, queued.id);
    assert!(
        store
            .claim_next(Utc::now() - Duration::minutes(5))
            .await
            .unwrap()
            .is_none()
    );
    // A failed audit write rolls back the result and leaves the claim active.
    // The constraint is scoped to this unique Project in the disposable DB.
    let constraint = format!("reject_build_audit_{}", project.id.simple());
    let ddl = format!(
        "ALTER TABLE activityevents ADD CONSTRAINT {constraint} CHECK (resourceid <> '{}' OR eventtype <> 'BuildRunSucceeded') NOT VALID",
        project.id
    );
    // Both substitutions are locally generated, canonical UUIDs, never input.
    sqlx::raw_sql(sqlx::AssertSqlSafe(ddl.as_str()))
        .execute(&pool)
        .await
        .unwrap();
    assert!(store.finish(&claim, &success_result('a')).await.is_err());
    assert_eq!(store.get_run(queued.id).await.unwrap().status, "Preparing");
    assert_eq!(
        store.get(project.id).await.unwrap().current_run_id,
        Some(queued.id)
    );
    sqlx::raw_sql(sqlx::AssertSqlSafe(
        format!("ALTER TABLE activityevents DROP CONSTRAINT {constraint}").as_str(),
    ))
    .execute(&pool)
    .await
    .unwrap();
    store
        .finish(
            &claim,
            &BuildExecutionResult {
                status: "Succeeded",
                exit_code: Some(0),
                image_digest: Some(format!("sha256:{}", "a".repeat(64))),
                resolved_commit_sha: Some("b".repeat(40)),
                image_references: vec!["citadel/test:main".into()],
                error_code: None,
                error_message: None,
                logs: vec![],
            },
        )
        .await
        .unwrap();
    assert_eq!(store.get_run(queued.id).await.unwrap().status, "Succeeded");
    // Verify queued/started/terminal Activities. Late completion
    // must neither alter the result nor duplicate the terminal Activity.
    assert!(!store.finish(&claim, &success_result('a')).await.unwrap());
    let events: Vec<String> = sqlx::query_scalar(
        "SELECT eventtype FROM activityevents WHERE resourceid=$1 ORDER BY createdat,id",
    )
    .bind(project.id)
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(
        events,
        [
            "BuildCreated",
            "BuildRunQueued",
            "BuildRunStarted",
            "BuildRunSucceeded"
        ]
    );
    let stack_image = PostgresStackBuildImageResolver::new(pool.clone())
        .resolve(&[StackBuildImageBinding {
            service_name: "api".to_owned(),
            build_project_id: project.id,
            redeploy_on_build: false,
            resolved_image_reference: None,
            resolved_digest: None,
            resolved_build_run_id: None,
            applied_image_reference: None,
            applied_digest: None,
            applied_build_run_id: None,
            applied_at: None,
        }])
        .await
        .unwrap();
    assert_eq!(stack_image[0].build_run_id, Some(queued.id));
    assert_eq!(
        stack_image[0].image_reference,
        format!("citadel/test@sha256:{}", "a".repeat(64))
    );
    let cancelled = store.enqueue(actor, project.id, "Manual").await.unwrap();
    assert!(store.cancel_queued(cancelled.id).await.unwrap());
    assert_eq!(
        store.get_run(cancelled.id).await.unwrap().status,
        "Cancelled"
    );
    let interrupted = store.enqueue(actor, project.id, "Manual").await.unwrap();
    let _ = store
        .claim_next(Utc::now() - Duration::minutes(5))
        .await
        .unwrap()
        .unwrap();
    sqlx::query("UPDATE buildruns SET startedat=CURRENT_TIMESTAMP-INTERVAL '1 hour' WHERE id=$1")
        .bind(interrupted.id)
        .execute(&pool)
        .await
        .unwrap();
    assert!(
        store
            .claim_next(Utc::now() - Duration::minutes(5))
            .await
            .unwrap()
            .is_none()
    );
    assert_eq!(
        store.get_run(interrupted.id).await.unwrap().status,
        "Interrupted"
    );
    let interrupted_activity: serde_json::Value = sqlx::query_scalar(
        "SELECT info::jsonb FROM activityevents WHERE resourceid=$1 AND eventtype='BuildRunFailed'",
    )
    .bind(project.id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(interrupted_activity["Status"], "Interrupted");
    for marker in ['c', 'd'] {
        let run = store.enqueue(actor, project.id, "Manual").await.unwrap();
        let claim = store
            .claim_next(Utc::now() - Duration::minutes(5))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(claim.run.id, run.id);
        store.finish(&claim, &success_result(marker)).await.unwrap();
    }
    let retained: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM buildruns WHERE buildprojectid=$1")
            .bind(project.id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(retained, 2);
    // Build queue parity: different Projects cannot race past a Pool's limit.
    let mut pool_input: citadel_builds::BuildAgentPoolConfiguration = serde_json::from_value(serde_json::json!({
        "name":format!("pool-{}",Uuid::now_v7()),"enabled":true,
        "providerSpec":{"$type":"SelfManagedVm","connectionMode":"EdgeAgent"},"maxActiveBuilders":1
    })).unwrap();
    pool_input.validate().unwrap();
    let build_pool = store.create_pool(actor, &pool_input).await.unwrap();
    input.tag_ids.clear();
    input.platform_id = None;
    input.builder_kind = "BuildAgentPool".into();
    input.build_agent_pool_id = Some(build_pool.id);
    let mut pool_projects = Vec::new();
    for _ in 0..2 {
        input.name = format!("pool-build-{}", Uuid::now_v7());
        let project = store.create(actor, &input).await.unwrap();
        let patched = project
            .apply_patch(
                serde_json::json!({"dockerfilePath":"docker/Production"}),
                false,
            )
            .unwrap();
        assert_eq!(patched.git_repository_id, project.git_repository_id);
        assert_eq!(patched.branch.as_deref(), Some(project.branch.as_str()));
        assert_eq!(
            patched.dockerfile_path.as_deref(),
            Some("docker/Production")
        );
        assert!(
            project
                .apply_patch(serde_json::json!({"name":"wrong-route"}), false)
                .is_err()
        );
        assert!(
            project
                .apply_patch(serde_json::json!({"enabled":false}), true)
                .is_err()
        );
        store.enqueue(actor, project.id, "Manual").await.unwrap();
        pool_projects.push(project.id);
    }
    let (a, b) = tokio::join!(
        store.claim_next(Utc::now() - Duration::minutes(5)),
        store.claim_next(Utc::now() - Duration::minutes(5))
    );
    let claimed = [a.unwrap(), b.unwrap()]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
    assert_eq!(claimed.len(), 1);
    assert!(
        store
            .claim_next(Utc::now() - Duration::minutes(5))
            .await
            .unwrap()
            .is_none()
    );
    store
        .finish(&claimed[0], &success_result('e'))
        .await
        .unwrap();
    let next = store
        .claim_next(Utc::now() - Duration::minutes(5))
        .await
        .unwrap()
        .unwrap();
    assert_ne!(next.project.id, claimed[0].project.id);
    store.finish(&next, &success_result('f')).await.unwrap();
    sqlx::query("DELETE FROM buildruns WHERE buildprojectid=ANY($1)")
        .bind(&pool_projects)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM buildprojects WHERE id=ANY($1)")
        .bind(&pool_projects)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM buildagentpools WHERE id=$1")
        .bind(build_pool.id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE platforms SET connectortype='Agent' WHERE id=$1")
        .bind(platform)
        .execute(&pool)
        .await
        .unwrap();
    let target_guard_protector = Arc::new(AesGcmSecretProtector::new(&[91_u8; 32]).unwrap());
    let executor = LocalDockerBuildExecutor::new(
        std::env::temp_dir().join(Uuid::now_v7().to_string()),
        "docker-command-must-not-run",
        Arc::new(PostgresBuildSecretResolver::new(pool.clone(), target_guard_protector).unwrap()),
        Arc::new(PostgresBuildRegistryCredentialResolver::new(pool.clone())),
        4096,
        pool.clone(),
    );
    let rejected = executor
        .execute(
            &claim,
            &citadel_builds::NoopBuildLogSink,
            &CancellationToken::new(),
        )
        .await;
    assert_eq!(rejected.status, "Failed");
    assert!(
        rejected
            .error_message
            .as_deref()
            .is_some_and(|message| message.contains("requires Agent execution"))
    );
    sqlx::query("DELETE FROM buildrunlogs WHERE buildrunid IN (SELECT id FROM buildruns WHERE buildprojectid=$1)").bind(project.id).execute(&pool).await.unwrap();
    sqlx::query("DELETE FROM buildruns WHERE buildprojectid=$1")
        .bind(project.id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM resourcetags WHERE resourcetype='Build' AND resourceid=$1")
        .bind(project.id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM buildprojects WHERE id=$1")
        .bind(project.id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM tags WHERE id=$1")
        .bind(tag)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM gitrepositories WHERE id=$1")
        .bind(repository)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM registries WHERE id=$1")
        .bind(registry)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM platforms WHERE id=$1")
        .bind(platform)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM activityevents WHERE createdbyactorid=$1")
        .bind(actor.value())
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM actors WHERE id=$1")
        .bind(actor.value())
        .execute(&pool)
        .await
        .unwrap();
}

fn success_result(marker: char) -> BuildExecutionResult {
    BuildExecutionResult {
        status: "Succeeded",
        exit_code: Some(0),
        image_digest: Some(format!("sha256:{}", marker.to_string().repeat(64))),
        resolved_commit_sha: Some(marker.to_string().repeat(40)),
        image_references: vec![format!("docker.io/citadel/test:{marker}")],
        error_code: None,
        error_message: None,
        logs: vec![],
    }
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE7_DATABASE_URL"]
async fn build_secret_resolver_decrypts_internal_values_without_persisting_plaintext() {
    let url = std::env::var("CITADEL_PHASE7_DATABASE_URL").unwrap();
    MigrationRunner::migrate(&url).await.unwrap();
    let pool = PgPoolOptions::new()
        .max_connections(2)
        .connect(&url)
        .await
        .unwrap();
    let secret_id = Uuid::now_v7();
    let protector = Arc::new(AesGcmSecretProtector::new(&[73_u8; 32]).unwrap());
    let envelope = SecretProtector::protect(protector.as_ref(), b"build-secret-value").unwrap();
    assert!(!envelope.contains("build-secret-value"));
    sqlx::query(
        "INSERT INTO secretdefinitions(id,name,providertype) VALUES($1,$2,'InternalEncrypted')",
    )
    .bind(secret_id)
    .bind(format!("build-secret-{}", secret_id.simple()))
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query("INSERT INTO internalsecretvalues(secretid,encryptedvalue) VALUES($1,$2)")
        .bind(secret_id)
        .bind(&envelope)
        .execute(&pool)
        .await
        .unwrap();

    let resolver = PostgresBuildSecretResolver::new(pool.clone(), protector).unwrap();
    let resolved = resolver.resolve(secret_id).await.unwrap();
    assert_eq!(resolved.as_str(), "build-secret-value");
    let stored: String =
        sqlx::query_scalar("SELECT encryptedvalue FROM internalsecretvalues WHERE secretid=$1")
            .bind(secret_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(stored, envelope);

    sqlx::query("DELETE FROM secretdefinitions WHERE id=$1")
        .bind(secret_id)
        .execute(&pool)
        .await
        .unwrap();
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE7_DATABASE_URL"]
async fn build_registry_credentials_are_loaded_only_for_authenticated_registries() {
    let url = std::env::var("CITADEL_PHASE7_DATABASE_URL").unwrap();
    MigrationRunner::migrate(&url).await.unwrap();
    let pool = PgPoolOptions::new()
        .max_connections(2)
        .connect(&url)
        .await
        .unwrap();
    let actor = ActorId::new(Uuid::now_v7());
    let authenticated = Uuid::now_v7();
    let anonymous = Uuid::now_v7();
    sqlx::query("INSERT INTO actors(id,isenabled,type) VALUES($1,true,'System')")
        .bind(actor.value())
        .execute(&pool)
        .await
        .unwrap();
    for (id, configuration) in [
        (
            authenticated,
            serde_json::json!({"$type":"Custom","AuthEnabled":true,"Username":"builder","Password":"registry-secret"}),
        ),
        (
            anonymous,
            serde_json::json!({"$type":"Custom","AuthEnabled":false,"Username":"ignored","Password":"ignored"}),
        ),
    ] {
        sqlx::query("INSERT INTO registries(id,configuration,createdbyactorid,name,registryhost,status) VALUES($1,$2,$3,$4,'registry.example.test','Enabled')")
            .bind(id)
            .bind(configuration)
            .bind(actor.value())
            .bind(format!("registry-{}", id.simple()))
            .execute(&pool)
            .await
            .unwrap();
    }
    let resolver = PostgresBuildRegistryCredentialResolver::new(pool.clone());
    let credentials = resolver.resolve(authenticated).await.unwrap().unwrap();
    assert_eq!(credentials.username, "builder");
    assert_eq!(credentials.password.as_str(), "registry-secret");
    assert!(resolver.resolve(anonymous).await.unwrap().is_none());

    sqlx::query("DELETE FROM registries WHERE id=ANY($1)")
        .bind([authenticated, anonymous])
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM actors WHERE id=$1")
        .bind(actor.value())
        .execute(&pool)
        .await
        .unwrap();
}
