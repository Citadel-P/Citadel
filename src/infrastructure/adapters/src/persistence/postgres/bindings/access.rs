use super::queries::storage;
use citadel_bindings::{BindingError, SecretUseScope};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

pub async fn grant(
    tx: &mut Transaction<'_, Postgres>,
    secret: Uuid,
    scope: SecretUseScope,
) -> Result<(), BindingError> {
    let (scope, id) = database_parts(scope)?;
    sqlx::query("INSERT INTO secretusegrants(secretid,scope,resourceid) VALUES($1,$2,$3) ON CONFLICT DO NOTHING")
        .bind(secret).bind(scope).bind(id).execute(&mut **tx).await.map_err(storage)?;
    Ok(())
}

/// Authorize use and persist the destination grant in the mutation transaction.
/// Creator grants exist only for secrets made before their new Build/Repository exists.
pub async fn authorize_and_grant(
    tx: &mut Transaction<'_, Postgres>,
    secret: Uuid,
    scope: &str,
    resource: Uuid,
    creator: Option<citadel_primitives::ActorId>,
) -> Result<(), BindingError> {
    let allowed: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM secretusegrants WHERE secretid=$1 AND ((scope='Global' AND resourceid='00000000-0000-0000-0000-000000000000') OR (scope=$2 AND resourceid=$3) OR (scope='Creator' AND resourceid=$4)))")
        .bind(secret).bind(scope).bind(resource).bind(creator.map(|actor| actor.value())).fetch_one(&mut **tx).await.map_err(storage)?;
    if !allowed {
        return Err(BindingError::Validation(
            "Secret is not authorized for this resource.".into(),
        ));
    }
    // Store the resulting resource grant, so execution never relies on creator authority.
    sqlx::query("INSERT INTO secretusegrants(secretid,scope,resourceid) VALUES($1,$2,$3) ON CONFLICT DO NOTHING")
        .bind(secret).bind(scope).bind(resource).execute(&mut **tx).await.map_err(storage)?;
    Ok(())
}

/// The caller has authorized copying bindings from the source resource.
pub async fn copy_grants(
    tx: &mut Transaction<'_, Postgres>,
    scope: &str,
    source: Uuid,
    target: Uuid,
) -> Result<(), BindingError> {
    let secrets: Vec<Uuid> = sqlx::query_scalar("SELECT DISTINCT secretid FROM resourcebindings WHERE scope=$1 AND resourceid=$2 AND secretid IS NOT NULL")
        .bind(scope).bind(source).fetch_all(&mut **tx).await.map_err(storage)?;
    for secret in secrets {
        authorize_and_grant(tx, secret, scope, source, None).await?;
        sqlx::query("INSERT INTO secretusegrants(secretid,scope,resourceid) VALUES($1,$2,$3) ON CONFLICT DO NOTHING")
            .bind(secret).bind(scope).bind(target).execute(&mut **tx).await.map_err(storage)?;
    }
    Ok(())
}

// Persist explicit scope names; Rust debug formatting is not a storage contract.
pub(super) fn database_parts(scope: SecretUseScope) -> Result<(&'static str, Uuid), BindingError> {
    use citadel_primitives::ResourceType;
    Ok(match scope {
        SecretUseScope::Global => ("Global", Uuid::nil()),
        SecretUseScope::Creator(actor) => ("Creator", actor.value()),
        SecretUseScope::Resource {
            resource_type,
            resource_id,
        } => {
            let name = match resource_type {
                ResourceType::Stack => "Stack",
                ResourceType::Deployment => "Deployment",
                ResourceType::SwarmService => "SwarmService",
                ResourceType::Build => "Build",
                ResourceType::BackupRepository => "BackupRepository",
                _ => {
                    return Err(BindingError::Validation(
                        "Resource type cannot own secret grants.".into(),
                    ));
                }
            };
            (name, resource_id)
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use citadel_primitives::{ActorId, ResourceType};

    #[test]
    fn grant_scopes_match_storage_and_reject_unsupported_resources() {
        let id = Uuid::now_v7();
        assert_eq!(
            database_parts(SecretUseScope::Global).unwrap(),
            ("Global", Uuid::nil())
        );
        assert_eq!(
            database_parts(SecretUseScope::Creator(ActorId::new(id))).unwrap(),
            ("Creator", id)
        );
        for (resource_type, name) in [
            (ResourceType::Stack, "Stack"),
            (ResourceType::Deployment, "Deployment"),
            (ResourceType::SwarmService, "SwarmService"),
            (ResourceType::Build, "Build"),
            (ResourceType::BackupRepository, "BackupRepository"),
        ] {
            assert_eq!(
                database_parts(SecretUseScope::Resource {
                    resource_type,
                    resource_id: id
                })
                .unwrap(),
                (name, id)
            );
        }
        assert!(
            database_parts(SecretUseScope::Resource {
                resource_type: ResourceType::Platform,
                resource_id: id
            })
            .is_err()
        );
    }
}
