use super::*;
use chrono::Utc;
use citadel_resource_copies::*;
use sqlx::Row;

async fn fixture() -> (sqlx::PgPool, ActorPrincipal, FixtureIds) {
    let url = std::env::var("CITADEL_EXECUTION_DATABASE_URL").unwrap();
    MigrationRunner::migrate(&url).await.unwrap();
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&url)
        .await
        .unwrap();
    let admin = seed_administrator(&pool).await;
    let ids = seed_dependencies(&pool, admin.actor_id).await;
    (pool, admin, ids)
}
async fn deployment(pool: &sqlx::PgPool, actor: ActorId, platform: Uuid) -> Uuid {
    PostgresDeploymentRepository::new(pool.clone())
        .create(
            actor,
            true,
            &citadel_deployments::CreateDeployment {
                name: format!("destination-source-{}", Uuid::now_v7().simple()),
                platform_id: platform,
                description: None,
                tag_ids: vec![],
                duplicate_source: None,
                spec: serde_json::from_value(
                    json!({"image":{"$type":"Local","imageId":"fixture"},"volumes":["data:/data"]}),
                )
                .unwrap(),
            },
        )
        .await
        .unwrap()
        .id
}
async fn snapshot(
    pool: &sqlx::PgPool,
    actor: ActorId,
    ids: &FixtureIds,
    node: Option<&str>,
) -> Uuid {
    let repo = Uuid::now_v7();
    let policy = Uuid::now_v7();
    let run = Uuid::now_v7();
    sqlx::query("INSERT INTO backuprepositories(id,createdbyactorid,name,normalizedname,passwordsecretid,spec,status,type) VALUES($1,$2,$1::text,$1::text,$3,'{}','Ready','S3Compatible')").bind(repo).bind(actor.value()).bind(ids.secret).execute(pool).await.unwrap();
    sqlx::query("INSERT INTO backuppolicies(id,backuprepositoryid,createdbyactorid,name,normalizedname,runasactorid,source) VALUES($1,$2,$3,$1::text,$1::text,$3,$4)").bind(policy).bind(repo).bind(actor.value()).bind(json!({"$type":"DockerVolume","platformId":ids.platform,"volumeName":"data"})).execute(pool).await.unwrap();
    sqlx::query("INSERT INTO backupruns(id,backuppolicyid,backuprepositoryid,policynamesnapshot,repositorytypesnapshot,snapshotavailability,sourcesnapshot,status,trigger,triggeredbyactorid,completedat) SELECT $1,id,backuprepositoryid,name,'S3Compatible','Available',source,'Succeeded','Manual',createdbyactorid,CURRENT_TIMESTAMP FROM backuppolicies WHERE id=$2").bind(run).bind(policy).execute(pool).await.unwrap();
    sqlx::query("INSERT INTO backuprunitems(id,backuprunid,platformid,dockernodeid,volumename,status,resticsnapshotid,completedat) VALUES($1,$2,$3,$4,'data','Succeeded','snapshot',CURRENT_TIMESTAMP)").bind(Uuid::now_v7()).bind(run).bind(ids.platform).bind(node).execute(pool).await.unwrap();
    repo
}
async fn plan(
    store: &PostgresCopyRepository,
    actor: ActorId,
    kind: CopyKind,
    id: Uuid,
    target: Uuid,
    node: Option<&str>,
) -> CopyPlan {
    let source = store.source(actor, kind, id).await.unwrap();
    let snapshots = store.snapshots(actor, &source).await.unwrap();
    let request = CopyRequest {
        destination_platform_id: Some(target),
        destination_node_id: node.map(str::to_owned),
        data_mode: CopyDataMode::ExistingBackup,
        consistency_acknowledged: false,
        kind,
        source_id: id,
        name: format!("target-copy-{}", Uuid::now_v7().simple()),
        selections: snapshots
            .iter()
            .map(|s| CopySelection {
                volume: s.volume.clone(),
                backup_run_id: s.backup_run_id,
                backup_run_item_id: s.backup_run_item_id,
            })
            .collect(),
    };
    let mut plan = planning::plan(Uuid::now_v7(), source, request, snapshots).unwrap();
    store.prepare_destination(actor, &mut plan).await.unwrap();
    plan
}
async fn finish(
    store: &PostgresCopyRepository,
    pool: &sqlx::PgPool,
    actor: ActorId,
    plan: &CopyPlan,
) -> Uuid {
    store.save_review(actor, plan).await.unwrap();
    let started = store.start(actor, plan.preview.id).await.unwrap();
    assert_eq!(started.platform_id, plan.target_platform());
    for restore in &started.restores {
        let row = sqlx::query(
            "SELECT targetplatformid,targetdockernodeid FROM backuprestoreruns WHERE id=$1",
        )
        .bind(restore.id)
        .fetch_one(pool)
        .await
        .unwrap();
        assert_eq!(
            row.get::<Uuid, _>("targetplatformid"),
            plan.target_platform()
        );
        assert_eq!(
            row.get::<Option<String>, _>("targetdockernodeid")
                .as_deref(),
            plan.target_node(&plan.preview.volumes[0])
        );
        // This fixture verifies persistence. Runtime data/ownership is covered by Restic acceptance.
        sqlx::query("UPDATE backuprestoreruns SET status='Succeeded',completedat=CURRENT_TIMESTAMP WHERE id=$1").bind(restore.id).execute(pool).await.unwrap();
    }
    let completed = store
        .advance(actor, plan.preview.id, Some(Ok(())))
        .await
        .unwrap();
    assert_eq!(
        completed.status,
        CopyStatus::Succeeded,
        "{:?}",
        completed.message
    );
    let id = completed.destination_id.unwrap();
    let receipts =
        citadel_adapters::persistence::postgres::resource_copies::storage::destination_receipts(
            pool, id,
        )
        .await
        .unwrap();
    assert_eq!(receipts.len(), 1);
    assert_eq!(receipts[0].platform_id, plan.target_platform());
    assert_eq!(
        receipts[0].node_id.as_deref(),
        plan.target_node(&plan.preview.volumes[0])
    );
    id
}

#[tokio::test]
#[ignore = "requires disposable CITADEL_EXECUTION_DATABASE_URL"]
async fn standalone_destinations_route_restores_fence_identity_and_reject_local_repositories() {
    let _lock = FIXTURE_LOCK.lock().await;
    let (pool, admin, ids) = fixture().await;
    let actor = admin.actor_id;
    let target = seed_dependencies(&pool, actor).await;
    let wrong = seed_dependencies(&pool, actor).await;
    sqlx::query(
        "UPDATE platforms SET platformdescriptor='{\"$type\":\"DockerSwarm\"}' WHERE id=$1",
    )
    .bind(wrong.platform)
    .execute(&pool)
    .await
    .unwrap();
    let source = deployment(&pool, actor, ids.platform).await;
    let repo = snapshot(&pool, actor, &ids, None).await;
    let store = PostgresCopyRepository::new(pool.clone());
    let catalog = store
        .targets(actor, CopyKind::Deployment, source)
        .await
        .unwrap();
    assert!(catalog.platforms.iter().any(|p| p.id == target.platform));
    assert!(!catalog.platforms.iter().any(|p| p.id == wrong.platform));
    let good = plan(
        &store,
        actor,
        CopyKind::Deployment,
        source,
        target.platform,
        None,
    )
    .await;
    assert!(good.preview.blockers.is_empty());
    assert!(good.preview.volumes[0].snapshots[0].supports_cross_platform_restore);
    for (platform, mode, node) in [
        (wrong.platform, CopyDataMode::ExistingBackup, Some("node")),
        (target.platform, CopyDataMode::CurrentData, None),
    ] {
        let mut forged = good.clone();
        forged.request.destination_platform_id = Some(platform);
        forged.request.destination_node_id = node.map(str::to_owned);
        forged.request.data_mode = mode;
        assert!(store.prepare_destination(actor, &mut forged).await.is_err());
    }
    store.save_review(actor, &good).await.unwrap();
    sqlx::query("UPDATE platforms SET address=address || '/changed' WHERE id=$1")
        .bind(target.platform)
        .execute(&pool)
        .await
        .unwrap();
    assert!(matches!(
        store.start(actor, good.preview.id).await,
        Err(CopyError::Conflict(_))
    ));
    let good = plan(
        &store,
        actor,
        CopyKind::Deployment,
        source,
        target.platform,
        None,
    )
    .await;
    sqlx::query("UPDATE backuprepositories SET type='FileSystem' WHERE id=$1")
        .bind(repo)
        .execute(&pool)
        .await
        .unwrap();
    assert!(store.save_review(actor, &good).await.is_err());
    let service = CopyService::new(
        Arc::new(PostgresCopyRepository::new(pool.clone())),
        Arc::new(CopyStorageRuntime {
            image: "unused".into(),
            runtime: Arc::new(PlatformRuntimeRouter::new(
                pool.clone(),
                DockerClient::new(
                    "/no-preview-runtime.sock",
                    std::time::Duration::from_secs(1),
                )
                .unwrap(),
                None,
                Default::default(),
            )),
        }),
        Arc::new(BackupService::new(
            Arc::new(PostgresBackupPersistence::new(pool.clone())),
            Arc::new(FakeBackupExecutor),
            Arc::new(FakeBackupPlanner::default()),
            Duration::minutes(5),
            Arc::new(AllowBackupExecution),
        )),
        Arc::new(|_, _| {}),
    );
    let filtered = service.preview(actor, good.request.clone()).await.unwrap();
    assert!(
        filtered.volumes[0].snapshots.is_empty(),
        "host-local snapshots must not be offered remotely"
    );
    assert!(!filtered.blockers.is_empty());
    assert!(
        !store.snapshots(actor, &good.source).await.unwrap()[0].supports_cross_platform_restore
    );
    sqlx::query("UPDATE backuprepositories SET type='S3Compatible' WHERE id=$1")
        .bind(repo)
        .execute(&pool)
        .await
        .unwrap();
    let copied = finish(&store, &pool, actor, &good).await;
    let row = sqlx::query("SELECT platformid,spec FROM deployments WHERE id=$1")
        .bind(copied)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(row.get::<Uuid, _>("platformid"), target.platform);
    let spec: Value = row.get("spec");
    assert_eq!(
        spec["Volumes"][0],
        format!("{}:/data", good.preview.volumes[0].target)
    );
    assert_eq!(
        store
            .source(actor, CopyKind::Deployment, source)
            .await
            .unwrap()
            .platform_id,
        ids.platform
    );
    // Destination drift after start prevents finalization, too.
    let stale = plan(
        &store,
        actor,
        CopyKind::Deployment,
        source,
        target.platform,
        None,
    )
    .await;
    store.save_review(actor, &stale).await.unwrap();
    store.start(actor, stale.preview.id).await.unwrap();
    sqlx::query("UPDATE platforms SET address=address || '/changed' WHERE id=$1")
        .bind(target.platform)
        .execute(&pool)
        .await
        .unwrap();
    let stopped = store.advance(actor, stale.preview.id, None).await.unwrap();
    assert_eq!(stopped.status, CopyStatus::Failed);
    assert!(stopped.destination_id.is_none());
    assert!(stopped.restores.iter().all(|r| r.status == "Cancelled"));
    pool.close().await;
}

#[tokio::test]
#[ignore = "requires disposable CITADEL_EXECUTION_DATABASE_URL"]
async fn destination_catalog_filters_permissions_and_rechecks_revocation() {
    use citadel_adapters::persistence::postgres::identity::users::repository::PostgresUserRepository;
    use citadel_identity::{ResourceAccessInput, UserRepository};
    use citadel_primitives::PermissionLevel;
    let _lock = FIXTURE_LOCK.lock().await;
    let (pool, admin, ids) = fixture().await;
    let regular = seed_regular_user(&pool).await;
    let target = seed_dependencies(&pool, admin.actor_id).await;
    let hidden = seed_dependencies(&pool, admin.actor_id).await;
    let source = deployment(&pool, admin.actor_id, ids.platform).await;
    let role = Uuid::now_v7();
    sqlx::query("INSERT INTO roles(id,name,roletype) VALUES($1,$1::text,'Custom')")
        .bind(role)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO permissions(id,roleid,resourcetype,permissionlevel,specificpermissions) VALUES($1,$2,$3,2,0)").bind(Uuid::now_v7()).bind(role).bind(ResourceType::Deployment as i32).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO actorroles(actorid,roleid) VALUES($1,$2)")
        .bind(regular.actor_id.value())
        .bind(role)
        .execute(&pool)
        .await
        .unwrap();
    let users = PostgresUserRepository::new(pool.clone());
    let mut grants = vec![];
    for platform in [ids.platform, target.platform] {
        let grant = ResourceAccessInput {
            resource_type: ResourceType::Platform,
            resource_id: platform,
            permission_level: if platform == target.platform {
                PermissionLevel::Execute
            } else {
                PermissionLevel::Write
            },
            specific_permissions: vec![],
        };
        users
            .add_resource_access(regular.subject_id, &grant, admin.actor_id, Utc::now(), true)
            .await
            .unwrap();
        grants.push(grant);
    }
    let store = PostgresCopyRepository::new(pool.clone());
    let catalog = store
        .targets(regular.actor_id, CopyKind::Deployment, source)
        .await
        .unwrap();
    assert_eq!(catalog.platforms.len(), 2);
    assert!(!catalog.platforms.iter().any(|p| p.id == hidden.platform));
    let mut review = plan(
        &store,
        regular.actor_id,
        CopyKind::Deployment,
        source,
        target.platform,
        None,
    )
    .await;
    users
        .remove_resource_access(regular.subject_id, &grants[1], admin.actor_id, Utc::now())
        .await
        .unwrap();
    assert!(matches!(
        store
            .prepare_destination(regular.actor_id, &mut review)
            .await,
        Err(CopyError::NotFound)
    ));
    assert_eq!(
        store
            .targets(regular.actor_id, CopyKind::Deployment, source)
            .await
            .unwrap()
            .platforms
            .len(),
        1
    );
    review.request.destination_platform_id = Some(hidden.platform);
    assert!(matches!(
        store
            .prepare_destination(regular.actor_id, &mut review)
            .await,
        Err(CopyError::NotFound)
    ));
    pool.close().await;
}

async fn swarm_inventory(pool: &sqlx::PgPool, platform: Uuid, prefix: &str) {
    sqlx::query("UPDATE platforms SET platformdescriptor='{\"$type\":\"DockerSwarm\",\"controlAvailable\":true,\"localNodeState\":\"active\"}',clusterid=$2 WHERE id=$1").bind(platform).bind(format!("cluster{prefix}")).execute(pool).await.unwrap();
    sqlx::query("INSERT INTO swarmnodeprojections(platformid,dockernodeid,address,architecture,availability,desiredtaskcount,engineversion,hostname,isleader,labels,observedat,operatingsystem,reachability,role,runningtaskcount,status,versionindex) VALUES($1,$2,'127.0.0.1','amd64','Active',0,'fixture',$2,false,'{}',CURRENT_TIMESTAMP,'linux','Reachable','Worker',0,'Ready',1)").bind(platform).bind(format!("{prefix}node")).execute(pool).await.unwrap();
    sqlx::query("INSERT INTO swarmnetworkprojections(platformid,dockernetworkid,driver,enableipv6,isattachable,isencrypted,isingress,isinternal,labels,name,observedat,scope,servicenames,subnets) VALUES($1,$2,'overlay',false,true,false,false,false,'{}','app-network',CURRENT_TIMESTAMP,'swarm','[]','[]')").bind(platform).bind(format!("{prefix}network")).execute(pool).await.unwrap();
    sqlx::query("INSERT INTO swarmsecretprojections(platformid,dockersecretid,labels,name,observedat,servicenames,versionindex) VALUES($1,$2,'{}','app-secret',CURRENT_TIMESTAMP,'[]',1)").bind(platform).bind(format!("{prefix}secret")).execute(pool).await.unwrap();
    sqlx::query("INSERT INTO swarmconfigprojections(platformid,dockerconfigid,labels,name,observedat,servicenames,versionindex) VALUES($1,$2,'{}','app-config',CURRENT_TIMESTAMP,'[]',1)").bind(platform).bind(format!("{prefix}config")).execute(pool).await.unwrap();
}

#[tokio::test]
#[ignore = "requires disposable CITADEL_EXECUTION_DATABASE_URL"]
async fn swarm_destinations_remap_references_and_pin_restores_to_reviewed_node() {
    use citadel_adapters::persistence::postgres::swarm_services::PostgresSwarmServiceRepository;
    use citadel_swarm_services::SwarmServiceRepository;
    let _lock = FIXTURE_LOCK.lock().await;
    let (pool, admin, ids) = fixture().await;
    let actor = admin.actor_id;
    let target = seed_dependencies(&pool, actor).await;
    swarm_inventory(&pool, ids.platform, "source").await;
    swarm_inventory(&pool, target.platform, "target").await;
    let services = PostgresSwarmServiceRepository::new(pool.clone());
    let source = services.create(actor,true,&citadel_swarm_services::CreateSwarmService {
        name:format!("cross-swarm-{}",Uuid::now_v7().simple()),platform_id:ids.platform,description:None,tag_ids:vec![],duplicate_source:None,
        spec:serde_json::from_value(json!({"image":{"$type":"External","registryId":ids.registry,"imageTag":"alpine"},"replicas":1,
            "mounts":[{"kind":"Volume","source":"data","target":"/data","readOnly":false}],"placementConstraints":["node.id == sourcenode","node.labels.storage == yes"],
            "networkIds":["sourcenetwork"], "secrets":[{"secretId":"sourcesecret","secretName":"app-secret","targetName":"password"}],
            "configs":[{"configId":"sourceconfig","configName":"app-config","targetName":"config"}]})).unwrap(),
    }).await.unwrap();
    snapshot(&pool, actor, &ids, Some("sourcenode")).await;
    let store = PostgresCopyRepository::new(pool.clone());
    let catalog = store
        .targets(actor, CopyKind::SwarmService, source.id)
        .await
        .unwrap();
    assert!(catalog.swarm);
    assert_eq!(catalog.source_node_id.as_deref(), Some("sourcenode"));
    let good = plan(
        &store,
        actor,
        CopyKind::SwarmService,
        source.id,
        target.platform,
        Some("targetnode"),
    )
    .await;
    let CopySpec::SwarmService(spec) = &good.spec else {
        panic!()
    };
    assert_eq!(spec.network_ids, ["targetnetwork"]);
    assert_eq!(spec.secrets[0].secret_id, "targetsecret");
    assert_eq!(spec.configs[0].config_id, "targetconfig");
    assert_eq!(
        spec.placement_constraints,
        ["node.id == targetnode", "node.labels.storage == yes"]
    );
    assert_eq!(
        good.preview.volumes[0].docker_node_id.as_deref(),
        Some("sourcenode")
    );
    assert_eq!(
        good.preview.destination_node_id.as_deref(),
        Some("targetnode")
    );
    let mut bad = good.clone();
    bad.request.destination_node_id = None;
    assert!(store.prepare_destination(actor, &mut bad).await.is_err());
    bad.request.destination_node_id = Some("missing".into());
    assert!(store.prepare_destination(actor, &mut bad).await.is_err());
    sqlx::query("UPDATE swarmnodeprojections SET availability='Drain' WHERE platformid=$1")
        .bind(target.platform)
        .execute(&pool)
        .await
        .unwrap();
    assert!(store.save_review(actor, &good).await.is_err());
    sqlx::query("UPDATE swarmnodeprojections SET availability='Active' WHERE platformid=$1")
        .bind(target.platform)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE swarmsecretprojections SET isstale=true WHERE platformid=$1")
        .bind(target.platform)
        .execute(&pool)
        .await
        .unwrap();
    assert!(store.save_review(actor, &good).await.is_err());
    sqlx::query("UPDATE swarmsecretprojections SET isstale=false WHERE platformid=$1")
        .bind(target.platform)
        .execute(&pool)
        .await
        .unwrap();
    let id = finish(&store, &pool, actor, &good).await;
    let copy = store
        .source(actor, CopyKind::SwarmService, id)
        .await
        .unwrap();
    assert_eq!(copy.platform_id, target.platform);
    let CopySpec::SwarmService(spec) = copy.spec else {
        panic!()
    };
    assert_eq!(planning::pinned_node(&spec), Some("targetnode"));
    assert_eq!(spec.network_ids, ["targetnetwork"]);
    pool.close().await;
}

#[tokio::test]
#[ignore = "requires disposable CITADEL_EXECUTION_DATABASE_URL"]
async fn copied_stack_locks_its_platform_but_ordinary_drafts_remain_movable() {
    use citadel_adapters::persistence::postgres::stacks::PostgresStackRepository;
    use citadel_stacks::{CreateStack, StackRepository, StackSource, UpdateStack};

    let _lock = FIXTURE_LOCK.lock().await;
    let (pool, admin, ids) = fixture().await;
    let actor = admin.actor_id;
    let target = seed_dependencies(&pool, actor).await;
    let stacks = PostgresStackRepository::new(pool.clone());
    let source = stacks.create(actor, true, &CreateStack {
        name: format!("movable-stack-{}", Uuid::now_v7().simple()),
        platform_id: ids.platform,
        description: None,
        stack_source: StackSource::WebEditor,
        spec: serde_json::from_value(json!({
            "$type": "WebEditor",
            "composeFile": "services:\n  app:\n    image: alpine\n    volumes: [data:/data]\nvolumes:\n  data:\n    external: true\n    name: data\n"
        })).unwrap(),
        drift_policy: None,
        tag_ids: vec![],
        duplicate_source: None,
    }).await.unwrap();
    assert!(!source.has_copied_storage);
    let patch = |platform| UpdateStack {
        platform_id: Some(platform),
        name: None,
        description: Some(Some("Editable metadata".into())),
        stack_source: None,
        spec: None,
        drift_policy: None,
        row_version: None,
    };
    // A draft with ordinary configuration is still free to choose another host.
    let moved = stacks
        .update(
            actor,
            true,
            source.id,
            source.row_version,
            &patch(target.platform),
            source.spec.as_ref().unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(moved.platform_id, Some(target.platform));
    stacks
        .update(
            actor,
            true,
            source.id,
            moved.row_version,
            &patch(ids.platform),
            moved.spec.as_ref().unwrap(),
        )
        .await
        .unwrap();

    snapshot(&pool, actor, &ids, None).await;
    let copies = PostgresCopyRepository::new(pool.clone());
    let review = plan(
        &copies,
        actor,
        CopyKind::Stack,
        source.id,
        target.platform,
        None,
    )
    .await;
    assert!(
        review.preview.blockers.is_empty(),
        "{:?}",
        review.preview.blockers
    );
    let copied_id = finish(&copies, &pool, actor, &review).await;
    let copied = stacks.get_authorized(actor, true, copied_id).await.unwrap();
    assert!(copied.has_copied_storage);
    assert_eq!(copied.platform_id, Some(target.platform));
    let view =
        citadel_server::api::resources::stacks::views::StackView::try_from(copied.clone()).unwrap();
    assert_eq!(
        serde_json::to_value(view).unwrap()["hasCopiedStorage"],
        true
    );
    let listed = stacks
        .list_authorized(actor, true, &Default::default())
        .await
        .unwrap();
    assert!(
        listed
            .iter()
            .find(|s| s.id == copied_id)
            .unwrap()
            .has_copied_storage
    );

    let result = stacks
        .update(
            actor,
            true,
            copied_id,
            copied.row_version,
            &patch(ids.platform),
            copied.spec.as_ref().unwrap(),
        )
        .await;
    assert!(
        matches!(result, Err(citadel_stacks::StackError::Validation(ref message)) if message.contains("copied volumes"))
    );
    let unchanged = stacks.get_authorized(actor, true, copied_id).await.unwrap();
    assert_eq!(unchanged.platform_id, copied.platform_id);
    assert_eq!(unchanged.row_version, copied.row_version);
    // The lock only restricts relocation; normal configuration edits still work.
    let edited = stacks
        .update(
            actor,
            true,
            copied_id,
            copied.row_version,
            &patch(target.platform),
            copied.spec.as_ref().unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(edited.description.as_deref(), Some("Editable metadata"));
    assert!(edited.has_copied_storage);
}
