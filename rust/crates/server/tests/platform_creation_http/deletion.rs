use super::*;

async fn delete(
    app: &Router,
    actor: Option<ActorPrincipal>,
    body: Value,
) -> axum::response::Response {
    let mut request = Request::builder()
        .method(Method::DELETE)
        .uri("/api/v1/platforms")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&body).unwrap()))
        .unwrap();
    if let Some(actor) = actor {
        request.extensions_mut().insert(actor);
    }
    app.clone().oneshot(request).await.unwrap()
}

async fn registered(harness: &TestHarness) -> Uuid {
    let name = format!("delete-{}", Uuid::now_v7().simple());
    let response = request(
        &harness.app,
        Some(harness.administrator.clone()),
        agent_input(&name, "Docker"),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    Uuid::parse_str(response_json(response).await["id"].as_str().unwrap()).unwrap()
}

async fn exists(harness: &TestHarness, id: Uuid) -> bool {
    sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM platforms WHERE id=$1)")
        .bind(id)
        .fetch_one(&harness.pool)
        .await
        .unwrap()
}

async fn deleted_activities(harness: &TestHarness, id: Uuid) -> i64 {
    sqlx::query_scalar(
        "SELECT count(*) FROM activityevents WHERE resourceid=$1 AND eventtype='PlatformDeleted'",
    )
    .bind(id)
    .fetch_one(&harness.pool)
    .await
    .unwrap()
}

// Ports PlatformDeleteTests.Delete_Platform_Should_Delete_Entity_And_Add_Activity,
// including the historical backup-item FK regression, against the real PostgreSQL schema.
#[tokio::test]
#[ignore = "requires CITADEL_PHASE4_DATABASE_URL"]
async fn delete_platform_persists_snapshot_cascades_inventory_and_backup_items_and_notifies() {
    let harness = harness(StaticInventory::standalone(Uuid::now_v7().to_string())).await;
    let id = registered(&harness).await;
    let run = seed_backup_item(&harness, id).await;
    let mut events = harness.realtime.subscribe();
    let response = delete(
        &harness.app,
        Some(harness.administrator.clone()),
        json!({"ids":[id,id]}),
    )
    .await;
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "{}",
        response_json(response).await
    );
    assert!(!exists(&harness, id).await);
    assert_eq!(deleted_activities(&harness, id).await, 1);
    let (snapshot, platform_id): (Value, Option<Uuid>) = sqlx::query_as("SELECT info::jsonb, platformid FROM activityevents WHERE resourceid=$1 AND eventtype='PlatformDeleted'")
        .bind(id).fetch_one(&harness.pool).await.unwrap();
    assert_eq!(snapshot["Platform"]["Id"], id.to_string());
    assert!(
        snapshot["Platform"]["Address"]
            .as_str()
            .unwrap()
            .starts_with("https://delete-")
    );
    assert!(
        snapshot["Platform"]["Name"]
            .as_str()
            .unwrap()
            .starts_with("delete-")
    );
    assert_eq!(platform_id, None);
    for query in [
        "SELECT count(*) FROM containers WHERE platformid=$1",
        "SELECT count(*) FROM images WHERE platformid=$1",
        "SELECT count(*) FROM backuprunitems WHERE platformid=$1",
    ] {
        let count: i64 = sqlx::query_scalar(query)
            .bind(id)
            .fetch_one(&harness.pool)
            .await
            .unwrap();
        assert_eq!(count, 0, "{query}");
    }
    let retained_run: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM backupruns WHERE id=$1)")
            .bind(run)
            .fetch_one(&harness.pool)
            .await
            .unwrap();
    assert!(retained_run);
    let event = events.try_recv().unwrap();
    assert_eq!(event.resource_type(), "Platform");
    // The fake runtime was consulted only during creation, never for deletion.
    assert_eq!(harness.runtime.selections.lock().unwrap().len(), 1);
}

// Ports PlatformDeleteTests.Delete_Platforms_Should_Not_Partially_Delete_When_Later_Id_Does_Not_Exist.
#[tokio::test]
#[ignore = "requires CITADEL_PHASE4_DATABASE_URL"]
async fn delete_missing_platform_rolls_back_entire_selection_without_notifications() {
    let harness = harness(StaticInventory::standalone(Uuid::now_v7().to_string())).await;
    let id = registered(&harness).await;
    let mut events = harness.realtime.subscribe();
    let response = delete(
        &harness.app,
        Some(harness.administrator.clone()),
        json!({"ids":[id,Uuid::now_v7()]}),
    )
    .await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert!(exists(&harness, id).await);
    assert_eq!(deleted_activities(&harness, id).await, 0);
    assert!(events.try_recv().is_err());
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE4_DATABASE_URL"]
async fn delete_validates_input_and_requires_execute_permission() {
    let harness = harness(StaticInventory::standalone(Uuid::now_v7().to_string())).await;
    let id = registered(&harness).await;
    assert_eq!(
        delete(&harness.app, None, json!({"ids":[id]}))
            .await
            .status(),
        StatusCode::UNAUTHORIZED
    );
    let reader = seed_actor(&harness.pool, false).await;
    assert_eq!(
        delete(&harness.app, Some(reader.clone()), json!({"ids":[id]}))
            .await
            .status(),
        StatusCode::FORBIDDEN
    );
    for body in [
        json!({}),
        json!({"ids":[]}),
        json!({"ids":["bad"]}),
        json!({"ids":[Uuid::nil()]}),
    ] {
        assert_eq!(
            delete(&harness.app, Some(harness.administrator.clone()), body)
                .await
                .status(),
            StatusCode::BAD_REQUEST
        );
    }
    assert!(exists(&harness, id).await);
    assert_eq!(deleted_activities(&harness, id).await, 0);
    // Resource-scoped Execute grants work without an Administrator role.
    sqlx::query("INSERT INTO resourceaccesses (id,actorid,permissionlevel,resourceid,resourcetype,specificpermissions) VALUES ($1,$2,$3,$4,$5,0)")
        .bind(Uuid::now_v7()).bind(reader.actor_id.value()).bind(citadel_domain::PermissionLevel::Execute as i32)
        .bind(id).bind(citadel_domain::ResourceType::Platform as i32).execute(&harness.pool).await.unwrap();
    assert_eq!(
        delete(&harness.app, Some(reader), json!({"ids":[id]}))
            .await
            .status(),
        StatusCode::OK
    );
    let remaining: i64 =
        sqlx::query_scalar("SELECT count(*) FROM resourceaccesses WHERE resourceid=$1")
            .bind(id)
            .fetch_one(&harness.pool)
            .await
            .unwrap();
    assert_eq!(remaining, 0);
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE4_DATABASE_URL"]
async fn referenced_platform_returns_conflict_without_deleting_any_selection_or_audit() {
    let harness = harness(StaticInventory::standalone(Uuid::now_v7().to_string())).await;
    let id = registered(&harness).await;
    let other = Uuid::now_v7();
    sqlx::query("INSERT INTO platforms (id,name,address,connectortype,cpucount,imagecount,memtotal,networkcount,platformdescriptor,status,volumecount) VALUES ($1,$2,$2,'Agent',0,0,0,0,'{}','Offline',0)")
        .bind(other).bind(other.to_string()).execute(&harness.pool).await.unwrap();
    sqlx::query("INSERT INTO deployments (id,name,platformid,spec,status,createdbyactorid) VALUES ($1,'dependent',$2,'{}','Created',$3)")
        .bind(Uuid::now_v7()).bind(id).bind(harness.administrator.actor_id.value()).execute(&harness.pool).await.unwrap();
    use citadel_platforms::deletion::{PlatformDeletionError, PlatformDeletionStore};
    let result = citadel_adapters::platform_deletion::PostgresPlatformDeletionStore::new(
        harness.pool.clone(),
    )
    .delete(harness.administrator.actor_id, &[id, other])
    .await;
    assert!(
        matches!(result, Err(PlatformDeletionError::InUse)),
        "{result:?}"
    );
    let mut events = harness.realtime.subscribe();
    let response = delete(
        &harness.app,
        Some(harness.administrator.clone()),
        json!({"ids":[other,id]}),
    )
    .await;
    assert_eq!(
        response.status(),
        StatusCode::CONFLICT,
        "{}",
        response_json(response).await
    );
    for id in [other, id] {
        assert!(exists(&harness, id).await);
        assert_eq!(deleted_activities(&harness, id).await, 0);
    }
    assert!(events.try_recv().is_err());
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE4_DATABASE_URL"]
async fn concurrent_deletes_emit_only_one_activity_and_notification() {
    let harness = harness(StaticInventory::standalone(Uuid::now_v7().to_string())).await;
    let id = registered(&harness).await;
    let mut events = harness.realtime.subscribe();
    let (first, second) = tokio::join!(
        delete(
            &harness.app,
            Some(harness.administrator.clone()),
            json!({"ids":[id]})
        ),
        delete(
            &harness.app,
            Some(harness.administrator.clone()),
            json!({"ids":[id]})
        )
    );
    let mut statuses = [first.status(), second.status()];
    statuses.sort();
    assert_eq!(statuses, [StatusCode::OK, StatusCode::NOT_FOUND]);
    assert_eq!(deleted_activities(&harness, id).await, 1);
    assert!(events.try_recv().is_ok());
    assert!(events.try_recv().is_err());
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE4_DATABASE_URL"]
async fn deletion_revokes_edge_credentials_and_disconnects_sessions_only_after_commit() {
    use citadel_adapters::edge::EdgeTarget;
    let harness = harness(StaticInventory::standalone(Uuid::now_v7().to_string())).await;
    let id = registered(&harness).await;
    let enrollment = Uuid::now_v7();
    let agent = Uuid::now_v7();
    sqlx::query("INSERT INTO edgeagentenrollments (id,createdbyactorid,expiresatutc,platformid,resourceid,tokenhash) VALUES ($1,$2,now()+interval '1 hour',$3,$3,$4)")
        .bind(enrollment).bind(harness.administrator.actor_id.value()).bind(id).bind(enrollment.to_string())
        .execute(&harness.pool).await.unwrap();
    sqlx::query("INSERT INTO edgeagentbindings (id,agentfingerprint,agentid,agentpublickey,connectionstatus,platformid,resourceid) VALUES ($1,$2,$1,'test-key','Connected',$3,$3)")
        .bind(agent).bind(agent.to_string()).bind(id).execute(&harness.pool).await.unwrap();
    let (session, _outbound) = harness
        .edge
        .register(EdgeTarget::platform(id), agent)
        .unwrap();
    let response = delete(
        &harness.app,
        Some(harness.administrator.clone()),
        json!({"ids":[id,Uuid::now_v7()]}),
    )
    .await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert!(!session.is_closed());
    let active: bool =
        sqlx::query_scalar("SELECT revokedatutc IS NULL FROM edgeagentenrollments WHERE id=$1")
            .bind(enrollment)
            .fetch_one(&harness.pool)
            .await
            .unwrap();
    assert!(active);
    assert_eq!(
        delete(
            &harness.app,
            Some(harness.administrator.clone()),
            json!({"ids":[id]})
        )
        .await
        .status(),
        StatusCode::OK
    );
    assert!(session.is_closed());
    let revoked: (bool, bool) = sqlx::query_as("SELECT (SELECT revokedatutc IS NOT NULL FROM edgeagentenrollments WHERE id=$1),(SELECT revokedatutc IS NOT NULL FROM edgeagentbindings WHERE id=$2)")
        .bind(enrollment).bind(agent).fetch_one(&harness.pool).await.unwrap();
    assert_eq!(revoked, (true, true));
}

async fn seed_backup_item(harness: &TestHarness, platform: Uuid) -> Uuid {
    let secret = Uuid::now_v7();
    let repository = Uuid::now_v7();
    let policy = Uuid::now_v7();
    let run = Uuid::now_v7();
    let actor = harness.administrator.actor_id.value();
    let source = json!({"$type":"DockerVolume","platformId":platform,"volumeName":"backup-volume"});
    sqlx::query(
        "INSERT INTO secretdefinitions (id,name,providertype) VALUES ($1,$2,'InternalEncrypted')",
    )
    .bind(secret)
    .bind(format!("secret-{secret}"))
    .execute(&harness.pool)
    .await
    .unwrap();
    sqlx::query("INSERT INTO backuprepositories (id,name,normalizedname,type,spec,passwordsecretid,status,controlstate,createdbyactorid) VALUES ($1,$2,$2,'FileSystem','{}',$3,'Unknown','Idle',$4)")
        .bind(repository).bind(repository.to_string()).bind(secret).bind(actor).execute(&harness.pool).await.unwrap();
    sqlx::query("INSERT INTO backuppolicies (id,name,normalizedname,source,backuprepositoryid,runasactorid,createdbyactorid) VALUES ($1,$2,$2,$3,$4,$5,$5)")
        .bind(policy).bind(policy.to_string()).bind(&source).bind(repository).bind(actor).execute(&harness.pool).await.unwrap();
    sqlx::query("INSERT INTO backupruns (id,backuppolicyid,backuprepositoryid,policynamesnapshot,sourcesnapshot,repositorytypesnapshot,trigger,status,snapshotavailability,triggeredbyactorid) VALUES ($1,$2,$3,'backup-policy',$4,'FileSystem','Manual','Succeeded','Available',$5)")
        .bind(run).bind(policy).bind(repository).bind(source).bind(actor).execute(&harness.pool).await.unwrap();
    sqlx::query("INSERT INTO backuprunitems (id,backuprunid,platformid,volumename,status) VALUES ($1,$2,$3,'backup-volume','Succeeded')")
        .bind(Uuid::now_v7()).bind(run).bind(platform).execute(&harness.pool).await.unwrap();
    run
}
