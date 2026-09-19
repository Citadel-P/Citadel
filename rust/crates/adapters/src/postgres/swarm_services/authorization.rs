use super::*;

pub(super) async fn ensure_platform(
    tx: &mut Transaction<'_, Postgres>,
    actor_id: ActorId,
    administrator: bool,
    platform_id: Uuid,
    online: bool,
) -> Result<(), SwarmServiceError> {
    let row = sqlx::query("SELECT status,platformdescriptor FROM platforms WHERE id=$1")
        .bind(platform_id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(storage)?
        .ok_or(SwarmServiceError::NotFound)?;
    if !administrator
        && !has_access(
            tx,
            actor_id,
            platform_id,
            PermissionRequirement {
                resource_type: ResourceType::Platform,
                level: PermissionLevel::Read,
                specific: None,
            },
        )
        .await?
    {
        return Err(SwarmServiceError::NotFound);
    }
    let descriptor: Value = row.try_get("platformdescriptor").map_err(storage)?;
    if descriptor.get("$type").and_then(Value::as_str) != Some("DockerSwarm") {
        return Err(SwarmServiceError::Validation(
            "Managed Swarm Services require a Docker Swarm platform.".to_owned(),
        ));
    }
    if online
        && (row.try_get::<String, _>("status").map_err(storage)? != "Online"
            || descriptor.get("controlAvailable").and_then(Value::as_bool) == Some(false))
    {
        return Err(SwarmServiceError::Conflict(
            "The Docker Swarm manager is not available.".to_owned(),
        ));
    }
    Ok(())
}
pub(super) async fn ensure_access(
    tx: &mut Transaction<'_, Postgres>,
    actor: ActorId,
    administrator: bool,
    id: Uuid,
    requirement: PermissionRequirement,
) -> Result<(), SwarmServiceError> {
    if administrator || has_access(tx, actor, id, requirement).await? {
        Ok(())
    } else {
        Err(SwarmServiceError::NotFound)
    }
}
pub(super) async fn has_access(
    tx: &mut Transaction<'_, Postgres>,
    actor: ActorId,
    id: Uuid,
    requirement: PermissionRequirement,
) -> Result<bool, SwarmServiceError> {
    let (actual_level,actual_specific)=sqlx::query_as::<_,(i32,i32)>(r#"WITH scope AS(SELECT id actorid FROM actors WHERE id=$1 AND isenabled UNION SELECT team.actorid FROM actorteammemberships m JOIN teams team ON team.id=m.teamid JOIN actors a ON a.id=team.actorid AND a.isenabled WHERE m.memberactorid=$1), grants AS(SELECT p.permissionlevel,p.specificpermissions FROM scope s JOIN actorroles ar ON ar.actorid=s.actorid JOIN permissions p ON p.roleid=ar.roleid WHERE p.resourcetype=$2 UNION ALL SELECT ra.permissionlevel,ra.specificpermissions FROM scope s JOIN resourceaccesses ra ON ra.actorid=s.actorid WHERE ra.resourcetype=$2 AND ra.resourceid=$3) SELECT COALESCE(MAX(permissionlevel),0)::integer,COALESCE(bit_or(specificpermissions),0)::integer FROM grants"#).bind(actor.value()).bind(requirement.resource_type as i32).bind(id).fetch_one(&mut **tx).await.map_err(storage)?;
    Ok(decode_permission(false, actual_level, actual_specific)?.allows(requirement))
}
