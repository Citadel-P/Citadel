use crate::{
    NewRegistry, RegistryConnectionChecker, RegistryDetails, RegistryError, RegistryRepository,
};
use citadel_primitives::ActorId;

pub async fn create_registry(
    repository: &dyn RegistryRepository,
    checker: &dyn RegistryConnectionChecker,
    actor: ActorId,
    input: &mut NewRegistry,
) -> Result<RegistryDetails, RegistryError> {
    input.validate()?;
    checker.check(&input.configuration).await?;
    repository.create_registry(actor, input).await
}

pub async fn update_registry(
    repository: &dyn RegistryRepository,
    checker: &dyn RegistryConnectionChecker,
    actor: ActorId,
    id: uuid::Uuid,
    patch: &crate::RegistryPatch,
    kind: crate::RegistryMutationKind,
) -> Result<RegistryDetails, RegistryError> {
    if kind == crate::RegistryMutationKind::Update
        && !matches!(patch.configuration, crate::MetadataPatch::Missing)
    {
        let current = repository.get_registry(id).await?;
        let mut updated = patch.apply_to(&current);
        updated.validate()?;
        checker.check(&updated.configuration).await?;
    }
    repository.update_registry(actor, id, patch, kind).await
}
