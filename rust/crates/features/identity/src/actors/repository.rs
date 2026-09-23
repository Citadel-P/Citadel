use super::*;

pub trait ActorRepository: Send + Sync {
    fn get(&self, id: Uuid) -> BoxFuture<'_, Result<ActorDetails, IdentityError>>;
    fn set_enabled(
        &self,
        id: Uuid,
        enabled: bool,
    ) -> BoxFuture<'_, Result<ActorDetails, IdentityError>>;
}
