use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::get,
};
use chrono::Utc;
use citadel_adapters::connectors::edge::EdgeRegistry;
use citadel_adapters::connectors::routing::containers::ContainerRuntimeRouter;
use citadel_adapters::persistence::postgres::backups::PostgresBackupPersistence;
use citadel_adapters::persistence::postgres::backups::source_planner::PostgresBackupSourcePlanner;
use citadel_adapters::persistence::postgres::stacks::PostgresStackRepository;
use citadel_backups::policies::read_models::BackupPreviewKind;
use citadel_backups::*;
use citadel_database::MigrationRunner;
use citadel_docker::client::DockerClient;
use citadel_docker::client::DockerEndpoint;
use citadel_identity::SYSTEM_ACTOR_ID;
use citadel_primitives::ActorId;
use citadel_stacks::{
    CreateStack, StackRepository, StackSource, StackSpec, StackSpecCommon, StackUpdateBehavior,
};
use serde_json::{Value, json};
use sqlx::postgres::PgPoolOptions;
use std::{
    collections::{BTreeSet, HashMap},
    sync::{
        Arc, RwLock,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

type Documents = Arc<RwLock<HashMap<String, Value>>>;
async fn inspect(
    State((documents, requests)): State<(Documents, Arc<AtomicUsize>)>,
    Path(id): Path<String>,
) -> Result<Json<Value>, StatusCode> {
    requests.fetch_add(1, Ordering::SeqCst);
    documents
        .read()
        .unwrap()
        .get(&id)
        .cloned()
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}
fn identities(items: &[BackupSourceItem]) -> BTreeSet<String> {
    items
        .iter()
        .map(|item| {
            item.host_path
                .clone()
                .unwrap_or_else(|| item.volume_name.clone())
        })
        .collect()
}

#[tokio::test]
#[ignore = "requires disposable CITADEL_EXECUTION_DATABASE_URL; Docker is simulated"]
async fn stack_preview_and_execution_discover_owned_physical_mounts_without_saved_bindings() {
    let url = std::env::var("CITADEL_EXECUTION_DATABASE_URL").unwrap();
    MigrationRunner::migrate(&url).await.unwrap();
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .connect(&url)
        .await
        .unwrap();
    let platform = Uuid::now_v7();
    let actor = ActorId::new(SYSTEM_ACTOR_ID);
    sqlx::query("INSERT INTO platforms(id,address,connectortype,cpucount,imagecount,memtotal,name,networkcount,platformdescriptor,status,volumecount) VALUES($1,$3,'Local',1,0,1048576,$2,0,'{\"$type\":\"DockerStandalone\"}','Online',0)")
        .bind(platform).bind(format!("discovery-{platform}")).bind(format!("unix:///discovery-{platform}.sock")).execute(&pool).await.unwrap();
    let stack = PostgresStackRepository::new(pool.clone()).create(actor, true, &CreateStack {
        name: format!("discovery-{platform}"), platform_id: platform, description: None, stack_source: StackSource::WebEditor,
        spec: StackSpec::WebEditor { compose_file: "services:\n  database:\n    image: postgres\n  wordpress:\n    image: wordpress\n    volumes:\n      - wordpress:/var/www/html\nvolumes:\n  wordpress: {}\n".into(), update_behavior: StackUpdateBehavior::Disabled, common: StackSpecCommon::default() },
        drift_policy: None, tag_ids: vec![], duplicate_source: None,
    }).await.unwrap();
    for (id, owner) in [
        ("wordpress", Some(stack.id)),
        ("database", Some(stack.id)),
        ("unrelated", None),
    ] {
        sqlx::query("INSERT INTO containers(id,created,updated,stackid,dockercontainerid,dockerimageid,name,platformid,ports,state) VALUES($1,0,0,$2,$3,'image',$3,$4,'{}','Running')")
            .bind(Uuid::now_v7()).bind(owner).bind(id).bind(platform).execute(&pool).await.unwrap();
    }
    let docs: Documents = Arc::new(RwLock::new(HashMap::from([
        (
            "wordpress".into(),
            json!({"Id":"wordpress","Config":{"Labels":{"com.docker.compose.service":"wordpress"}},"Mounts":[
                {"Type":"volume","Name":"example_wordpress","Destination":"/var/www/html"},
                {"Type":"volume","Name":"example_wordpress","Destination":"/var/www/html/wordpress"},
                {"Type":"volume","Name":"anonymous-generated-name","Destination":"/cache"},
                {"Type":"bind","Source":"/srv/wordpress","Destination":"/uploads"},
                {"Type":"bind","Source":"/var/run/docker.sock","Destination":"/var/run/docker.sock"},
                {"Type":"tmpfs","Destination":"/tmp"}
            ]}),
        ),
        (
            "database".into(),
            json!({"Id":"database","Config":{"Labels":{"com.docker.compose.service":"database"}},"Mounts":[
                {"Type":"volume","Name":"external_database","Destination":"/var/lib/mysql"},
                {"Type":"volume","Name":"example_wordpress","Destination":"/shared"},
                {"Type":"bind","Source":"/srv/wordpress","Destination":"/shared-uploads"}
            ]}),
        ),
        (
            "unrelated".into(),
            json!({"Id":"unrelated","Mounts":[{"Type":"volume","Name":"do-not-back-up"}]}),
        ),
    ])));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let shutdown = CancellationToken::new();
    let server_shutdown = shutdown.clone();
    let requests = Arc::new(AtomicUsize::new(0));
    let app = Router::new()
        .route(
            "/version",
            get(|| async { Json(json!({"ApiVersion":"1.49","MinAPIVersion":"1.41"})) }),
        )
        .route("/v1.49/containers/{id}/json", get(inspect))
        .route("/v1.49/info", get(|| async { Json(json!({"ID":"discovery-daemon", "Swarm":{"NodeID":"node-a","LocalNodeState":"active","ControlAvailable":true}})) }))
        .with_state((docs.clone(), requests.clone()));
    let server = tokio::spawn(async move {
        axum::serve(listener, app)
            .with_graceful_shutdown(server_shutdown.cancelled_owned())
            .await
            .unwrap();
    });
    let docker = DockerClient::with_endpoint(
        DockerEndpoint::Tcp(format!("http://{address}").parse().unwrap()),
        Duration::from_secs(2),
        "/host",
    )
    .unwrap();
    let planner = PostgresBackupSourcePlanner::new(pool.clone()).with_runtime(
        ContainerRuntimeRouter::new(pool.clone(), docker, None, EdgeRegistry::default()),
    );
    let cancel = CancellationToken::new();
    let secret = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO secretdefinitions(id,name,providertype) VALUES($1,$2,'InternalEncrypted')",
    )
    .bind(secret)
    .bind(format!("discovery-{secret}"))
    .execute(&pool)
    .await
    .unwrap();
    let store = PostgresBackupPersistence::new(pool.clone());
    let repository = store.create_repository(actor, &BackupRepositoryConfiguration {
        name: format!("discovery-{platform}"), description: None, password_secret_id: secret,
        spec: serde_json::from_value(json!({"$type":"FileSystem","location":"Core","path":"/tmp/disposable-backups"})).unwrap(),
    }).await.unwrap();
    let operation = Uuid::now_v7();
    assert!(
        store
            .acquire_repository_operation(
                repository.id,
                operation,
                "Validate",
                Utc::now() + chrono::Duration::minutes(5)
            )
            .await
            .unwrap()
    );
    store
        .record_repository_operation(
            repository.id,
            BackupRepositoryOperation {
                operation_id: operation,
                operation: "Validate",
                location: "Core",
                platform_id: None,
                succeeded: true,
                message: None,
            },
        )
        .await
        .unwrap();
    store
        .release_repository_operation(repository.id, operation)
        .await
        .unwrap();
    let source = spec::BackupSourceSpec::Stack { stack_id: stack.id };
    let policy = store
        .create_policy(
            actor,
            &BackupPolicyConfiguration {
                name: format!("discovery-{platform}"),
                description: None,
                source,
                backup_repository_id: repository.id,
                enabled: true,
                cron: None,
                time_zone: None,
                webhook: None,
                keep_last_successful: Some(2),
                timeout_seconds: Some(60),
                alert_on_failure: false,
                run_as_actor_id: None,
                tag_ids: vec![],
            },
        )
        .await
        .unwrap();
    let run = store
        .enqueue_backup(actor, policy.id, "Manual")
        .await
        .unwrap();
    let mut claim = BackupClaim {
        policy,
        repository,
        run,
    };
    let expected = BTreeSet::from([
        "example_wordpress".into(),
        "external_database".into(),
        "anonymous-generated-name".into(),
        "/srv/wordpress".into(),
    ]);
    requests.store(0, Ordering::SeqCst);
    let plan = planner.plan(&claim, &cancel).await.unwrap();
    assert_eq!(requests.load(Ordering::SeqCst), 2);
    assert_eq!(identities(&plan.items), expected);
    assert_eq!(
        plan.items.len(),
        4,
        "Repeated mounts and shared storage must produce one snapshot per source"
    );
    assert!(plan.warnings.iter().any(|w| w.contains("docker.sock")));
    requests.store(0, Ordering::SeqCst);
    let preview = planner
        .preview(BackupPreviewKind::Stack, stack.id, actor, true, &cancel)
        .await
        .unwrap();
    assert_eq!(requests.load(Ordering::SeqCst), 2);
    assert_eq!(
        preview
            .volumes
            .iter()
            .map(|v| v.name.clone())
            .collect::<BTreeSet<_>>(),
        expected
    );
    assert!(
        !preview
            .warnings
            .iter()
            .any(|w| w.contains("Compose declarations"))
    );

    // Reconciliation can remove a missing container entirely. The remaining
    // service must not turn this into a successful, partial stack backup.
    sqlx::query(
        "UPDATE containers SET stackid=NULL WHERE platformid=$1 AND dockercontainerid='database'",
    )
    .bind(platform)
    .execute(&pool)
    .await
    .unwrap();
    assert!(
        matches!(planner.plan(&claim, &cancel).await, Err(BackupError::Validation(m)) if m.contains("services are missing") && m.contains("database"))
    );
    let partial = planner
        .preview(BackupPreviewKind::Stack, stack.id, actor, true, &cancel)
        .await
        .unwrap();
    assert!(
        partial
            .warnings
            .iter()
            .any(|w| w.contains("services are missing"))
    );
    sqlx::query(
        "UPDATE containers SET stackid=$1 WHERE platformid=$2 AND dockercontainerid='database'",
    )
    .bind(stack.id)
    .bind(platform)
    .execute(&pool)
    .await
    .unwrap();

    // Even obsolete release bindings must not silently add detached volumes.
    sqlx::query("INSERT INTO stackreleasevolumebindings(id,platformid,stackreleaseid,volumename) SELECT $1,$2,currentstackreleaseid,'obsolete-data' FROM stacks WHERE id=$3")
        .bind(Uuid::now_v7()).bind(platform).bind(stack.id).execute(&pool).await.unwrap();
    assert_eq!(
        identities(&planner.plan(&claim, &cancel).await.unwrap().items),
        expected
    );
    // Coverage must use the same physical inventory, not obsolete release bindings.
    for id in ["wordpress", "database"] {
        let mounts = docs.read().unwrap()[id]["Mounts"].clone();
        sqlx::query("UPDATE containers SET mounts=$1 WHERE platformid=$2 AND dockercontainerid=$3")
            .bind(mounts)
            .bind(platform)
            .bind(id)
            .execute(&pool)
            .await
            .unwrap();
    }
    use citadel_backups::coverage::CoverageReader;
    let coverage =
        citadel_adapters::persistence::postgres::backups::coverage::PostgresVolumeCoverageReader(
            pool.clone(),
        )
        .snapshot(actor, true, &[platform])
        .await
        .unwrap();
    let workload = coverage
        .workloads
        .iter()
        .find(|w| w.id == stack.id)
        .unwrap();
    assert_eq!(
        workload
            .volumes
            .iter()
            .filter(|v| v.included)
            .map(|v| v.target.name.clone())
            .collect::<BTreeSet<_>>(),
        expected
    );
    sqlx::query("UPDATE containers SET projectionstalesince=1 WHERE platformid=$1 AND dockercontainerid='database'").bind(platform).execute(&pool).await.unwrap();
    assert!(
        matches!(planner.plan(&claim, &cancel).await, Err(BackupError::Validation(m)) if m.contains("stale or incomplete"))
    );
    let partial = planner
        .preview(BackupPreviewKind::Stack, stack.id, actor, true, &cancel)
        .await
        .unwrap();
    assert!(
        !partial
            .volumes
            .iter()
            .any(|v| v.name == "external_database")
    );
    assert!(
        partial
            .warnings
            .iter()
            .any(|w| w.contains("stale or incomplete"))
    );
    sqlx::query("UPDATE containers SET projectionstalesince=NULL WHERE platformid=$1")
        .bind(platform)
        .execute(&pool)
        .await
        .unwrap();
    let original = docs.write().unwrap().remove("database").unwrap();
    assert!(
        matches!(planner.plan(&claim, &cancel).await, Err(BackupError::Validation(m)) if m.contains("could not be inspected"))
    );
    docs.write()
        .unwrap()
        .insert("database".into(), json!({"Id":"database"}));
    assert!(
        matches!(planner.plan(&claim, &cancel).await, Err(BackupError::Validation(m)) if m.contains("mount information"))
    );
    docs.write().unwrap().insert("database".into(), original);

    // Deployment source semantics remain declared volumes + discovered bind mounts.
    let deployment = Uuid::now_v7();
    let spec = serde_json::from_value::<citadel_deployments::DeploymentSpec>(
        json!({"image":{"$type":"Local","imageId":"image"},"volumes":["example_wordpress:/data"]}),
    )
    .unwrap()
    .to_storage_value()
    .unwrap();
    sqlx::query("INSERT INTO deployments(id,name,platformid,createdbyactorid,spec,status) VALUES($1,$2,$3,$4,$5,'Running')").bind(deployment).bind(format!("discovery-{deployment}")).bind(platform).bind(actor.value()).bind(spec).execute(&pool).await.unwrap();
    sqlx::query("UPDATE containers SET stackid=NULL,deploymentid=$1 WHERE platformid=$2 AND dockercontainerid='wordpress'").bind(deployment).bind(platform).execute(&pool).await.unwrap();
    claim.run.source_snapshot = spec::BackupSourceSpec::Deployment {
        deployment_id: deployment,
    };
    let plan = planner.plan(&claim, &cancel).await.unwrap();
    assert_eq!(
        identities(&plan.items),
        BTreeSet::from(["example_wordpress".into(), "/srv/wordpress".into()])
    );
    requests.store(0, Ordering::SeqCst);
    let preview = planner
        .preview(
            BackupPreviewKind::Deployment,
            deployment,
            actor,
            true,
            &cancel,
        )
        .await
        .unwrap();
    assert_eq!(
        requests.load(Ordering::SeqCst),
        1,
        "volumes and binds share one inspection"
    );
    assert!(
        preview
            .volumes
            .iter()
            .any(|v| v.name == "anonymous-generated-name" && v.kind == "AnonymousNamed")
    );
    sqlx::query("UPDATE containers SET stackid=NULL,deploymentid=$1 WHERE platformid=$2 AND dockercontainerid='database'").bind(deployment).bind(platform).execute(&pool).await.unwrap();
    assert!(
        matches!(planner.plan(&claim, &cancel).await, Err(BackupError::Conflict(m)) if m.contains("ambiguous"))
    );
    // No owned containers: declarations and old bindings cannot be execution sources.
    claim.run.source_snapshot = spec::BackupSourceSpec::Stack { stack_id: stack.id };
    assert!(
        matches!(planner.plan(&claim, &cancel).await, Err(BackupError::Validation(m)) if m.contains("No synchronized containers"))
    );
    assert!(
        planner
            .preview(BackupPreviewKind::Stack, stack.id, actor, true, &cancel)
            .await
            .unwrap()
            .volumes
            .is_empty()
    );
    // Swarm planning retains stable task/node checks but resolves both storage
    // types from a single inspection of each task.
    sqlx::query("UPDATE platforms SET platformdescriptor=$2 WHERE id=$1")
        .bind(platform)
        .bind(json!({"$type":"DockerSwarm","nodeID":"node-a"}))
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO swarmnodeprojections(platformid,dockernodeid,address,architecture,availability,desiredtaskcount,engineversion,hostname,isleader,labels,observedat,operatingsystem,reachability,role,runningtaskcount,status,versionindex) VALUES($1,'node-a','10.0.0.2','amd64','active',2,'29','node-a',true,'{}',now(),'linux','reachable','manager',2,'ready',1)")
        .bind(platform).execute(&pool).await.unwrap();
    for service in ["wordpress", "database"] {
        sqlx::query("INSERT INTO swarmserviceprojections(platformid,dockerserviceid,name,image,mode,desiredtaskcount,runningtaskcount,ports,networkids,secretids,configids,labels,updatestate,versionindex,observedat,stackid,dockerstacknamespace,ownership) VALUES($1,$2,$3,'image','Replicated',1,1,'[]','[]','[]','[]','{}','',1,now(),$4,'actual','CitadelStack')")
            .bind(platform).bind(service).bind(format!("actual_{service}")).bind(stack.id).execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO swarmtaskprojections(platformid,dockertaskid,desiredstate,dockernodeid,dockerserviceid,dockercontainerid,image,name,nodehostname,observedat,ports,servicename,state,statusmessage,versionindex) VALUES($1,$2,'running','node-a',$2,$2,'image',$2,'node-a',now(),'[]',$2,'running','',1)")
            .bind(platform).bind(service).execute(&pool).await.unwrap();
    }
    claim.repository.repository_type = "S3Compatible".into();
    requests.store(0, Ordering::SeqCst);
    let plan = planner.plan(&claim, &cancel).await.unwrap();
    assert_eq!(identities(&plan.items), expected);
    assert_eq!(
        requests.load(Ordering::SeqCst),
        2,
        "one inspection per task for volumes and binds"
    );
    assert!(
        plan.items
            .iter()
            .all(|i| i.docker_node_id.as_deref() == Some("node-a"))
    );
    sqlx::query("UPDATE swarmtaskprojections SET isstale=true WHERE platformid=$1 AND dockertaskid='database'")
        .bind(platform).execute(&pool).await.unwrap();
    assert!(
        matches!(planner.plan(&claim, &cancel).await, Err(BackupError::Validation(m)) if m.contains("stale"))
    );

    shutdown.cancel();
    server.await.unwrap();
    // The runner shares its disposable database with other execution tests.
    sqlx::query("DELETE FROM backupruns WHERE backuppolicyid=$1")
        .bind(claim.policy.id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM backuppolicies WHERE id=$1")
        .bind(claim.policy.id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM backuprepositories WHERE id=$1")
        .bind(claim.repository.id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM stacks WHERE id=$1")
        .bind(stack.id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM deployments WHERE id=$1")
        .bind(deployment)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM platforms WHERE id=$1")
        .bind(platform)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM secretdefinitions WHERE id=$1")
        .bind(secret)
        .execute(&pool)
        .await
        .unwrap();
    pool.close().await;
}
