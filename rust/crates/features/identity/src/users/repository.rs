use super::*;

pub trait UserReader: Send + Sync {
    fn list<'a>(
        &'a self,
        name: Option<&'a str>,
        limit: i64,
        offset: i64,
    ) -> BoxFuture<'a, Result<StoredPage<UserDetails>, IdentityError>>;

    fn get(&self, id: Uuid) -> BoxFuture<'_, Result<Option<UserDetails>, IdentityError>>;

    fn search<'a>(
        &'a self,
        query: &'a str,
        limit: i64,
    ) -> BoxFuture<'a, Result<Vec<UserSearchItemDetails>, IdentityError>>;
}

pub trait UserRepository: Send + Sync {
    fn password_context(
        &self,
        id: Uuid,
    ) -> BoxFuture<'_, Result<Option<UserPasswordContext>, IdentityError>>;

    fn create<'a>(
        &'a self,
        user: &'a NewUserMutation,
        custom_access_control_enabled: bool,
    ) -> BoxFuture<'a, Result<UserDetails, IdentityError>>;

    fn patch<'a>(
        &'a self,
        id: Uuid,
        patch: &'a UserPatchMutation,
        changed_by_actor_id: ActorId,
        changed_at: DateTime<Utc>,
        custom_access_control_enabled: bool,
    ) -> BoxFuture<'a, Result<UserDetails, IdentityError>>;

    fn rename<'a>(
        &'a self,
        id: Uuid,
        name: &'a str,
        changed_by_actor_id: ActorId,
        changed_at: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<UserDetails, IdentityError>>;

    fn add_role(
        &self,
        id: Uuid,
        role_id: Uuid,
        changed_by_actor_id: ActorId,
        changed_at: DateTime<Utc>,
        custom_access_control_enabled: bool,
    ) -> BoxFuture<'_, Result<UserDetails, IdentityError>>;

    fn remove_role(
        &self,
        id: Uuid,
        role_id: Uuid,
        changed_by_actor_id: ActorId,
        changed_at: DateTime<Utc>,
    ) -> BoxFuture<'_, Result<UserDetails, IdentityError>>;

    fn add_resource_access<'a>(
        &'a self,
        id: Uuid,
        access: &'a UserResourceAccessInput,
        changed_by_actor_id: ActorId,
        changed_at: DateTime<Utc>,
        custom_access_control_enabled: bool,
    ) -> BoxFuture<'a, Result<UserDetails, IdentityError>>;

    fn remove_resource_access<'a>(
        &'a self,
        id: Uuid,
        access: &'a UserResourceAccessInput,
        changed_by_actor_id: ActorId,
        changed_at: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<UserDetails, IdentityError>>;

    fn delete<'a>(
        &'a self,
        ids: &'a [Uuid],
        changed_by_actor_id: ActorId,
        changed_at: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<(), IdentityError>>;
}
