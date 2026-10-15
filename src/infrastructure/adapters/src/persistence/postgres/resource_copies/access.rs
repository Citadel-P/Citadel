use super::*;
pub(super) struct Access {
    pub actor: ActorId,
    pub administrator: bool,
    pub fence: ReadFence,
}
impl PostgresCopyRepository {
    pub(super) async fn access(&self, actor: ActorId) -> Result<Access, CopyError> {
        let fence = self.authorization.read_fence().await;
        let principal = PostgresIdentityStore::new(self.pool.clone())
            .load_run_as_principal(actor)
            .await
            .map_err(storage_error)?
            .ok_or(CopyError::NotFound)?;
        Ok(Access {
            actor,
            administrator: principal.is_administrator(),
            fence,
        })
    }
    pub(super) async fn require(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        access: &Access,
        kind: ResourceType,
        id: Uuid,
        level: PermissionLevel,
        specific: Option<SpecificPermission>,
    ) -> Result<(), CopyError> {
        if access.administrator {
            return Ok(());
        }
        let permissions = self
            .authorization
            .resources_in_transaction(tx, access.actor, kind, &[id], &access.fence)
            .await
            .map_err(storage_error)?;
        if permissions.get(&id).copied().flatten().is_some_and(|p| {
            p.allows(PermissionRequirement {
                resource_type: kind,
                level,
                specific,
            })
        }) {
            Ok(())
        } else {
            Err(CopyError::NotFound)
        }
    }
    pub(super) async fn authorize_source(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        access: &Access,
        kind: CopyKind,
        id: Uuid,
    ) -> Result<(), CopyError> {
        self.require(
            tx,
            access,
            kind.resource_type(),
            id,
            PermissionLevel::Read,
            None,
        )
        .await?;
        self.require(
            tx,
            access,
            kind.resource_type(),
            Uuid::nil(),
            PermissionLevel::Write,
            None,
        )
        .await?;
        let bindings: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM resourcebindings WHERE scope=$1 AND resourceid=$2)",
        )
        .bind(kind.as_str())
        .bind(id)
        .fetch_one(&mut **tx)
        .await
        .map_err(storage_error)?;
        if bindings {
            self.require(
                tx,
                access,
                kind.resource_type(),
                id,
                PermissionLevel::Read,
                Some(SpecificPermission::ResourceBindings),
            )
            .await?;
            self.require(
                tx,
                access,
                kind.resource_type(),
                Uuid::nil(),
                PermissionLevel::Write,
                Some(SpecificPermission::ResourceBindings),
            )
            .await?;
        }
        Ok(())
    }
}
