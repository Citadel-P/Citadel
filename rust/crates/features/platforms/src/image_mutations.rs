use crate::{
    ImageInventoryPort, RuntimeCapabilityError, RuntimeErrorKind, RuntimeImageSummary,
    images::ImageDeletionPort,
};
use futures_util::future::BoxFuture;
use std::{collections::BTreeMap, time::Duration};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

pub struct ImageDeletionClaim {
    pub ids: Vec<Uuid>,
    pub started: i64,
}
pub type PreparedImagePull = (String, Option<zeroize::Zeroizing<String>>);

pub trait ImageMutationStore: Send + Sync {
    fn prepare_pull<'a>(
        &'a self,
        registry: Uuid,
        reference: &'a str,
    ) -> BoxFuture<'a, Result<PreparedImagePull, RuntimeCapabilityError>>;
    fn persist_pull<'a>(
        &'a self,
        platform: Uuid,
        registry: Uuid,
        image: &'a RuntimeImageSummary,
    ) -> BoxFuture<'a, Result<(), RuntimeCapabilityError>>;
    fn claim_deletion<'a>(
        &'a self,
        platform: Uuid,
        ids: &'a [String],
    ) -> BoxFuture<'a, Result<ImageDeletionClaim, RuntimeCapabilityError>>;
    fn complete_deletion<'a>(
        &'a self,
        platform: Uuid,
        claim: &'a ImageDeletionClaim,
        observed: Option<&'a [RuntimeImageSummary]>,
    ) -> BoxFuture<'a, Result<(), RuntimeCapabilityError>>;
}
pub async fn delete_images(
    store: &dyn ImageMutationStore,
    platform_id: Uuid,
    ids: &[String],
    force: bool,
    no_prune: bool,
    runtime: &(impl ImageDeletionPort + ImageInventoryPort + ?Sized),
) -> Result<Vec<BTreeMap<String, String>>, RuntimeCapabilityError> {
    let claim = store.claim_deletion(platform_id, ids).await?;
    let cancel = CancellationToken::new();
    let _guard = cancel.clone().drop_guard();
    let result = tokio::time::timeout(Duration::from_secs(60), async {
        let mut results = Vec::new();
        for id in ids {
            results.extend(runtime.delete_image(id, force, no_prune, &cancel).await?);
        }
        Ok(results)
    })
    .await
    .unwrap_or_else(|_| {
        Err(error(
            RuntimeErrorKind::Timeout,
            "Image deletion timed out. Refresh inventory before retrying.",
        ))
    });

    // Even success can mean an untagged image, rather than removal of its data.
    // Trust a fresh inventory, not the requested ID list or a partial response.
    let observed =
        tokio::time::timeout(Duration::from_secs(15), runtime.list_images(&cancel)).await;

    store
        .complete_deletion(
            platform_id,
            &claim,
            observed
                .as_ref()
                .ok()
                .and_then(|v| v.as_ref().ok())
                .map(Vec::as_slice),
        )
        .await?;
    result
}
fn error(kind: RuntimeErrorKind, message: &str) -> RuntimeCapabilityError {
    RuntimeCapabilityError::new(kind, message, false)
}

/// Commit observed image state before publishing completion to subscribers.
#[allow(clippy::too_many_arguments)]
pub async fn pull_and_persist(
    store: &dyn ImageMutationStore,
    runtime: &(impl crate::image_pull::ImagePullPort + crate::ImageInventoryPort + ?Sized),
    input: &crate::image_pull::PullImageInput,
    image: &str,
    auth: Option<&str>,
    sender: &tokio::sync::mpsc::Sender<crate::image_pull::PullImageStreamItem>,
    cancel: &tokio_util::sync::CancellationToken,
    changed: impl Fn(&str, &str, &str),
) -> Result<(), crate::RuntimeCapabilityError> {
    let observed =
        crate::image_pull::pull_and_observe(runtime, image, auth, sender, cancel).await?;
    store
        .persist_pull(input.platform_id, input.registry_id, &observed)
        .await?;
    changed("image", "create", &observed.id);
    changed("platform", "update", &input.platform_id.to_string());
    let digest = observed
        .repo_digests
        .first()
        .and_then(|v| v.split_once('@'))
        .map(|(_, d)| d.to_owned());
    let _ = sender
        .send(crate::image_pull::PullImageStreamItem {
            docker_image_id: Some(observed.id),
            digest,
            ..Default::default()
        })
        .await;
    Ok(())
}
