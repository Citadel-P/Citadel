use super::*;

// Ports BackupPolicyCommandsTests' patch/active-run/immutable-source and licensed
// trigger cases through HTTP and PostgreSQL, rather than a handler-only mock.
pub(super) async fn verify_policy(
    app: &Router,
    pool: &sqlx::PgPool,
    admin: &ActorPrincipal,
    policy: &Value,
) {
    let id = policy["id"].as_str().unwrap();
    let uuid = Uuid::parse_str(id).unwrap();
    let path = format!("/api/v1/backupPolicies/{id}");
    let regular = seed_regular_user(pool).await;
    for (actor, status) in [
        (None, StatusCode::UNAUTHORIZED),
        (Some(regular), StatusCode::FORBIDDEN),
    ] {
        assert_eq!(
            request(
                app,
                Method::PATCH,
                &path,
                actor,
                Some(json!({"enabled":false}))
            )
            .await
            .status(),
            status
        );
    }
    for body in [
        json!([]),
        json!({"timeoutSeconds":1}),
        json!({"description":"x".repeat(601)}),
        json!({"cron":"bad","timeZone":"UTC"}),
        json!({"webhook":{"enabled":true}}),
    ] {
        assert_eq!(
            request(app, Method::PATCH, &path, Some(admin.clone()), Some(body))
                .await
                .status(),
            StatusCode::BAD_REQUEST
        );
    }
    let response=request(app,Method::PATCH,&path,Some(admin.clone()),Some(json!({"description":" edited ","keepLastSuccessful":3,"name":"ignored","controlState":"Processing"}))).await;
    let status = response.status();
    let saved = response_json(response).await;
    assert_eq!(status, StatusCode::OK, "{saved}");
    assert_eq!(saved["description"], "edited");
    assert_eq!(saved["keepLastSuccessful"], 3);
    assert_eq!(saved["controlState"], "Idle");
    assert_ne!(saved["name"], "ignored");
    let stored: i32 =
        sqlx::query_scalar("SELECT keeplastsuccessful FROM backuppolicies WHERE id=$1")
            .bind(uuid)
            .fetch_one(pool)
            .await
            .unwrap();
    assert_eq!(stored, 3);
    let licensed = request(
        app,
        Method::PATCH,
        &path,
        Some(admin.clone()),
        Some(json!({"cron":"0 2 * * *","timeZone":"UTC"})),
    )
    .await;
    assert_eq!(licensed.status(), StatusCode::FORBIDDEN);
    sqlx::query("UPDATE backuppolicies SET controlstate='Processing' WHERE id=$1")
        .bind(uuid)
        .execute(pool)
        .await
        .unwrap();
    assert_eq!(
        request(
            app,
            Method::PATCH,
            &path,
            Some(admin.clone()),
            Some(json!({"description":"blocked"}))
        )
        .await
        .status(),
        StatusCode::CONFLICT
    );
    sqlx::query("UPDATE backuppolicies SET controlstate='Idle',firstsuccessfulrunat=CURRENT_TIMESTAMP WHERE id=$1").bind(uuid).execute(pool).await.unwrap();
    assert_eq!(
        request(
            app,
            Method::PATCH,
            &path,
            Some(admin.clone()),
            Some(json!({"backupRepositoryId":Uuid::now_v7()}))
        )
        .await
        .status(),
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        request(
            app,
            Method::PATCH,
            &path,
            Some(admin.clone()),
            Some(json!({"source":{"$type":"CitadelSystem"}}))
        )
        .await
        .status(),
        StatusCode::BAD_REQUEST
    );
    sqlx::query("UPDATE backuppolicies SET firstsuccessfulrunat=NULL WHERE id=$1")
        .bind(uuid)
        .execute(pool)
        .await
        .unwrap();
    let response = request(
        app,
        Method::PATCH,
        &path,
        Some(admin.clone()),
        Some(json!({"description":null,"keepLastSuccessful":2})),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let activity: String = sqlx::query_scalar(
        "SELECT info FROM activityevents WHERE resourceid=$1 ORDER BY createdat DESC LIMIT 1",
    )
    .bind(uuid)
    .fetch_one(pool)
    .await
    .unwrap();
    let activity: Value = serde_json::from_str(&activity).unwrap();
    assert_eq!(activity["$type"], "BackupPolicyUpdated");
    assert!(activity["NewPolicy"].get("Webhook").is_none());
}

pub(super) async fn verify_previews(
    pool: &sqlx::PgPool,
    identity: Arc<IdentityService>,
    admin: &ActorPrincipal,
    fixture: &FixtureIds,
) {
    let planner = Arc::new(
        citadel_adapters::backup_source_planner::PostgresBackupSourcePlanner::new(pool.clone()),
    );
    let backups = Arc::new(BackupService::new(
        Arc::new(PostgresBackupStore::new(pool.clone())),
        Arc::new(FakeBackupExecutor),
        planner,
        Duration::minutes(5),
        Arc::new(AllowBackupExecution),
    ));
    let app = backups_http::router(BackupsHttpState {
        identity,
        backups,
        cancellation: CancellationToken::new(),
    });
    let deployment = Uuid::now_v7();
    let deployment_spec: citadel_deployments::DeploymentSpec = serde_json::from_value(json!({
        "image":{"$type":"External","registryId":"00000000-0000-0000-0000-000000000100","imageTag":"nginx"},
        "volumes":["preview-data:/data","/tmp:/host"]
    })).unwrap();
    sqlx::query("INSERT INTO deployments(id,name,platformid,spec,status,createdbyactorid) VALUES($1,$2,$3,$4,'Created',$5)")
        .bind(deployment).bind(format!("preview-{deployment}")).bind(fixture.platform).bind(deployment_spec.to_storage_value().unwrap()).bind(admin.actor_id.value()).execute(pool).await.unwrap();
    let path = format!("/api/v1/deployments/{deployment}/backup-source-preview");
    let before: i64 = sqlx::query_scalar("SELECT count(*) FROM backupruns")
        .fetch_one(pool)
        .await
        .unwrap();
    let response = request(&app, Method::GET, &path, Some(admin.clone()), None).await;
    let status = response.status();
    let preview = response_json(response).await;
    assert_eq!(status, StatusCode::OK, "{preview}");
    assert_eq!(preview["deploymentId"], deployment.to_string());
    assert_eq!(preview["volumes"][0]["name"], "preview-data");
    assert_eq!(preview["volumes"][0]["hasBackupCoverage"], false);
    assert_eq!(preview["volumes"].as_array().unwrap().len(), 1);
    let (stacks, _) = stack_webhooks::service(pool.clone());
    let stack=stacks.create(admin.actor_id,true,serde_json::from_value::<citadel_server::api::stacks::requests::CreateStackInput>(json!({"name":format!("preview-stack-{deployment}"),"platformId":fixture.platform,"stackSource":"WebEditor","spec":{"$type":"WebEditor","composeFile":"services:\n  web:\n    image: nginx\n    volumes:\n      - stack-data:/data\nvolumes:\n  stack-data:\n    external: true"}})).unwrap().try_into().unwrap()).await.unwrap();
    let response = request(
        &app,
        Method::GET,
        &format!("/api/v1/stacks/{}/backup-source-preview", stack.id),
        Some(admin.clone()),
        None,
    )
    .await;
    let status = response.status();
    let stack_preview = response_json(response).await;
    assert_eq!(status, StatusCode::OK, "{stack_preview}");
    assert_eq!(stack_preview["stackId"], stack.id.to_string());
    assert_eq!(stack_preview["volumes"][0]["name"], "stack-data");
    assert_eq!(stack_preview["volumes"][0]["isExternal"], true);
    let swarm = Uuid::now_v7();
    let service = Uuid::now_v7();
    let docker_service = format!("service-{service}");
    sqlx::query("INSERT INTO platforms(id,address,connectortype,cpucount,imagecount,memtotal,name,networkcount,platformdescriptor,status,volumecount) VALUES($1,$2,'Local',0,0,0,$2,0,'{\"$type\":\"DockerSwarm\"}','Online',0)").bind(swarm).bind(swarm.to_string()).execute(pool).await.unwrap();
    sqlx::query("INSERT INTO swarmservices(id,name,dockername,platformid,createdbyactorid,spec,health,desiredspechash,synchronizationstate,updatedat,dockerserviceid) VALUES($1,$2,$2,$3,$4,$5,'Healthy','hash','InSync',CURRENT_TIMESTAMP,$6)")
        .bind(service).bind(format!("preview-service-{service}")).bind(swarm).bind(admin.actor_id.value()).bind(json!({"Image":{"$type":"External","RegistryId":"00000000-0000-0000-0000-000000000100","ImageTag":"redis"},"Mounts":[{"Kind":"Volume","Source":"worker-data","Target":"/data"}]})).bind(&docker_service).execute(pool).await.unwrap();
    sqlx::query("INSERT INTO swarmserviceprojections(platformid,dockerserviceid,name,mode,image,runningtaskcount,desiredtaskcount,updatestate,ports,networkids,secretids,configids,labels,observedat,versionindex,swarmserviceid) VALUES($1,$2,$2,'Replicated','redis',1,1,'Completed','[]','[]','[]','[]','{}',CURRENT_TIMESTAMP,1,$3)").bind(swarm).bind(&docker_service).bind(service).execute(pool).await.unwrap();
    sqlx::query("INSERT INTO swarmtaskprojections(platformid,dockertaskid,desiredstate,dockernodeid,dockerserviceid,image,name,nodehostname,observedat,ports,servicename,state,versionindex) VALUES($1,'task','running','worker-1',$2,'redis','task','worker-host',CURRENT_TIMESTAMP,'[]',$2,'running',1)").bind(swarm).bind(&docker_service).execute(pool).await.unwrap();
    let response = request(
        &app,
        Method::GET,
        &format!("/api/v1/swarmServices/{service}/backup-source-preview"),
        Some(admin.clone()),
        None,
    )
    .await;
    let status = response.status();
    let preview = response_json(response).await;
    assert_eq!(status, StatusCode::OK, "{preview}");
    assert_eq!(preview["volumes"][0]["name"], "worker-data");
    assert_eq!(preview["volumes"][0]["dockerNodeId"], "worker-1");
    sqlx::query("UPDATE swarmtaskprojections SET isstale=true WHERE platformid=$1")
        .bind(swarm)
        .execute(pool)
        .await
        .unwrap();
    assert_eq!(
        request(
            &app,
            Method::GET,
            &format!("/api/v1/swarmServices/{service}/backup-source-preview"),
            Some(admin.clone()),
            None
        )
        .await
        .status(),
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM backupruns")
            .fetch_one(pool)
            .await
            .unwrap(),
        before,
        "previews must not create runs"
    );
    let regular = seed_regular_user(pool).await;
    for (actor, status) in [
        (None, StatusCode::UNAUTHORIZED),
        (Some(regular), StatusCode::FORBIDDEN),
    ] {
        assert_eq!(
            request(&app, Method::GET, &path, actor, None)
                .await
                .status(),
            status
        );
    }
    for resource in ["deployments", "stacks", "swarmServices"] {
        assert_eq!(
            request(
                &app,
                Method::GET,
                &format!(
                    "/api/v1/{resource}/{}/backup-source-preview",
                    Uuid::now_v7()
                ),
                Some(admin.clone()),
                None
            )
            .await
            .status(),
            StatusCode::NOT_FOUND
        );
    }
}

pub(super) async fn verify_streams(
    app: &Router,
    pool: &sqlx::PgPool,
    backups: &Arc<BackupService>,
    admin: &ActorPrincipal,
    policy: &Value,
) {
    let id = policy["id"].as_str().unwrap();
    let url = format!("/api/v1/backupPolicies/{id}/run");
    let regular = seed_regular_user(pool).await;
    assert_eq!(
        request(app, Method::POST, &url, Some(regular), Some(json!({})))
            .await
            .status(),
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        request(app, Method::POST, &url, None, Some(json!({})))
            .await
            .status(),
        StatusCode::UNAUTHORIZED
    );
    let response = request(
        app,
        Method::POST,
        &url,
        Some(admin.clone()),
        Some(json!({"trigger":"Manual"})),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert!(
        response.headers()["content-type"]
            .to_str()
            .unwrap()
            .contains("application/json")
    );
    let policy_uuid = Uuid::parse_str(id).unwrap();
    let run: Uuid = sqlx::query_scalar("SELECT currentrunid FROM backuppolicies WHERE id=$1")
        .bind(policy_uuid)
        .fetch_one(pool)
        .await
        .unwrap();
    // A second observer request cannot accidentally enqueue a concurrent execution.
    assert_eq!(
        request(
            app,
            Method::POST,
            &url,
            Some(admin.clone()),
            Some(json!({}))
        )
        .await
        .status(),
        StatusCode::CONFLICT
    );
    for _ in 0..100 {
        if sqlx::query_scalar::<_, String>("SELECT status FROM backupruns WHERE id=$1")
            .bind(run)
            .fetch_one(pool)
            .await
            .unwrap()
            == "Succeeded"
        {
            break;
        }
        backups
            .process_backup(&CancellationToken::new())
            .await
            .unwrap();
    }
    let values = tokio::time::timeout(std::time::Duration::from_secs(3), response_json(response))
        .await
        .unwrap();
    let items = values.as_array().unwrap();
    assert_eq!(items.first().unwrap()["status"], "Queued");
    assert_eq!(items.last().unwrap()["status"], "Succeeded");
    assert_eq!(items.last().unwrap()["runId"], run.to_string());
    let run_count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM backupruns WHERE backuppolicyid=$1 AND id=$2")
            .bind(policy_uuid)
            .bind(run)
            .fetch_one(pool)
            .await
            .unwrap();
    assert_eq!(run_count, 1);
    let target = policy["source"]["platformId"].clone();
    let response=request(app,Method::POST,&format!("/api/v1/backupRuns/{run}/restoreVolume/run"),Some(admin.clone()),Some(json!({"targetPlatformId":target,"targetVolumeName":format!("restored-{run}"),"overwriteExisting":false}))).await;
    assert_eq!(response.status(), StatusCode::OK);
    let restore: Uuid = sqlx::query_scalar(
        "SELECT id FROM backuprestoreruns WHERE backuprunid=$1 ORDER BY queuedat DESC LIMIT 1",
    )
    .bind(run)
    .fetch_one(pool)
    .await
    .unwrap();
    for _ in 0..100 {
        if sqlx::query_scalar::<_, String>("SELECT status FROM backuprestoreruns WHERE id=$1")
            .bind(restore)
            .fetch_one(pool)
            .await
            .unwrap()
            == "Succeeded"
        {
            break;
        }
        backups
            .process_restore(&CancellationToken::new())
            .await
            .unwrap();
    }
    let values = tokio::time::timeout(std::time::Duration::from_secs(3), response_json(response))
        .await
        .unwrap();
    let items = values.as_array().unwrap();
    assert_eq!(items.last().unwrap()["status"], "Succeeded");
    assert_eq!(items.last().unwrap()["restoreRunId"], restore.to_string());
    assert!(items.last().unwrap().get("runId").is_none());

    // Closing a progress sheet abandons its subscription, not the durable job.
    let response = request(
        app,
        Method::POST,
        &url,
        Some(admin.clone()),
        Some(json!({})),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let detached: Uuid = sqlx::query_scalar("SELECT currentrunid FROM backuppolicies WHERE id=$1")
        .bind(policy_uuid)
        .fetch_one(pool)
        .await
        .unwrap();
    drop(response);
    for _ in 0..100 {
        if backups.store().get_run(detached).await.unwrap().status == "Succeeded" {
            break;
        }
        backups
            .process_backup(&CancellationToken::new())
            .await
            .unwrap();
    }
    assert_eq!(
        backups.store().get_run(detached).await.unwrap().status,
        "Succeeded"
    );

    // Explicit cancellation remains available and is visible on the existing stream.
    let response = request(
        app,
        Method::POST,
        &url,
        Some(admin.clone()),
        Some(json!({})),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let cancelled: Uuid = sqlx::query_scalar("SELECT currentrunid FROM backuppolicies WHERE id=$1")
        .bind(policy_uuid)
        .fetch_one(pool)
        .await
        .unwrap();
    let mut cancellation_progress = backups.subscribe_progress();
    let cancel = request(
        app,
        Method::POST,
        &format!("/api/v1/backupRuns/{cancelled}/cancel"),
        Some(admin.clone()),
        None,
    )
    .await;
    assert!(cancel.status().is_success());
    let notice = tokio::time::timeout(
        std::time::Duration::from_secs(1),
        cancellation_progress.recv(),
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(notice.run_id, cancelled);
    assert_eq!(notice.status.as_deref(), Some("Cancelled"));
    let values = tokio::time::timeout(std::time::Duration::from_secs(3), response_json(response))
        .await
        .unwrap();
    assert_eq!(
        values.as_array().unwrap().last().unwrap()["status"],
        "Cancelled"
    );
}
