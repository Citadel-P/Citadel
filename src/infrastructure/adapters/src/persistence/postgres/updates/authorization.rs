use super::*;

pub(super) async fn instance(tx: &mut Transaction<'_, Postgres>) -> Result<Uuid, UpdateError> {
    sqlx::query_scalar("SELECT instanceid FROM citadelinstanceidentity WHERE id=1")
        .fetch_one(&mut **tx)
        .await
        .map_err(storage)
}

pub(super) async fn authorize(
    tx: &mut Transaction<'_, Postgres>,
    actor: UpdateActor,
) -> Result<(), UpdateError> {
    lock_identity_mutations(tx).await.map_err(storage)?;
    let authorized = sqlx::query_scalar::<_, bool>(r#"
SELECT EXISTS(SELECT 1 FROM users u JOIN actors a ON a.id=u.actorid
 WHERE u.id=$1 AND a.id=$2 AND a.isenabled AND a.type='User'
 AND EXISTS(SELECT 1 FROM actorroles ar WHERE ar.roleid=$3 AND (ar.actorid=a.id OR EXISTS(
 SELECT 1 FROM actorteammemberships m JOIN teams t ON t.id=m.teamid JOIN actors ta ON ta.id=t.actorid
 WHERE m.memberactorid=a.id AND t.actorid=ar.actorid AND ta.isenabled)))
 AND EXISTS(SELECT 1 FROM instancesetupstates WHERE id=1 AND initializedat IS NOT NULL))
"#).bind(actor.subject_id).bind(actor.actor_id.value()).bind(ADMIN_ROLE_ID).fetch_one(&mut **tx).await.map_err(storage)?;
    if !authorized {
        return Err(UpdateError::Forbidden);
    }
    Ok(())
}
