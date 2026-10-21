use super::*;
use citadel_adapters::{
    connectors::routing::{
        platforms::runtime::PlatformRuntimeRouter, resource_copies::CopyStorageRuntime,
    },
    persistence::postgres::{
        backups::source_planner::PostgresBackupSourcePlanner,
        resource_copies::PostgresCopyRepository,
    },
    security::identity::backup_authorization::IdentityBackupRunAuthorizer,
};
use citadel_backups::BackupService;
use citadel_primitives::ResourceType;
use citadel_resource_copies::CopyService;
use citadel_server::realtime_groups::{Group, GroupReadPort};

#[tokio::test]
#[ignore = "requires disposable CITADEL_PLATFORM_DATABASE_URL"]
async fn workload_lists_preserve_actor_specific_copy_eligibility() {
    let f = fixture().await;
    let mut reader = super::realtime_groups::reader(&f);
    let backups = Arc::new(BackupService::new(
        reader.backups.clone(),
        Arc::new(super::volume_coverage::NoExecution),
        Arc::new(PostgresBackupSourcePlanner::new(f.pool.clone())),
        Duration::minutes(5),
        Arc::new(IdentityBackupRunAuthorizer::new(reader.identity.clone())),
    ));
    reader.copy_eligibility = Some(Arc::new(CopyService::new(
        Arc::new(PostgresCopyRepository::new(f.pool.clone())),
        Arc::new(CopyStorageRuntime {
            image: "unused".into(),
            runtime: Arc::new(PlatformRuntimeRouter::new(
                f.pool.clone(),
                DockerClient::new("/no-copy-runtime.sock", StdDuration::from_secs(1)).unwrap(),
                None,
                Default::default(),
            )),
        }),
        backups,
        Arc::new(|_, _| panic!("eligibility must not mutate resources")),
    )));
    let standalone = Uuid::now_v7();
    sqlx::query("INSERT INTO platforms(id,address,connectortype,cpucount,imagecount,memtotal,name,networkcount,platformdescriptor,status,volumecount) VALUES($1,'local','Local',1,0,1,$1::text,0,'{\"$type\":\"Docker\"}','Online',0)")
        .bind(standalone).execute(&f.pool).await.unwrap();
    let deployment = Uuid::now_v7();
    let stack = Uuid::now_v7();
    let release = Uuid::now_v7();
    let service = Uuid::now_v7();
    let deployment_spec: citadel_deployments::DeploymentSpec = serde_json::from_value(json!({
        "image":{"$type":"Local","imageId":"fixture"}, "volumes":["data:/data"]
    }))
    .unwrap();
    let stack_spec: citadel_stacks::StackSpec = serde_json::from_value(json!({
        "$type":"WebEditor", "composeFile":"services:\n  web:\n    image: alpine\n    volumes: [data:/data]\nvolumes:\n  data: {}\n"
    })).unwrap();
    let service_spec: citadel_swarm_services::SwarmServiceSpec = serde_json::from_value(json!({
        "image":{"$type":"External","registryId":Uuid::now_v7(),"imageTag":"alpine"},
        "schedulingMode":"Replicated", "replicas":1,
        "mounts":[{"kind":"Volume","source":"data","target":"/data","readOnly":false}],
        "placementConstraints":["node.id == fixture"]
    }))
    .unwrap();
    sqlx::query("INSERT INTO deployments(id,name,platformid,createdbyactorid,spec,status) VALUES($1,$1::text,$2,$3,$4,'Created')")
        .bind(deployment).bind(standalone).bind(f.actor_id).bind(deployment_spec.to_storage_value().unwrap()).execute(&f.pool).await.unwrap();
    sqlx::query("INSERT INTO stacks(id,name,createdbyactorid,driftpolicy,stacksource,stackupdatestate) VALUES($1,$1::text,$2,$3,'WebEditor',$4)")
        .bind(stack).bind(f.actor_id)
        .bind(citadel_stacks::StackDriftPolicy::default().to_storage_value().unwrap())
        .bind(citadel_stacks::StackUpdateState::new(&stack_spec).to_storage_value().unwrap())
        .execute(&f.pool).await.unwrap();
    sqlx::query("INSERT INTO stackreleases(id,stackid,platformid,createdbyactorid,spec,status,version) VALUES($1,$2,$3,$4,$5,'Created','1')")
        .bind(release).bind(stack).bind(standalone).bind(f.actor_id).bind(stack_spec.to_storage_value().unwrap()).execute(&f.pool).await.unwrap();
    sqlx::query("UPDATE stacks SET currentstackreleaseid=$2 WHERE id=$1")
        .bind(stack)
        .bind(release)
        .execute(&f.pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO swarmservices(id,name,dockername,platformid,createdbyactorid,spec,desiredspechash,health,synchronizationstate,updatedat,autoupdatestate_status,autoupdatestate_lastcheckedat) VALUES($1,$1::text,$1::text,$2,$3,$4,'fixture','Created','NeverApplied',CURRENT_TIMESTAMP,'Unknown','-infinity')")
        .bind(service).bind(f.platform_id).bind(f.actor_id).bind(service_spec.to_storage_value().unwrap()).execute(&f.pool).await.unwrap();
    let shutdown = tokio_util::sync::CancellationToken::new();
    let _shutdown_guard = shutdown.clone().drop_guard();
    let app = workload_router(&f, &reader, shutdown);
    let regular = super::lookup::subject(&f).await;
    let denied = super::lookup::subject(&f).await;
    let role = Uuid::now_v7();
    sqlx::query("INSERT INTO roles(id,name,roletype) VALUES($1,'copy-list-reader','Custom')")
        .bind(role)
        .execute(&f.pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO actorroles(actorid,roleid) VALUES($1,$2)")
        .bind(regular.actor_id.value())
        .bind(role)
        .execute(&f.pool)
        .await
        .unwrap();
    for kind in [
        ResourceType::Deployment,
        ResourceType::Stack,
        ResourceType::SwarmService,
    ] {
        sqlx::query("INSERT INTO permissions(id,roleid,resourcetype,permissionlevel,specificpermissions) VALUES($1,$2,$3,1,0)")
            .bind(Uuid::now_v7()).bind(role).bind(kind as i32).execute(&f.pool).await.unwrap();
    }
    super::lookup::grant(
        &f,
        regular.actor_id.value(),
        ResourceType::Platform,
        f.platform_id,
        0,
    )
    .await;

    for (id, kind, list, detail) in [
        (
            deployment,
            ResourceType::Deployment,
            "deployments".into(),
            format!("deployment:{deployment}"),
        ),
        (
            stack,
            ResourceType::Stack,
            "stacks".into(),
            format!("stack:{stack}"),
        ),
        (
            service,
            ResourceType::SwarmService,
            format!("swarm-services:{}", f.platform_id),
            format!("swarm-service:{service}"),
        ),
    ] {
        super::lookup::grant(&f, regular.actor_id.value(), kind, id, 0).await;
        let (endpoint, key) = match kind {
            ResourceType::Deployment => ("deployments", "deployments"),
            ResourceType::Stack => ("stacks", "stacks"),
            ResourceType::SwarmService => ("swarmServices", "swarmServices"),
            _ => unreachable!(),
        };
        let list_path = format!("/api/v1/{endpoint}");
        let detail_path = format!("{list_path}/{id}");
        let list = Group::parse(&list).unwrap();
        let detail = Group::parse(&detail).unwrap();
        for (principal, expected) in [(&f.administrator, "Available"), (&regular, "Unavailable")] {
            let list_value = reader.read(principal, &list, None).await.unwrap();
            let row = list_value
                .rows
                .iter()
                .flat_map(|b| &b.rows)
                .find(|r| r["id"] == id.to_string())
                .unwrap();
            assert_eq!(
                row["copyEligibility"]["status"], expected,
                "{kind:?}: {row}"
            );
            let http_list = read_http(&app, principal, &list_path).await;
            let http_row = http_list[key]
                .as_array()
                .unwrap()
                .iter()
                .find(|r| r["id"] == id.to_string())
                .unwrap();
            let http_detail = read_http(&app, principal, &detail_path).await;
            assert_eq!(http_row["copyEligibility"], row["copyEligibility"]);
            assert_eq!(http_row["copyEligibility"], http_detail["copyEligibility"]);
            let detail_value = reader.read(principal, &detail, None).await.unwrap();
            assert_eq!(
                row["copyEligibility"],
                detail_value.rows[0].rows[0]["copyEligibility"]
            );
        }
        assert!(
            read_http(&app, &denied, &list_path).await[key]
                .as_array()
                .unwrap()
                .is_empty()
        );
        assert!(matches!(
            reader.read(&denied, &list, None).await,
            Err(RealtimeReadError::Authorization)
        ));
    }
    // The next list snapshot reflects changed configuration instead of replacing
    // eligibility with null (which previously forced the menu to fetch details).
    sqlx::query("UPDATE deployments SET spec=jsonb_set(spec,'{Volumes}','[]') WHERE id=$1")
        .bind(deployment)
        .execute(&f.pool)
        .await
        .unwrap();
    let changed = reader
        .read(
            &f.administrator,
            &Group::parse("deployments").unwrap(),
            None,
        )
        .await
        .unwrap();
    let row = changed
        .rows
        .iter()
        .flat_map(|b| &b.rows)
        .find(|r| r["id"] == deployment.to_string())
        .unwrap();
    assert_eq!(row["copyEligibility"]["status"], "Unavailable");
    assert!(
        row["copyEligibility"]["reason"]
            .as_str()
            .unwrap()
            .contains("No named volumes")
    );
    let refreshed = read_http(&app, &f.administrator, "/api/v1/deployments").await;
    let http_row = refreshed["deployments"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["id"] == deployment.to_string())
        .unwrap();
    assert_eq!(http_row["copyEligibility"], row["copyEligibility"]);
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM resourcecopies")
        .fetch_one(&f.pool)
        .await
        .unwrap();
    assert_eq!(count, 0, "eligibility must not create copy operations");
    f.docker_server.abort();
    f.pool.close().await;
    std::fs::remove_file(&f.docker_socket).unwrap();
}

async fn read_http(app: &Router, principal: &ActorPrincipal, path: &str) -> Value {
    let mut request = Request::builder().uri(path).body(Body::empty()).unwrap();
    request.extensions_mut().insert(principal.clone());
    json_body(app.clone().oneshot(request).await.unwrap()).await
}

fn workload_router(
    f: &Fixture,
    reader: &citadel_server::realtime_groups::ApplicationGroupReader,
    shutdown: tokio_util::sync::CancellationToken,
) -> Router {
    use citadel_adapters::{
        connectors::routing::{
            deployments::DeploymentRuntimeRouter, stacks::StackRuntimeRouter,
            swarm_services::SwarmServiceRuntimeRouter,
        },
        persistence::postgres::licensing::store::PostgresLicenseEntitlementService,
    };
    use citadel_server::{
        api::routes::{deployments, stacks, swarm_services},
        tasks::{
            deployments::TrackedDeploymentTasks, stacks::TrackedStackTasks,
            swarm_services::TrackedSwarmServiceTasks,
        },
    };
    let tasks = citadel_runtime::DynamicTasks::new(shutdown.clone());
    deployments::router(deployments::DeploymentsHttpState {
        copy_eligibility: reader.copy_eligibility.clone(),
        backup_coverage: None,
        identity: reader.identity.clone(),
        deployments: Arc::new(citadel_deployments::DeploymentService::new(
            Arc::new(TrackedDeploymentTasks::new(tasks.clone())),
            reader.deployments.clone(),
            Arc::new(DeploymentRuntimeRouter::new(
                f.pool.clone(),
                f.docker.clone(),
                None,
            )),
            Arc::new(PostgresLicenseEntitlementService::new(f.pool.clone())),
            Arc::new(citadel_deployments::EmptyDeploymentBindingResolver),
            shutdown.clone(),
        )),
    })
    .merge(stacks::router(stacks::StacksHttpState {
        copy_eligibility: reader.copy_eligibility.clone(),
        backup_coverage: None,
        identity: reader.identity.clone(),
        platforms: reader.platforms.clone(),
        stacks: Arc::new(citadel_stacks::StackService::new(
            Arc::new(TrackedStackTasks::new(tasks.clone())),
            reader.stacks.clone(),
            Arc::new(StackRuntimeRouter::new(
                f.pool.clone(),
                f.docker.clone(),
                None,
            )),
            Arc::new(NoBindings),
            Arc::new(citadel_stacks::NoopStackChangeNotifier),
            shutdown.clone(),
        )),
    }))
    .merge(swarm_services::router(
        swarm_services::SwarmServicesHttpState {
            copy_eligibility: reader.copy_eligibility.clone(),
            backup_coverage: None,
            identity: reader.identity.clone(),
            services: Arc::new(citadel_swarm_services::SwarmServiceService::new(
                Arc::new(TrackedSwarmServiceTasks::new(tasks)),
                reader.services.clone(),
                Arc::new(SwarmServiceRuntimeRouter::new(
                    f.pool.clone(),
                    f.docker.clone(),
                    None,
                )),
                Arc::new(citadel_swarm_services::EmptySwarmServiceBindingResolver),
                shutdown,
            )),
        },
    ))
}

struct NoBindings;
impl citadel_stacks::StackBindingResolverPort for NoBindings {
    fn resolve<'a>(
        &'a self,
        _: Uuid,
        _: &'a [String],
    ) -> futures_util::future::BoxFuture<
        'a,
        Result<citadel_stacks::ResolvedStackBindings, citadel_stacks::StackError>,
    > {
        panic!("list and detail reads must not resolve bindings")
    }
}
