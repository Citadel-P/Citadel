use super::*;

pub trait TeamReader: Send + Sync {
    fn list<'a>(
        &'a self,
        name: Option<&'a str>,
        limit: i64,
        offset: i64,
    ) -> BoxFuture<'a, Result<StoredPage<TeamDetails>, IdentityError>>;

    fn get(&self, id: Uuid) -> BoxFuture<'_, Result<Option<TeamDetails>, IdentityError>>;

    fn search<'a>(
        &'a self,
        query: &'a str,
        limit: i64,
    ) -> BoxFuture<'a, Result<Vec<TeamSearchItemDetails>, IdentityError>>;
}

pub trait TeamRepository: Send + Sync {
    fn create<'a>(
        &'a self,
        team: &'a NewTeamMutation,
        custom_access_control_enabled: bool,
    ) -> BoxFuture<'a, Result<TeamDetails, IdentityError>>;

    fn patch<'a>(
        &'a self,
        id: Uuid,
        patch: &'a TeamPatchMutation,
        changed_by_actor_id: ActorId,
        changed_at: DateTime<Utc>,
        custom_access_control_enabled: bool,
    ) -> BoxFuture<'a, Result<TeamDetails, IdentityError>>;

    fn rename<'a>(
        &'a self,
        id: Uuid,
        name: &'a str,
        changed_by_actor_id: ActorId,
        changed_at: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<TeamDetails, IdentityError>>;

    fn add_role(
        &self,
        id: Uuid,
        role_id: Uuid,
        changed_by_actor_id: ActorId,
        changed_at: DateTime<Utc>,
        custom_access_control_enabled: bool,
    ) -> BoxFuture<'_, Result<TeamDetails, IdentityError>>;

    fn remove_role(
        &self,
        id: Uuid,
        role_id: Uuid,
        changed_by_actor_id: ActorId,
        changed_at: DateTime<Utc>,
    ) -> BoxFuture<'_, Result<TeamDetails, IdentityError>>;

    fn add_member(
        &self,
        id: Uuid,
        member_actor_id: ActorId,
        changed_by_actor_id: ActorId,
        changed_at: DateTime<Utc>,
        custom_access_control_enabled: bool,
    ) -> BoxFuture<'_, Result<TeamDetails, IdentityError>>;

    fn remove_member(
        &self,
        id: Uuid,
        member_actor_id: ActorId,
        changed_by_actor_id: ActorId,
        changed_at: DateTime<Utc>,
    ) -> BoxFuture<'_, Result<TeamDetails, IdentityError>>;

    fn add_resource_access<'a>(
        &'a self,
        id: Uuid,
        access: &'a TeamResourceAccessInput,
        changed_by_actor_id: ActorId,
        changed_at: DateTime<Utc>,
        custom_access_control_enabled: bool,
    ) -> BoxFuture<'a, Result<TeamDetails, IdentityError>>;

    fn remove_resource_access<'a>(
        &'a self,
        id: Uuid,
        access: &'a TeamResourceAccessInput,
        changed_by_actor_id: ActorId,
        changed_at: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<TeamDetails, IdentityError>>;

    fn delete<'a>(
        &'a self,
        ids: &'a [Uuid],
        changed_by_actor_id: ActorId,
        changed_at: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<(), IdentityError>>;
}
