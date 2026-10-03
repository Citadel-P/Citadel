use super::*;

pub(super) async fn ensure_access(
    tx: &mut Transaction<'_, Postgres>,
    actor: ActorId,
    administrator: bool,
    id: Uuid,
    requirement: PermissionRequirement,
) -> Result<(), StackError> {
    if administrator {
        return Ok(());
    }
    let row=sqlx::query(r#"WITH actor_scope AS (SELECT id actorid FROM actors WHERE id=$1 AND isenabled UNION SELECT t.actorid FROM actorteammemberships m JOIN teams t ON t.id=m.teamid JOIN actors a ON a.id=t.actorid AND a.isenabled WHERE m.memberactorid=$1), effective AS (SELECT COALESCE(MAX(p.permissionlevel),0)::integer level_mask,COALESCE(bit_or(p.specificpermissions),0)::integer specific_mask FROM actor_scope s JOIN actorroles ar ON ar.actorid=s.actorid JOIN permissions p ON p.roleid=ar.roleid WHERE p.resourcetype=2 UNION ALL SELECT COALESCE(MAX(ra.permissionlevel),0)::integer,COALESCE(bit_or(ra.specificpermissions),0)::integer FROM actor_scope s JOIN resourceaccesses ra ON ra.actorid=s.actorid WHERE ra.resourcetype=2 AND ra.resourceid=$2) SELECT COALESCE(MAX(level_mask),0)::integer level_mask,COALESCE(bit_or(specific_mask),0)::integer specific_mask FROM effective"#).bind(actor.value()).bind(id).fetch_one(&mut **tx).await.map_err(storage)?;
    let actual: i32 = row.try_get("level_mask").map_err(storage)?;
    let specs: i32 = row.try_get("specific_mask").map_err(storage)?;
    if !decode_permission(false, actual, specs)?.allows(requirement) {
        return Err(StackError::Forbidden);
    }
    Ok(())
}

pub(super) async fn ensure_platform(
    tx: &mut Transaction<'_, Postgres>,
    actor: ActorId,
    administrator: bool,
    id: Uuid,
) -> Result<(), StackError> {
    let row = sqlx::query("SELECT platformdescriptor FROM platforms WHERE id=$1")
        .bind(id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(storage)?
        .ok_or(StackError::NotFound)?;
    if !administrator {
        let allowed:bool=sqlx::query_scalar(r#"WITH actor_scope AS (
            SELECT id actorid FROM actors WHERE id=$1 AND isenabled
            UNION
            SELECT team.actorid FROM actorteammemberships membership
            JOIN teams team ON team.id=membership.teamid
            JOIN actors team_actor ON team_actor.id=team.actorid AND team_actor.isenabled
            WHERE membership.memberactorid=$1)
            SELECT EXISTS(
                SELECT 1 FROM actor_scope scope
                JOIN actorroles assignment ON assignment.actorid=scope.actorid
                JOIN permissions permission ON permission.roleid=assignment.roleid
                WHERE permission.resourcetype=0 AND permission.permissionlevel>=1
                UNION ALL
                SELECT 1 FROM actor_scope scope
                JOIN resourceaccesses access ON access.actorid=scope.actorid
                WHERE access.resourcetype=0 AND access.resourceid=$2 AND access.permissionlevel>=1)"#)
            .bind(actor.value()).bind(id).fetch_one(&mut **tx).await.map_err(storage)?;
        if !allowed {
            return Err(StackError::NotFound);
        }
    }
    let descriptor: Value = row.try_get("platformdescriptor").map_err(storage)?;
    let kind = descriptor
        .get("$type")
        .and_then(Value::as_str)
        .unwrap_or("Docker");
    if crate::persistence::postgres::platforms::classification::platform_kind(kind).is_err() {
        return Err(StackError::Validation(
            "Stacks require a Docker or Docker Swarm Platform.".to_owned(),
        ));
    }
    Ok(())
}
