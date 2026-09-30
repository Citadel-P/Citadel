use super::*;
use citadel_backups::BackupPersistence;

pub(super) async fn verify(pool: &sqlx::PgPool, admin: &ActorPrincipal, policy: &Value) {
    let id = Uuid::parse_str(policy["id"].as_str().unwrap()).unwrap();
    let platform = Uuid::parse_str(policy["source"]["platformId"].as_str().unwrap()).unwrap();
    let reader = seed_regular_user(pool).await;
    let store = PostgresBackupPersistence::new(pool.clone());
    let volumes = vec![
        ("data".into(), None),
        ("data".into(), Some("other-node".into())),
    ];
    use citadel_adapters::persistence::postgres::backups::coverage::volume_coverage;
    let visible = volume_coverage(pool, admin.actor_id, true, platform, &volumes)
        .await
        .unwrap();
    let data = visible.iter().find(|v| v.docker_node_id.is_none()).unwrap();
    assert!(data.policy_count > 0);
    assert_eq!(data.status, "Warning"); // No successful backup yet.
    assert_eq!(
        visible
            .iter()
            .find(|v| v.docker_node_id.is_some())
            .unwrap()
            .status,
        "Unprotected"
    );
    assert!(
        volume_coverage(pool, reader.actor_id, false, platform, &volumes)
            .await
            .unwrap()
            .iter()
            .all(|v| v.policy_count == 0)
    );

    let run = Uuid::now_v7();
    sqlx::query("INSERT INTO backupruns(id,backuppolicyid,backuprepositoryid,policynamesnapshot,repositorytypesnapshot,snapshotavailability,sourcesnapshot,status,trigger,triggeredbyactorid,completedat) SELECT $1,id,backuprepositoryid,name,'FileSystem','Available',source,'Succeeded','Manual',createdbyactorid,CURRENT_TIMESTAMP FROM backuppolicies WHERE id=$2")
        .bind(run).bind(id).execute(pool).await.unwrap();
    let tag = Uuid::now_v7();
    sqlx::query("INSERT INTO tags(id,name,normalizedname,color,createdbyactorid) VALUES($1,$2,$2,'#112233',$3)")
        .bind(tag).bind(format!("contract-{tag}")).bind(admin.actor_id.value()).execute(pool).await.unwrap();
    sqlx::query("INSERT INTO resourcetags(resourceid,resourcetype,tagid,createdbyactorid) VALUES($1,'BackupPolicy',$2,$3)")
        .bind(id).bind(tag).bind(admin.actor_id.value()).execute(pool).await.unwrap();
    let detail = store.get_policy(id).await.unwrap();
    assert!(detail.tags.iter().any(|v| v.id == tag));
    assert_eq!(detail.latest_run.as_ref().unwrap().id, run);
    let list = store.list_policies(admin.actor_id, true).await.unwrap();
    let listed = list.iter().find(|v| v.id == id).unwrap();
    assert_eq!(listed.latest_run.as_ref().unwrap().id, run);
    assert!(listed.tags.iter().any(|v| v.id == tag));
    let covered = volume_coverage(pool, admin.actor_id, true, platform, &volumes)
        .await
        .unwrap();
    let covered = covered.iter().find(|v| v.docker_node_id.is_none()).unwrap();
    assert_eq!(covered.status, "Protected");
    assert_eq!(covered.last_run_id, Some(run));
    sqlx::query("UPDATE backupruns SET status='Failed' WHERE id=$1")
        .bind(run)
        .execute(pool)
        .await
        .unwrap();
    assert_eq!(
        volume_coverage(pool, admin.actor_id, true, platform, &volumes)
            .await
            .unwrap()
            .iter()
            .find(|v| v.docker_node_id.is_none())
            .unwrap()
            .status,
        "Failed"
    );
    sqlx::query("DELETE FROM backupruns WHERE id=$1")
        .bind(run)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM tags WHERE id=$1")
        .bind(tag)
        .execute(pool)
        .await
        .unwrap();
}
