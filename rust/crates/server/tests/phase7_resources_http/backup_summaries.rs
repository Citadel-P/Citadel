use super::*;
use citadel_backups::BackupPersistence;

pub(super) async fn verify(
    app: &Router,
    pool: &sqlx::PgPool,
    admin: &ActorPrincipal,
    policy: &Value,
) {
    let platform = Uuid::parse_str(policy["source"]["platformId"].as_str().unwrap()).unwrap();
    let template = Uuid::parse_str(policy["id"].as_str().unwrap()).unwrap();
    let deployment = Uuid::now_v7();
    let service = Uuid::now_v7();
    let stack = Uuid::now_v7();
    let release = Uuid::now_v7();
    sqlx::query("INSERT INTO deployments(id,name,platformid,createdbyactorid,spec,status) VALUES($1,$2,$3,$4,'{}','Created')")
        .bind(deployment).bind(deployment.to_string()).bind(platform).bind(admin.actor_id.value()).execute(pool).await.unwrap();
    sqlx::query("INSERT INTO swarmservices(id,name,dockername,platformid,createdbyactorid,spec,desiredspechash,health,synchronizationstate,updatedat) VALUES($1,$2,$2,$3,$4,'{}','fixture','Created','Synchronized',CURRENT_TIMESTAMP)")
        .bind(service).bind(service.to_string()).bind(platform).bind(admin.actor_id.value()).execute(pool).await.unwrap();
    sqlx::query("INSERT INTO stacks(id,name,createdbyactorid,driftpolicy,stacksource,stackupdatestate) VALUES($1,$2,$3,'{}','WebEditor','{}')")
        .bind(stack).bind(stack.to_string()).bind(admin.actor_id.value()).execute(pool).await.unwrap();
    sqlx::query("INSERT INTO stackreleases(id,stackid,platformid,createdbyactorid,spec,status,version) VALUES($1,$2,$3,$4,'{}','Created','1')")
        .bind(release).bind(stack).bind(platform).bind(admin.actor_id.value()).execute(pool).await.unwrap();
    sqlx::query("UPDATE stacks SET currentstackreleaseid=$2 WHERE id=$1")
        .bind(stack)
        .bind(release)
        .execute(pool)
        .await
        .unwrap();
    let mut ids = Vec::new();
    for (source, enabled) in [
        (json!({"$type":"Stack","stackId":stack}), true),
        (
            json!({"$type":"Deployment","deploymentId":deployment}),
            false,
        ),
        (
            json!({"$type":"SwarmService","swarmServiceId":service}),
            true,
        ),
    ] {
        let id = Uuid::now_v7();
        sqlx::query("INSERT INTO backuppolicies(id,name,normalizedname,source,backuprepositoryid,enabled,keeplastsuccessful,timeoutseconds,alertonfailure,runasactorid,createdbyactorid) SELECT $1,$2,$2,$3,backuprepositoryid,$4,keeplastsuccessful,timeoutseconds,alertonfailure,runasactorid,createdbyactorid FROM backuppolicies WHERE id=$5")
            .bind(id).bind(id.to_string()).bind(source).bind(enabled).bind(template).execute(pool).await.unwrap();
        ids.push(id);
    }
    sqlx::query("INSERT INTO backupruns(id,backuppolicyid,backuprepositoryid,policynamesnapshot,repositorytypesnapshot,snapshotavailability,sourcesnapshot,status,trigger,triggeredbyactorid,completedat) SELECT $1,id,backuprepositoryid,name,'FileSystem','None',source,'Failed','Manual',createdbyactorid,CURRENT_TIMESTAMP FROM backuppolicies WHERE id=$2")
        .bind(Uuid::now_v7()).bind(ids[1]).execute(pool).await.unwrap();
    let empty = Uuid::now_v7();
    let path = format!(
        "/api/v1/backupPolicies/platform-summaries?platformIds={platform}&platformIds={platform}&platformIds={empty}"
    );
    assert_eq!(
        request(app, Method::GET, &path, None, None).await.status(),
        StatusCode::UNAUTHORIZED
    );
    let reader = seed_regular_user(pool).await;
    assert_eq!(
        request(app, Method::GET, &path, Some(reader.clone()), None)
            .await
            .status(),
        StatusCode::FORBIDDEN
    );
    let response = request(app, Method::GET, &path, Some(admin.clone()), None).await;
    assert_eq!(response.status(), StatusCode::OK);
    let response = response_json(response).await;
    let rows = response["platforms"].as_array().unwrap();
    assert_eq!(rows.len(), 2);
    let summary = rows
        .iter()
        .find(|v| v["platformId"] == platform.to_string())
        .unwrap();
    assert_eq!(summary["policyCount"], 5);
    assert_eq!(summary["enabledPolicyCount"], 4);
    assert_eq!(summary["dockerVolumePolicyCount"], 2);
    for name in [
        "stackPolicyCount",
        "deploymentPolicyCount",
        "swarmServicePolicyCount",
        "attentionPolicyCount",
    ] {
        assert_eq!(summary[name], 1);
    }
    assert_eq!(summary["lastRunStatus"], "Failed");
    assert!(summary["lastRunAt"].is_string());
    assert_eq!(
        rows.iter()
            .find(|v| v["platformId"] == empty.to_string())
            .unwrap()["policyCount"],
        0
    );
    let store = PostgresBackupPersistence::new(pool.clone());
    assert_eq!(
        store
            .platform_summaries(reader.actor_id, false, &[platform])
            .await
            .unwrap()[0]
            .policy_count,
        0
    );
    sqlx::query("INSERT INTO resourceaccesses(id,actorid,resourcetype,resourceid,permissionlevel,specificpermissions) VALUES($1,$2,$3,$4,1,0)")
        .bind(Uuid::now_v7()).bind(reader.actor_id.value()).bind(ResourceType::BackupPolicy as i32).bind(ids[2]).execute(pool).await.unwrap();
    let visible = store
        .platform_summaries(reader.actor_id, false, &[platform])
        .await
        .unwrap();
    assert_eq!(visible[0].policy_count, 1);
    assert_eq!(visible[0].swarm_service_policy_count, 1);
    assert!(visible[0].last_run_status.is_none());
    sqlx::query("UPDATE backuppolicies SET archivedat=CURRENT_TIMESTAMP WHERE id=ANY($1)")
        .bind(&ids)
        .execute(pool)
        .await
        .unwrap();
    assert_eq!(
        store
            .platform_summaries(admin.actor_id, true, &[platform])
            .await
            .unwrap()[0]
            .policy_count,
        2
    );
    for query in [
        "platformIds=invalid".into(),
        std::iter::repeat_n(format!("platformIds={platform}"), 257)
            .collect::<Vec<_>>()
            .join("&"),
    ] {
        assert_eq!(
            request(
                app,
                Method::GET,
                &format!("/api/v1/backupPolicies/platform-summaries?{query}"),
                Some(admin.clone()),
                None
            )
            .await
            .status(),
            StatusCode::BAD_REQUEST
        );
    }
    let response = response_json(
        request(
            app,
            Method::GET,
            "/api/v1/backupPolicies/platform-summaries",
            Some(admin.clone()),
            None,
        )
        .await,
    )
    .await;
    assert_eq!(response, json!({"platforms":[]}));
}
