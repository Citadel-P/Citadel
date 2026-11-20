use super::*;
use citadel_activities::{ActivityAccess, ActivityFilter, ActivityQueryStore};
use citadel_adapters::persistence::postgres::{
    activities::store::PostgresActivityStore,
    identity::service_accounts::repository::PostgresServiceAccountRepository,
    platforms::PostgresPlatformReader,
};
use citadel_identity::{NewServiceAccount, SYSTEM_ACTOR_ID, ServiceAccountRepository};
use citadel_platforms::PlatformReader;

#[tokio::test]
#[ignore = "requires disposable CITADEL_EXECUTION_DATABASE_URL"]
async fn team_membership_requires_enabled_actors_across_catalogs_and_activity() {
    let url = std::env::var("CITADEL_EXECUTION_DATABASE_URL").unwrap();
    MigrationRunner::migrate(&url).await.unwrap();
    let pool = sqlx::PgPool::connect(&url).await.unwrap();
    let team = Uuid::now_v7();
    let team_actor = Uuid::now_v7();
    sqlx::query("INSERT INTO actors(id,type) VALUES($1,'Team')")
        .bind(team_actor)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO teams(id,actorid,name) VALUES($1,$2,$3)")
        .bind(team)
        .bind(team_actor)
        .bind(format!("readers-{team}"))
        .execute(&pool)
        .await
        .unwrap();

    let accounts = PostgresServiceAccountRepository::new(pool.clone());
    let account = accounts
        .create(&NewServiceAccount {
            id: Uuid::now_v7(),
            actor_id: ActorId::new(Uuid::now_v7()),
            name: format!("member-{team}"),
            description: None,
            is_enabled: true,
            created_by_actor_id: ActorId::new(SYSTEM_ACTOR_ID),
            created_at: chrono::Utc::now(),
            team_ids: vec![team],
            role_ids: vec![],
            resource_accesses: vec![],
        })
        .await
        .unwrap();
    let platform = Uuid::now_v7();
    sqlx::query("INSERT INTO platforms(id,address,connectortype,cpucount,imagecount,memtotal,name,networkcount,platformdescriptor,status,volumecount) VALUES($1,'unix:///fixture','Local',1,0,1024,$2,0,'{\"$type\":\"Docker\"}','Online',0)")
        .bind(platform).bind(format!("platform-{platform}")).execute(&pool).await.unwrap();
    for (kind, id) in [
        (ResourceType::Platform, platform),
        (ResourceType::ServiceAccount, account.id),
    ] {
        sqlx::query("INSERT INTO resourceaccesses(id,actorid,resourcetype,resourceid,permissionlevel,specificpermissions) VALUES($1,$2,$3,$4,1,0)")
            .bind(Uuid::now_v7()).bind(team_actor).bind(kind as i32).bind(id).execute(&pool).await.unwrap();
    }
    let platforms = PostgresPlatformReader::new(pool.clone());
    let activities = PostgresActivityStore::new(pool.clone());
    let actor = account.actor_id;
    for (member_enabled, team_enabled) in [(true, true), (false, true), (true, false), (true, true)]
    {
        sqlx::query(
            "UPDATE actors SET isenabled=CASE WHEN id=$1 THEN $3 ELSE $4 END WHERE id IN ($1,$2)",
        )
        .bind(actor.value())
        .bind(team_actor)
        .bind(member_enabled)
        .bind(team_enabled)
        .execute(&pool)
        .await
        .unwrap();
        let visible = member_enabled && team_enabled;
        let page = accounts
            .list(actor, false, false, Some(&account.name), 10, 0)
            .await
            .unwrap();
        assert_eq!(page.items.iter().any(|item| item.id == account.id), visible);
        let list = platforms.list_authorized(actor, false, &[]).await.unwrap();
        assert_eq!(list.iter().any(|item| item.id == platform), visible);
        let page = activities
            .list_authorized(
                &ActivityAccess {
                    actor_id: actor,
                    administrator: false,
                },
                ActivityFilter {
                    resource_id: Some(account.id),
                    ..Default::default()
                }
                .validated()
                .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(page.total_count, if visible { 1 } else { 0 });
    }
}
