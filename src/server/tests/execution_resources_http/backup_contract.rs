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
    let visible = volume_coverage(pool, admin.actor_id, true, platform, &volumes, true)
        .await
        .unwrap();
    let data = visible.iter().find(|v| v.docker_node_id.is_none()).unwrap();
    assert!(data.policy_count > 0);
    assert_eq!(data.status, citadel_backups::BackupCoverageStatus::Warning); // No successful backup yet.
    assert_eq!(
        visible
            .iter()
            .find(|v| v.docker_node_id.is_some())
            .unwrap()
            .status,
        citadel_backups::BackupCoverageStatus::Unprotected
    );
    assert!(
        volume_coverage(pool, reader.actor_id, false, platform, &volumes, true)
            .await
            .unwrap()
            .iter()
            .all(|v| v.policy_count == 0)
    );

    let run = Uuid::now_v7();
    sqlx::query("INSERT INTO backupruns(id,backuppolicyid,backuprepositoryid,policynamesnapshot,repositorytypesnapshot,snapshotavailability,sourcesnapshot,status,trigger,triggeredbyactorid,completedat) SELECT $1,id,backuprepositoryid,name,'FileSystem','Available',source,'Succeeded','Manual',createdbyactorid,CURRENT_TIMESTAMP FROM backuppolicies WHERE id=$2")
        .bind(run).bind(id).execute(pool).await.unwrap();
    sqlx::query("INSERT INTO backuprunitems(id,backuprunid,platformid,volumename,status,resticsnapshotid,completedat) VALUES($1,$2,$3,'data','Succeeded','snapshot',CURRENT_TIMESTAMP)")
        .bind(Uuid::now_v7()).bind(run).bind(platform).execute(pool).await.unwrap();
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
    let updated = store
        .update_policy_description(admin.actor_id, id, Some("Updated metadata"))
        .await
        .unwrap();
    assert_eq!(
        updated.tags.iter().map(|v| v.id).collect::<Vec<_>>(),
        detail.tags.iter().map(|v| v.id).collect::<Vec<_>>()
    );
    assert_eq!(updated.latest_run.as_ref().unwrap().id, run);
    assert_eq!(updated.audit, detail.audit);
    let renamed = store
        .rename_policy(
            admin.actor_id,
            &citadel_backups::policies::metadata::RenameBackupPolicyInput {
                id,
                name: detail.name.clone(),
            },
        )
        .await
        .unwrap();
    assert_eq!(renamed.latest_run.as_ref().unwrap().id, run);
    assert!(renamed.tags.iter().any(|v| v.id == tag));
    let covered = volume_coverage(pool, admin.actor_id, true, platform, &volumes, true)
        .await
        .unwrap();
    let covered = covered.iter().find(|v| v.docker_node_id.is_none()).unwrap();
    assert_eq!(
        covered.status,
        citadel_backups::BackupCoverageStatus::Warning
    );
    assert!(covered.last_successful_run_at.is_some());
    assert_eq!(covered.last_run_id, Some(run));
    sqlx::query("UPDATE backuppolicies SET cron='0 0 * * *' WHERE id=$1")
        .bind(id)
        .execute(pool)
        .await
        .unwrap();
    // Another visible policy for this volume remains manual-only and has no run.
    // Its configuration warning must not be hidden by this scheduled success.
    assert_eq!(
        volume_coverage(pool, admin.actor_id, true, platform, &volumes, true)
            .await
            .unwrap()
            .iter()
            .find(|v| v.docker_node_id.is_none())
            .unwrap()
            .status,
        citadel_backups::BackupCoverageStatus::Warning
    );
    sqlx::query("UPDATE backuppolicies SET cron=NULL WHERE id=$1")
        .bind(id)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("UPDATE backupruns SET status='Failed' WHERE id=$1")
        .bind(run)
        .execute(pool)
        .await
        .unwrap();
    assert_eq!(
        volume_coverage(pool, admin.actor_id, true, platform, &volumes, true)
            .await
            .unwrap()
            .iter()
            .find(|v| v.docker_node_id.is_none())
            .unwrap()
            .status,
        citadel_backups::BackupCoverageStatus::Failed
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
    verify_override_names(pool, &reader, id, policy["name"].as_str().unwrap()).await;
}

async fn verify_override_names(
    pool: &sqlx::PgPool,
    user: &ActorPrincipal,
    policy_id: Uuid,
    policy_name: &str,
) {
    use citadel_adapters::persistence::postgres::identity::teams::repository::PostgresTeamRepository;
    use citadel_adapters::persistence::postgres::identity::users::repository::PostgresUserRepository;
    use citadel_identity::{TeamReader, UserReader};

    let team_id = Uuid::now_v7();
    let team_actor = Uuid::now_v7();
    sqlx::query("INSERT INTO actors(id, type) VALUES ($1, 'Team')")
        .bind(team_actor)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO teams(id, actorid, name) VALUES ($1, $2, $3)")
        .bind(team_id)
        .bind(team_actor)
        .bind(format!("backup-access-{team_id}"))
        .execute(pool)
        .await
        .unwrap();
    for actor in [user.actor_id.value(), team_actor] {
        sqlx::query("INSERT INTO resourceaccesses(id, actorid, permissionlevel, resourceid, resourcetype, specificpermissions) VALUES ($1, $2, 1, $3, 16, 0)")
            .bind(Uuid::now_v7()).bind(actor).bind(policy_id).execute(pool).await.unwrap();
    }
    let user_view = PostgresUserRepository::new(pool.clone())
        .get(user.subject_id)
        .await
        .unwrap()
        .unwrap();
    let team_view = PostgresTeamRepository::new(pool.clone())
        .get(team_id)
        .await
        .unwrap()
        .unwrap();
    // Fresh reads must hydrate the name, without relying on the grant dialog's local state.
    for accesses in [user_view.resource_accesses, team_view.resource_accesses] {
        let accesses = accesses.unwrap();
        let policy = accesses
            .iter()
            .find(|access| access.resource_id == policy_id)
            .unwrap();
        assert_eq!(policy.resource_name.as_deref(), Some(policy_name));
    }
}
