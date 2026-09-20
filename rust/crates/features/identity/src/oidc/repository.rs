use super::*;

pub trait OidcProtocol: Send + Sync {
    fn discover<'a>(
        &'a self,
        issuer: &'a str,
    ) -> BoxFuture<'a, Result<OidcDiscovery, IdentityError>>;

    #[allow(clippy::too_many_arguments)]
    fn exchange_and_validate<'a>(
        &'a self,
        provider: &'a OidcProvider,
        discovery: &'a OidcDiscovery,
        client_secret: &'a str,
        code: &'a str,
        redirect_uri: &'a str,
        code_verifier: &'a str,
        nonce: &'a str,
    ) -> BoxFuture<'a, Result<OidcIdentity, IdentityError>>;
}

pub trait OidcStore: Send + Sync {
    fn list(&self) -> BoxFuture<'_, Result<Vec<OidcProvider>, IdentityError>>;
    fn list_enabled(&self) -> BoxFuture<'_, Result<Vec<OidcProvider>, IdentityError>>;
    fn get(&self, id: Uuid) -> BoxFuture<'_, Result<Option<OidcProvider>, IdentityError>>;
    fn role_exists(&self, id: Uuid) -> BoxFuture<'_, Result<bool, IdentityError>>;
    fn create<'a>(
        &'a self,
        provider: &'a OidcProvider,
        actor_id: ActorId,
        now: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<OidcProvider, IdentityError>>;
    fn update<'a>(
        &'a self,
        provider: &'a OidcProvider,
        expected_updated_at: DateTime<Utc>,
        actor_id: ActorId,
        now: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<OidcProvider, IdentityError>>;
    fn rename<'a>(
        &'a self,
        id: Uuid,
        name: &'a str,
        actor_id: ActorId,
        now: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<OidcProvider, IdentityError>>;
    fn update_description<'a>(
        &'a self,
        id: Uuid,
        description: Option<&'a str>,
        actor_id: ActorId,
        now: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<OidcProvider, IdentityError>>;
    fn delete(
        &self,
        id: Uuid,
        actor_id: ActorId,
        now: DateTime<Utc>,
    ) -> BoxFuture<'_, Result<(), IdentityError>>;
    fn start_login(
        &self,
        state: OidcLoginState,
        now: DateTime<Utc>,
    ) -> BoxFuture<'_, Result<(), IdentityError>>;
    fn consume_login_state<'a>(
        &'a self,
        state_hash: &'a str,
    ) -> BoxFuture<'a, Result<Option<OidcLoginState>, IdentityError>>;
    fn resolve_identity<'a>(
        &'a self,
        provider: &'a OidcProvider,
        identity: &'a OidcIdentity,
        proposed_name: &'a str,
        now: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<UserAuthentication, IdentityError>>;
}
