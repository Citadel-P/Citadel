//! Ports BuildRunStartTests' completion propagation and failed post-build Apply
//! cases through the real Build finish transaction and PostgreSQL queue.
use super::*;
use citadel_adapters::persistence::postgres::{
    builds::completion::PostgresBuildCompletionRepository,
    deployments::PostgresDeploymentRepository, stacks::PostgresStackRepository,
};
use citadel_builds::{
    BuildCompletionRepository, BuildCompletionService, BuildConsumerClaim, BuildConsumerRuntime,
    BuildConsumerType, BuildEntitlements, BuildError,
};
use citadel_deployments::DeploymentRepository;
use citadel_stacks::StackRepository;
use std::sync::{
    Mutex,
    atomic::{AtomicBool, Ordering},
};

pub struct Consumers {
    manual: Uuid,
    automatic: Uuid,
    stack: Uuid,
}
pub async fn create(pool: &sqlx::PgPool, fixture: &FixtureIds, project: Uuid) -> Consumers {
    let platform = Uuid::now_v7();
    sqlx::query("INSERT INTO platforms(id,address,connectortype,cpucount,imagecount,memtotal,name,networkcount,platformdescriptor,status,volumecount) VALUES($1,$2,'Local',1,0,1048576,$2,0,'{\"$type\":\"Docker\"}'::json,'Online',0)")
        .bind(platform).bind(format!("build-consumers-{platform}")).execute(pool).await.unwrap();
    let store = PostgresDeploymentRepository::new(pool.clone());
    let mut ids = Vec::new();
    for automatic in [false, true] {
        let input = citadel_deployments::CreateDeployment {
            name: format!("consumer-{}", Uuid::now_v7()),
            platform_id: platform,
            description: None,
            spec: serde_json::from_value(json!({"image":{"$type":"Build","buildProjectId":project,"redeployOnBuild":automatic}})).unwrap(),
            tag_ids: Vec::new(),
            duplicate_source: None,
        };
        let resource = store
            .create(ActorId::new(SYSTEM_ACTOR_ID), true, &input)
            .await
            .unwrap();
        sqlx::query("UPDATE deployments SET spec=jsonb_set(spec::jsonb,'{Image,AppliedImageReference}','\"old-image\"'::jsonb)::json WHERE id=$1")
            .bind(resource.id).execute(pool).await.unwrap();
        ids.push(resource.id);
    }
    let input=serde_json::from_value::<citadel_server::api::resources::stacks::requests::CreateStackInput>(json!({"name":format!("consumer-{}",Uuid::now_v7()),"platformId":platform,"stackSource":"WebEditor",
        "spec":{"$type":"WebEditor","composeFile":"services:\n  web:\n    image: nginx\n  worker:\n    image: nginx\n","registryId":fixture.registry,"updateBehavior":"Disabled",
            "buildImageBindings":[{"serviceName":"web","buildProjectId":project,"redeployOnBuild":true,"appliedImageReference":"old-image"},{"serviceName":"worker","buildProjectId":project,"redeployOnBuild":false,"appliedImageReference":"old-image"}]}})).unwrap().try_into().unwrap();
    let stack = PostgresStackRepository::new(pool.clone())
        .create(ActorId::new(SYSTEM_ACTOR_ID), true, &input)
        .await
        .unwrap();
    Consumers {
        manual: ids[0],
        automatic: ids[1],
        stack: stack.id,
    }
}
#[derive(Default)]
struct Runtime {
    applied: Mutex<Vec<(BuildConsumerType, Uuid)>>,
    notified: Mutex<Vec<Uuid>>,
}
impl BuildConsumerRuntime for Runtime {
    fn apply<'a>(&'a self, claim: &'a BuildConsumerClaim) -> BoxFuture<'a, Result<(), BuildError>> {
        Box::pin(async move {
            self.applied
                .lock()
                .unwrap()
                .push((claim.resource_type, claim.resource_id));
            Err(BuildError::Conflict("injected Apply failure".into()))
        })
    }
    fn changed(&self, _: BuildConsumerType, id: Uuid) {
        self.notified.lock().unwrap().push(id);
    }
}
struct Entitlement(AtomicBool);
impl BuildEntitlements for Entitlement {
    fn enabled(
        &self,
        _: citadel_licensing::LicenseCapability,
    ) -> BoxFuture<'_, Result<bool, BuildError>> {
        Box::pin(async { Ok(self.0.load(Ordering::Relaxed)) })
    }
}
pub async fn verify(pool: &sqlx::PgPool, run: Uuid, ids: &Consumers) {
    let store = Arc::new(PostgresBuildCompletionRepository::new(pool.clone()));
    let runtime = Arc::new(Runtime::default());
    let entitlement = Arc::new(Entitlement(AtomicBool::new(false)));
    let service = BuildCompletionService::new(store.clone(), runtime.clone(), entitlement.clone());
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM buildcompletionqueue WHERE buildrunid=$1"
        )
        .bind(run)
        .fetch_one(pool)
        .await
        .unwrap(),
        3
    );
    sqlx::query("UPDATE deployments SET controlstate='Processing' WHERE id=$1")
        .bind(ids.automatic)
        .execute(pool)
        .await
        .unwrap();
    // Inventory's foreign-key checks must not block provenance propagation.
    // Hold real inserts open while the consumer updates both resource types.
    let platform: Uuid = sqlx::query_scalar("SELECT platformid FROM deployments WHERE id=$1")
        .bind(ids.manual)
        .fetch_one(pool)
        .await
        .unwrap();
    let mut inventory = pool.begin().await.unwrap();
    sqlx::query("SELECT id FROM platforms WHERE id=$1 FOR NO KEY UPDATE")
        .bind(platform)
        .execute(&mut *inventory)
        .await
        .unwrap();
    for (deployment, stack) in [(Some(ids.manual), None), (None, Some(ids.stack))] {
        let id = Uuid::now_v7();
        sqlx::query("INSERT INTO containers(id,created,dockercontainerid,dockerimageid,name,platformid,ports,deploymentid,stackid,state,updated) VALUES($1,0,$2,'image','consumer',$3,'[]',$4,$5,'Running',0)")
            .bind(id).bind(id.to_string()).bind(platform).bind(deployment).bind(stack)
            .execute(&mut *inventory).await.unwrap();
    }
    // Provenance still propagates without an automatic-execution license, but
    // busy resources are not modified and no Docker mutation is dispatched.
    assert_eq!(
        tokio::time::timeout(
            std::time::Duration::from_secs(5),
            service.process_batch(&CancellationToken::new())
        )
        .await
        .expect("inventory FK checks must not block Build consumers")
        .unwrap(),
        2
    );
    inventory.commit().await.unwrap();
    assert!(runtime.applied.lock().unwrap().is_empty());
    entitlement.0.store(true, Ordering::Relaxed);
    sqlx::query("UPDATE deployments SET controlstate='Idle' WHERE id=$1")
        .bind(ids.automatic)
        .execute(pool)
        .await
        .unwrap();
    assert_eq!(
        service
            .process_batch(&CancellationToken::new())
            .await
            .unwrap(),
        1
    );
    assert_eq!(
        runtime.applied.lock().unwrap().as_slice(),
        &[(BuildConsumerType::Deployment, ids.automatic)]
    );
    assert_eq!(runtime.notified.lock().unwrap().len(), 3);
    for id in [ids.manual, ids.automatic] {
        let spec: Value = sqlx::query_scalar("SELECT spec FROM deployments WHERE id=$1")
            .bind(id)
            .fetch_one(pool)
            .await
            .unwrap();
        assert_eq!(spec["Image"]["ResolvedBuildRunId"], run.to_string());
        assert_eq!(
            spec["Image"]["ResolvedImageReference"],
            "registry.test/project:latest"
        );
        assert_eq!(spec["Image"]["AppliedImageReference"], "old-image");
        assert!(spec["Image"]["AppliedBuildRunId"].is_null());
    }
    let spec: Value=sqlx::query_scalar("SELECT r.spec FROM stacks s JOIN stackreleases r ON r.id=s.currentstackreleaseid WHERE s.id=$1").bind(ids.stack).fetch_one(pool).await.unwrap();
    for binding in spec["BuildImageBindings"].as_array().unwrap() {
        assert_eq!(binding["ResolvedBuildRunId"], run.to_string());
        assert_eq!(binding["AppliedImageReference"], "old-image");
    }
    assert_eq!(
        sqlx::query_scalar::<_, String>("SELECT status FROM buildruns WHERE id=$1")
            .bind(run)
            .fetch_one(pool)
            .await
            .unwrap(),
        "Succeeded"
    );
    assert!(
        store.claim_next().await.unwrap().is_none(),
        "a failed consumer Apply is not silently replayed"
    );
    assert!(sqlx::query_scalar::<_,bool>("SELECT EXISTS(SELECT 1 FROM buildrunlogs WHERE buildrunid=$1 AND message LIKE '%Automatic Apply did not complete%')").bind(run).fetch_one(pool).await.unwrap());
}

pub async fn verify_lifecycle(
    pool: &sqlx::PgPool,
    builds: &BuildService,
    fixture: &FixtureIds,
    project: Uuid,
    original: Uuid,
    ids: &Consumers,
) {
    let actor = ActorId::new(SYSTEM_ACTOR_ID);
    let input = serde_json::from_value(json!({
        "name":format!("second-consumer-{}",Uuid::now_v7()),"enabled":true,"gitRepositoryId":fixture.git_repository,
        "branch":"main","contextPath":"src","dockerfilePath":"Dockerfile","buildArgs":[],"buildSecrets":[],
        "builderKind":"Platform","platformId":fixture.platform,"registryId":fixture.registry,
        "imageRepository":"citadel/second","tagTemplates":["latest"],"retentionRunCount":1
    })).unwrap();
    let other = builds.store().create(actor, &input).await.unwrap();
    sqlx::query("UPDATE stackreleases SET spec=jsonb_set(spec::jsonb,'{BuildImageBindings,1,BuildProjectId}',to_jsonb($2::text))::json WHERE id=(SELECT currentstackreleaseid FROM stacks WHERE id=$1)")
        .bind(ids.stack).bind(other.id).execute(pool).await.unwrap();
    sqlx::query("UPDATE buildprojects SET retentionruncount=1 WHERE id=$1")
        .bind(project)
        .execute(pool)
        .await
        .unwrap();
    let discarded = finish_next(builds, project).await;
    let newest = finish_next(builds, project).await;
    let other_run = finish_next(builds, other.id).await;
    assert!(
        !sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM buildruns WHERE id=$1)")
            .bind(discarded)
            .fetch_one(pool)
            .await
            .unwrap(),
        "superseded unreferenced Build can be retained away"
    );
    assert!(
        sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM buildruns WHERE id=$1)")
            .bind(original)
            .fetch_one(pool)
            .await
            .unwrap(),
        "the resolved image still pins its Build Run despite retention=1"
    );
    let queued: Vec<Uuid> = sqlx::query_scalar(
        "SELECT buildrunid FROM buildcompletionqueue WHERE resourceid=$1 ORDER BY buildrunid",
    )
    .bind(ids.stack)
    .fetch_all(pool)
    .await
    .unwrap();
    assert_eq!(
        queued.len(),
        2,
        "two Build Projects targeting the same Stack must not coalesce together"
    );
    assert!(queued.contains(&newest) && queued.contains(&other_run));
    let store = PostgresBuildCompletionRepository::new(pool.clone());
    let mut stack_claims = Vec::new();
    while let Some(claim) = store.claim_next().await.unwrap() {
        if claim.resource_type == BuildConsumerType::Stack {
            stack_claims.push(claim.clone());
            assert_eq!(
                claim.service_names,
                if claim.build_run_id == newest {
                    vec!["web".to_owned()]
                } else {
                    Vec::new()
                }
            );
            assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM buildcompletionqueue WHERE resourceid=$1 AND status='Processing'").bind(ids.stack).fetch_one(pool).await.unwrap(),1);
        }
        sqlx::query(
            "UPDATE buildcompletionqueue SET startedat=now()-interval '31 minutes' WHERE id=$1",
        )
        .bind(claim.id)
        .execute(pool)
        .await
        .unwrap();
        store.recover().await.unwrap();
        assert!(
            store
                .complete(&claim, "late completion")
                .await
                .unwrap()
                .is_none(),
            "a recovered receipt rejects a late completion"
        );
        assert!(sqlx::query_scalar::<_,bool>("SELECT EXISTS(SELECT 1 FROM buildrunlogs WHERE buildrunid=$1 AND message LIKE '%was interrupted%')").bind(claim.build_run_id).fetch_one(pool).await.unwrap());
    }
    assert_eq!(stack_claims.len(), 2);
    let spec:Value=sqlx::query_scalar("SELECT r.spec FROM stacks s JOIN stackreleases r ON r.id=s.currentstackreleaseid WHERE s.id=$1").bind(ids.stack).fetch_one(pool).await.unwrap();
    assert_eq!(
        spec["BuildImageBindings"][0]["ResolvedBuildRunId"],
        newest.to_string()
    );
    assert_eq!(
        spec["BuildImageBindings"][1]["ResolvedBuildRunId"],
        other_run.to_string()
    );
    assert_eq!(
        spec["BuildImageBindings"][0]["AppliedImageReference"],
        "old-image"
    );
    verify_queued_snapshot(pool, builds, other.id).await;
}

// Ports ExecuteQueuedBuildRun_ShouldUseQueuedRunTargetSnapshot_WhenProjectBuilderChangesAfterQueue.
// The run must also retain its BuildKit bindings, arguments, and output tags.
async fn verify_queued_snapshot(pool: &sqlx::PgPool, builds: &BuildService, project: Uuid) {
    let original = builds.store().get(project).await.unwrap();
    let mut input: citadel_builds::BuildAgentPoolConfiguration = serde_json::from_value(json!({"name":format!("snapshot-{}", Uuid::now_v7()),"enabled":true,"providerSpec":{"$type":"SelfManagedVm","connectionMode":"EdgeAgent"}})).unwrap();
    input.validate().unwrap();
    let changed_pool = builds
        .store()
        .create_pool(ActorId::new(SYSTEM_ACTOR_ID), &input)
        .await
        .unwrap();
    let run = builds
        .store()
        .enqueue(ActorId::new(SYSTEM_ACTOR_ID), project, "Manual")
        .await
        .unwrap();
    sqlx::query("UPDATE buildprojects SET platformid=NULL,builderkind='BuildAgentPool',buildagentpoolid=$2,buildargs='[{\"name\":\"NEW\",\"value\":\"changed\"}]',tagtemplates='[\"changed\"]',buildsecrets='[{\"id\":\"changed\",\"secretId\":\"00000000-0000-0000-0000-000000000001\"}]' WHERE id=$1")
        .bind(project).bind(changed_pool.id).execute(pool).await.unwrap();
    let claim = builds
        .store()
        .claim_next(chrono::Utc::now() - chrono::Duration::hours(1))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(claim.run.id, run.id);
    assert_eq!(claim.project.builder_kind, original.builder_kind);
    assert_eq!(claim.project.platform_id, original.platform_id);
    assert_eq!(
        claim.project.build_agent_pool_id,
        original.build_agent_pool_id
    );
    assert_eq!(claim.project.build_args, original.build_args);
    assert_eq!(claim.project.build_secrets, original.build_secrets);
    assert_eq!(claim.project.tag_templates, original.tag_templates);
    builds
        .store()
        .finish(
            &claim,
            &citadel_builds::BuildExecutionResult {
                status: citadel_builds::BuildRunStatus::Failed,
                exit_code: None,
                image_digest: None,
                resolved_commit_sha: None,
                image_references: vec![],
                error_code: Some("TestComplete".into()),
                error_message: None,
                logs: vec![],
            },
        )
        .await
        .unwrap();
}

async fn finish_next(builds: &BuildService, project: Uuid) -> Uuid {
    let run = builds
        .store()
        .enqueue(ActorId::new(SYSTEM_ACTOR_ID), project, "Manual")
        .await
        .unwrap();
    assert!(builds.process_one(&CancellationToken::new()).await.unwrap());
    assert_eq!(
        builds.store().get_run(run.id).await.unwrap().status,
        citadel_builds::BuildRunStatus::Succeeded
    );
    run.id
}
