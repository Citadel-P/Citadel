use super::*;
use crate::persistence::postgres::identity::users::repository::lock_identity_mutations;
use citadel_identity::{InstallationSettings, MfaPolicy, PatchInstallationSettings};
use citadel_primitives::PatchField;

pub(super) async fn load(pool: &PgPool) -> Result<InstallationSettings, IdentityError> {
    let row = sqlx::query("SELECT initializedat IS NOT NULL AS initialized, updatechecksenabled, mfapolicy FROM instancesetupstates WHERE id = 1")
        .fetch_one(pool).await.map_err(storage)?;
    map(&row)
}

pub(super) async fn patch(
    store: &PostgresIdentityStore,
    principal: &ActorPrincipal,
    patch: &PatchInstallationSettings,
    now: DateTime<Utc>,
) -> Result<InstallationSettings, IdentityError> {
    let _fence = store.authorization.read_fence().await;
    let mut tx = store.pool.begin().await.map_err(storage)?;
    lock_identity_mutations(&mut tx).await?;
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
    .fetch_one(&mut *tx)
    .await
    .map_err(storage)?;
    if !authorized {
        return Err(IdentityError::Forbidden);
    }
    let row = sqlx::query("SELECT initializedat IS NOT NULL AS initialized, updatechecksenabled, mfapolicy FROM instancesetupstates WHERE id = 1 FOR UPDATE")
        .fetch_one(&mut *tx).await.map_err(storage)?;
    let old = map(&row)?;
    if !old.initialized {
        return Err(IdentityError::Conflict(
            "Complete Citadel setup first.".into(),
        ));
    }
    let next = InstallationSettings {
        initialized: true,
        update_checks_enabled: match patch.update_checks_enabled {
            PatchField::Value(value) => value,
            _ => old.update_checks_enabled,
        },
        mfa_policy: match patch.mfa_policy {
            PatchField::Value(value) => value,
            _ => old.mfa_policy,
        },
    };
    if next != old {
        sqlx::query("UPDATE instancesetupstates SET updatechecksenabled=$1, mfapolicy=$2, updatedat=$3 WHERE id=1")
            .bind(next.update_checks_enabled).bind(next.mfa_policy.as_database_str()).bind(now)
            .execute(&mut *tx).await.map_err(storage)?;
        let event = ActivityEvent::new_user_event(
            principal.subject_id,
            principal.name.clone(),
            principal.actor_id,
            ActivityEventInfo::InstallationSettingsUpdated {
                previous_update_checks_enabled: old.update_checks_enabled,
                update_checks_enabled: next.update_checks_enabled,
                previous_mfa_policy: old.mfa_policy.as_database_str().into(),
                mfa_policy: next.mfa_policy.as_database_str().into(),
            },
            now,
        )
        .map_err(invalid_activity)?;
        insert_activity(&mut tx, &event).await?;
    }
    tx.commit().await.map_err(storage)?;
    Ok(next)
}

fn map(row: &sqlx::postgres::PgRow) -> Result<InstallationSettings, IdentityError> {
    let policy: String = row.try_get("mfapolicy").map_err(storage)?;
    Ok(InstallationSettings {
        initialized: row.try_get("initialized").map_err(storage)?,
        update_checks_enabled: row.try_get("updatechecksenabled").map_err(storage)?,
        mfa_policy: MfaPolicy::from_database_str(&policy)
            .ok_or_else(|| IdentityError::Storage("Invalid installation MFA policy.".into()))?,
    })
}
