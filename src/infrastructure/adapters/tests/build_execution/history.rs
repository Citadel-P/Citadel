use super::*;
use citadel_builds::BuildClaim;
use citadel_primitives::ResourceType;

pub(super) async fn verify_access(pool: &sqlx::PgPool, claims: &[BuildClaim]) {
    let store = PostgresBuildRepository::new(pool.clone());
    let reader = ActorId::new(Uuid::now_v7());
    let team = Uuid::now_v7();
    let team_actor = Uuid::now_v7();
    sqlx::query("INSERT INTO actors(id,type) VALUES($1,'User'),($2,'Team')")
        .bind(reader.value())
        .bind(team_actor)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO teams(id,actorid,name) VALUES($1,$2,$3)")
        .bind(team)
        .bind(team_actor)
        .bind(format!("history-{team}"))
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO actorteammemberships(teamid,memberactorid) VALUES($1,$2)")
        .bind(team)
        .bind(reader.value())
        .execute(pool)
        .await
        .unwrap();

    let runs = |parent, limit| {
        let store = &store;
        async move {
            store
                .list_runs(reader, false, parent, limit)
                .await
                .unwrap()
                .into_iter()
                .map(|run| run.id)
                .collect::<Vec<_>>()
        }
    };
    let grant = |actor: Uuid, project: Uuid| async move {
        sqlx::query("INSERT INTO resourceaccesses(id,actorid,resourcetype,resourceid,permissionlevel,specificpermissions) VALUES($1,$2,$3,$4,1,0)")
            .bind(Uuid::now_v7()).bind(actor).bind(ResourceType::Build as i32).bind(project)
            .execute(pool).await.unwrap();
    };
    assert!(runs(None, 50).await.is_empty());
    grant(team_actor, claims[0].project.id).await;
    assert_eq!(runs(None, 50).await, [claims[0].run.id]);
    assert!(runs(Some(claims[1].project.id), 50).await.is_empty());

    // Direct and inherited grants share one chronological feed, even when they overlap.
    grant(reader.value(), claims[0].project.id).await;
    grant(team_actor, claims[1].project.id).await;
    grant(reader.value(), claims[2].project.id).await;
    let mut expected = claims[..3]
        .iter()
        .map(|c| (c.run.queued_at, c.run.id))
        .collect::<Vec<_>>();
    expected.sort_unstable_by(|a, b| b.cmp(a));
    let ids: Vec<_> = expected.into_iter().map(|(_, id)| id).collect();
    assert_eq!(runs(None, 50).await, ids);
    assert_eq!(runs(None, 2).await, ids[..2]);
    assert_eq!(
        runs(Some(claims[1].project.id), 1).await,
        [claims[1].run.id]
    );

    sqlx::query("UPDATE actors SET isenabled=false WHERE id=$1")
        .bind(reader.value())
        .execute(pool)
        .await
        .unwrap();
    assert!(runs(None, 50).await.is_empty());
    assert!(runs(Some(claims[1].project.id), 1).await.is_empty());
}
