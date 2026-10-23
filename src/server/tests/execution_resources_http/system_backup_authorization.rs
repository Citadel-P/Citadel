use super::*;
use citadel_backups::BackupPersistence;

pub(super) async fn verify(
    app: &Router,
    pool: &sqlx::PgPool,
    identity: Arc<IdentityService>,
    administrator: &ActorPrincipal,
    repository: Uuid,
) {
    let operator = seed_regular_user(pool).await;
    let role = Uuid::now_v7();
    sqlx::query("INSERT INTO roles(id,name,roletype) VALUES($1,$2,'Custom')")
        .bind(role)
        .bind(format!("backup-only-{role}"))
        .execute(pool)
        .await
        .unwrap();
    for resource in [ResourceType::BackupPolicy, ResourceType::BackupRepository] {
        sqlx::query("INSERT INTO permissions(id,permissionlevel,resourcetype,roleid,specificpermissions) VALUES($1,4,$2,$3,0)").bind(Uuid::now_v7()).bind(resource as i32).bind(role).execute(pool).await.unwrap();
    }
    sqlx::query("INSERT INTO actorroles(actorid,roleid) VALUES($1,$2)")
        .bind(operator.actor_id.value())
        .bind(role)
        .execute(pool)
        .await
        .unwrap();

    // The same backup-only operator can still configure an explicitly authorized workload source.
    let platform: Uuid = sqlx::query_scalar("SELECT id FROM platforms ORDER BY id LIMIT 1")
        .fetch_one(pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO resourceaccesses(id,actorid,resourcetype,resourceid,permissionlevel,specificpermissions) VALUES($1,$2,$3,$4,1,0)")
        .bind(Uuid::now_v7()).bind(operator.actor_id.value()).bind(ResourceType::Platform as i32).bind(platform).execute(pool).await.unwrap();
    let workload = request(app, Method::POST, "/api/v1/backupPolicies", Some(operator.clone()), Some(json!({
        "name":format!("workload-boundary-{}",Uuid::now_v7()), "source":{"$type":"DockerVolume","platformId":platform,"volumeName":"disposable-fixture"},
        "backupRepositoryId":repository,"enabled":true,"alertOnFailure":false,"runAsActorId":operator.actor_id.value()
    }))).await;
    let status = workload.status();
    let body = response_json(workload).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let input = json!({"name":format!("system-boundary-{}",Uuid::now_v7()),"source":{"$type":"CitadelSystem"},"backupRepositoryId":repository,"enabled":true,"alertOnFailure":false,"runAsActorId":operator.actor_id.value()});
    assert_eq!(
        request(
            app,
            Method::POST,
            "/api/v1/backupPolicies",
            Some(operator.clone()),
            Some(input.clone())
        )
        .await
        .status(),
        StatusCode::FORBIDDEN
    );
    let policy = response_json(
        request(
            app,
            Method::POST,
            "/api/v1/backupPolicies",
            Some(administrator.clone()),
            Some(input),
        )
        .await,
    )
    .await;
    let id = Uuid::parse_str(policy["id"].as_str().unwrap()).unwrap();
    assert_eq!(
        request(
            app,
            Method::PATCH,
            &format!("/api/v1/backupPolicies/{id}"),
            Some(operator.clone()),
            Some(json!({"source":{"$type":"CitadelSystem"}}))
        )
        .await
        .status(),
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        request(
            app,
            Method::POST,
            &format!("/api/v1/backupPolicies/{id}/runs"),
            Some(operator.clone()),
            Some(json!({}))
        )
        .await
        .status(),
        StatusCode::FORBIDDEN
    );
    let store = PostgresBackupPersistence::new(pool.clone());
    let checked = store.get_policy(id).await.unwrap();
    sqlx::query("UPDATE backuppolicies SET rowversion=rowversion+1 WHERE id=$1")
        .bind(id)
        .execute(pool)
        .await
        .unwrap();
    assert!(matches!(
        store
            .enqueue_backup(
                administrator.actor_id,
                id,
                "Manual",
                Some(checked.row_version)
            )
            .await,
        Err(citadel_backups::BackupError::Conflict(_))
    ));

    store
        .enqueue_backup(administrator.actor_id, id, "Manual", None)
        .await
        .unwrap();
    let mut claim = store
        .claim_backup(chrono::Utc::now() - Duration::minutes(5))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(claim.policy.id, id);
    let authorizer = citadel_adapters::security::identity::backup_authorization::IdentityBackupRunAuthorizer::new(identity);
    assert!(authorizer.authorize_backup(&claim).await.is_err());
    claim.policy.run_as_actor_id = administrator.actor_id.value();
    authorizer.authorize_backup(&claim).await.unwrap();
    // Worker authorization uses current persisted roles, not a principal captured at queue time.
    let formerly_admin = seed_administrator(pool).await;
    sqlx::query("INSERT INTO actorroles(actorid,roleid) VALUES($1,$2)")
        .bind(formerly_admin.actor_id.value())
        .bind(role)
        .execute(pool)
        .await
        .unwrap();
    claim.policy.run_as_actor_id = formerly_admin.actor_id.value();
    authorizer.authorize_backup(&claim).await.unwrap();
    sqlx::query("DELETE FROM actorroles WHERE actorid=$1 AND roleid=$2")
        .bind(formerly_admin.actor_id.value())
        .bind(ADMIN_ROLE_ID)
        .execute(pool)
        .await
        .unwrap();
    assert!(authorizer.authorize_backup(&claim).await.is_err());
}
