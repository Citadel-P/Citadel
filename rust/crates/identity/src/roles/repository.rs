use super::*;

pub trait RoleReader: Send + Sync {
    fn list(&self) -> BoxFuture<'_, Result<Vec<RoleDetails>, IdentityError>>;

    fn get(&self, id: Uuid) -> BoxFuture<'_, Result<Option<RoleDetails>, IdentityError>>;
}

pub trait RoleRepository: Send + Sync {
    fn create<'a>(
        &'a self,
        role: &'a NewRoleMutation,
    ) -> BoxFuture<'a, Result<RoleDetails, IdentityError>>;

    fn patch_permissions<'a>(
        &'a self,
        id: Uuid,
        permissions: Option<&'a [RolePermissionDetails]>,
        changed_by_actor_id: ActorId,
        changed_at: DateTime<Utc>,
        custom_access_control_enabled: bool,
    ) -> BoxFuture<'a, Result<RoleDetails, IdentityError>>;

    fn rename<'a>(
        &'a self,
        id: Uuid,
        name: &'a str,
        changed_by_actor_id: ActorId,
        changed_at: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<RoleDetails, IdentityError>>;

    fn delete<'a>(
        &'a self,
        ids: &'a [Uuid],
        changed_by_actor_id: ActorId,
        changed_at: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<(), IdentityError>>;
}
