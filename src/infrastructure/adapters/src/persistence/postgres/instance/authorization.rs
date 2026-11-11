use super::repository::storage;
use crate::persistence::postgres::identity::users::repository::lock_identity_mutations;
use citadel_identity::{ADMIN_ROLE_ID, ActorPrincipal};
use citadel_instance::InstanceError;
use sqlx::{Postgres, Transaction};

pub(super) async fn authorize(
    tx: &mut Transaction<'_, Postgres>,
    principal: &ActorPrincipal,
) -> Result<(), InstanceError> {
    lock_identity_mutations(tx).await.map_err(storage)?;
    // Recheck the current enabled human administrator inside the write transaction.
    let authorized = sqlx::query_scalar::<_, bool>(
        r#"
SELECT EXISTS (
 SELECT 1 FROM users u JOIN actors a ON a.id = u.actorid
 WHERE u.id = $1 AND a.id = $2 AND a.isenabled AND a.type = 'User'
 AND EXISTS (
   SELECT 1 FROM actorroles ar JOIN roles r ON r.id = ar.roleid
   WHERE r.id = $3 AND (ar.actorid = a.id OR EXISTS (
     SELECT 1 FROM actorteammemberships m JOIN teams t ON t.id = m.teamid
     JOIN actors ta ON ta.id = t.actorid
     WHERE m.memberactorid = a.id AND t.actorid = ar.actorid AND ta.isenabled
   ))
 )
)
"#,
    )
    .bind(principal.subject_id)
    .bind(principal.actor_id.value())
    .bind(ADMIN_ROLE_ID)
    .fetch_one(&mut **tx)
    .await
    .map_err(storage)?;
    if !authorized {
        return Err(InstanceError::Forbidden);
    }
    Ok(())
}
