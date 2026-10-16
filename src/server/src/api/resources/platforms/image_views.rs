//! Actor-specific image decoration shared by HTTP and realtime snapshots.
use super::views::{ImageCapabilitiesView, ImageRegistryView, ImageView, ImagesResponse};
use citadel_identity::ActorPrincipal;
use citadel_platforms::{
    ImageDetails, RuntimeCapabilityError, RuntimeErrorKind, image_mutations::ImageMutationStore,
    image_updates::ImageUpdateService,
};
use citadel_registries::RegistryRepository;
use uuid::Uuid;

pub(crate) async fn image_list_view(
    principal: &ActorPrincipal,
    platform: Uuid,
    inventory: Vec<ImageDetails>,
    capabilities: ImageCapabilitiesView,
    store: &dyn ImageMutationStore,
    updates: &ImageUpdateService,
    registries: &dyn RegistryRepository,
) -> Result<ImagesResponse, RuntimeCapabilityError> {
    let (usage, updates) = tokio::join!(
        store.summaries(
            principal.actor_id,
            principal.is_administrator(),
            platform,
            &inventory
        ),
        updates.summaries(
            principal.actor_id,
            principal.is_administrator(),
            platform,
            &inventory
        ),
    );
    let mut images: Vec<_> = inventory
        .into_iter()
        .zip(usage?)
        .zip(updates?)
        .map(|((image, usage), updates)| {
            let mut view = ImageView::from(image);
            view.used_by = Some(usage);
            view.updates = Some(updates);
            view.capabilities = Some(capabilities);
            view
        })
        .collect();
    let registries = if images.iter().any(|image| image.registry_id.is_some()) {
        registries
            .list_registries(principal.actor_id, principal.is_administrator())
            .await
            .map_err(|_| registry_error())?
    } else {
        Vec::new()
    };
    let registries = registries
        .into_iter()
        .map(|registry| {
            Ok((
                registry.id,
                ImageRegistryView {
                    id: registry.id,
                    name: registry.name,
                    registry_host: registry.registry_host,
                    registry_type: serde_json::from_value(registry.registry_type.into())
                        .map_err(|_| registry_error())?,
                },
            ))
        })
        .collect::<Result<std::collections::HashMap<_, _>, RuntimeCapabilityError>>()?;
    for image in &mut images {
        image.registry = image
            .registry_id
            .and_then(|id| registries.get(&id).cloned());
    }
    Ok(ImagesResponse {
        images,
        capabilities,
    })
}

fn registry_error() -> RuntimeCapabilityError {
    RuntimeCapabilityError::new(
        RuntimeErrorKind::Unavailable,
        "Could not load image Registry summaries.",
        false,
    )
}
