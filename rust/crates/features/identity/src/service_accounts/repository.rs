use super::*;

pub trait ServiceAccountRepository: Send + Sync {
    fn list<'a>(
        &'a self,
        actor_id: ActorId,
        is_administrator: bool,
        include_archived: bool,
        name: Option<&'a str>,
        limit: i64,
        offset: i64,
    ) -> BoxFuture<'a, Result<StoredPage<ServiceAccountDetails>, IdentityError>>;

    fn get(&self, id: Uuid) -> BoxFuture<'_, Result<Option<ServiceAccountDetails>, IdentityError>>;
    fn usages(
        &self,
        actor_id: ActorId,
    ) -> BoxFuture<'_, Result<Vec<RunAsActorUsageDetails>, IdentityError>>;

    fn create<'a>(
        &'a self,
        account: &'a NewServiceAccount,
    ) -> BoxFuture<'a, Result<ServiceAccountDetails, IdentityError>>;

    fn update<'a>(
        &'a self,
        id: Uuid,
        description: &'a PatchField<String>,
        is_enabled: Option<bool>,
        actor_id: ActorId,
        updated_at: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<ServiceAccountDetails, IdentityError>>;

    fn rename<'a>(
        &'a self,
        id: Uuid,
        name: &'a str,
        actor_id: ActorId,
        updated_at: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<ServiceAccountDetails, IdentityError>>;

    fn archive<'a>(
        &'a self,
        ids: &'a [Uuid],
        actor_id: ActorId,
        archived_at: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<(), IdentityError>>;

    fn add_role(
        &self,
        account_id: Uuid,
        role_id: Uuid,
        actor_id: ActorId,
        changed_at: DateTime<Utc>,
    ) -> BoxFuture<'_, Result<ServiceAccountDetails, IdentityError>>;

    fn remove_role(
        &self,
        account_id: Uuid,
        role_id: Uuid,
        actor_id: ActorId,
        changed_at: DateTime<Utc>,
    ) -> BoxFuture<'_, Result<ServiceAccountDetails, IdentityError>>;

    fn add_resource_access<'a>(
        &'a self,
        account_id: Uuid,
        access: &'a ServiceAccountResourceAccess,
        actor_id: ActorId,
        changed_at: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<ServiceAccountDetails, IdentityError>>;

    fn remove_resource_access(
        &self,
        account_id: Uuid,
        resource_access_id: Uuid,
        actor_id: ActorId,
        changed_at: DateTime<Utc>,
    ) -> BoxFuture<'_, Result<ServiceAccountDetails, IdentityError>>;

    fn list_tokens(
        &self,
        account_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> BoxFuture<'_, Result<StoredPage<ServiceAccountTokenDetails>, IdentityError>>;

    fn create_token<'a>(
        &'a self,
        token: &'a NewServiceAccountToken,
        maximum_active: i64,
    ) -> BoxFuture<'a, Result<ServiceAccountTokenDetails, IdentityError>>;

    fn revoke_token(
        &self,
        account_id: Uuid,
        token_id: Uuid,
        actor_id: ActorId,
        revoked_at: DateTime<Utc>,
    ) -> BoxFuture<'_, Result<(), IdentityError>>;
}
