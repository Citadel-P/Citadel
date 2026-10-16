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
    pub versions: Vec<i64>,
}
pub struct ImageDeletionObservation {
    pub generation: crate::jobs::SnapshotGeneration,
    pub images: Vec<RuntimeImageSummary>,
}
pub type PreparedImagePull = (String, Option<zeroize::Zeroizing<String>>);

pub trait ImageMutationStore: crate::image_usage::ImageUsageStore {
    fn prepare_pull<'a>(
        &'a self,
        registry: Option<Uuid>,
        reference: &'a str,
    ) -> BoxFuture<'a, Result<PreparedImagePull, RuntimeCapabilityError>>;
    fn persist_pull<'a>(
        &'a self,
        platform: Uuid,
        registry: Option<Uuid>,
        image: &'a RuntimeImageSummary,
    ) -> BoxFuture<'a, Result<(), RuntimeCapabilityError>>;
    fn claim_deletion<'a>(
        &'a self,
        platform: Uuid,
        ids: &'a [String],
        force: bool,
        reviews: &'a [crate::image_usage::ImageDeletionReview],
    ) -> BoxFuture<'a, Result<ImageDeletionClaim, RuntimeCapabilityError>>;
    fn complete_deletion<'a>(
        &'a self,
        platform: Uuid,
        claim: &'a ImageDeletionClaim,
        observed: Option<&'a ImageDeletionObservation>,
    ) -> BoxFuture<'a, Result<(), RuntimeCapabilityError>>;
}
#[allow(clippy::too_many_arguments)]
pub async fn delete_images<F, Fut>(
    store: &dyn ImageMutationStore,
    platform_id: Uuid,
    ids: &[String],
    force: bool,
    no_prune: bool,
    reviews: &[crate::image_usage::ImageDeletionReview],
    runtime: &(
         impl ImageDeletionPort + crate::images::ImageInspectionPort + ImageInventoryPort + ?Sized
     ),
    shutdown: &CancellationToken,
    authorize: F,
) -> Result<Vec<BTreeMap<String, String>>, RuntimeCapabilityError>
where
    F: Fn() -> Fut + Send + Sync,
    Fut: std::future::Future<Output = Result<(), RuntimeCapabilityError>> + Send,
{
    let cancelled = || {
        error(
            RuntimeErrorKind::Cancelled,
            "Image deletion interrupted by shutdown; inventory will be reconciled.",
        )
    };
    let claim = tokio::select! {
        biased;
        () = shutdown.cancelled() => return Err(cancelled()),
        claim = store.claim_deletion(platform_id, ids, force, reviews) => claim?,
    };
    let cancel = shutdown.child_token();
    let _guard = cancel.clone().drop_guard();
    let result = tokio::select! {
        biased;
        () = cancel.cancelled() => Err(cancelled()),
        result = tokio::time::timeout(Duration::from_secs(60), async {
            let mut results = Vec::new();
            for id in ids {
                if cancel.is_cancelled() { return Err(cancelled()); }
                if !force {
                    let image = runtime.inspect_image(id, &cancel).await?;
                    if image.id != *id || !image.containers.is_empty() {
                        return Err(error(RuntimeErrorKind::Conflict,
                            "The image identity changed or a container references it. Refresh inventory and review deletion again."));
                    }
                    if let Some(review) = reviews.iter().find(|review| review.docker_image_id == *id) {
                        let mut tags = image.repo_tags;
                        tags.sort();
                        if tags != review.tags {
                            return Err(error(RuntimeErrorKind::Conflict,
                                "Image tags changed after review. Review deletion again."));
                        }
                    }
                }
                authorize().await?;
                results.extend(runtime.delete_image(id, force, no_prune, &cancel).await?);
            }
            Ok(results)
        }) => result.unwrap_or_else(|_| Err(error(RuntimeErrorKind::Timeout,
            "Image deletion timed out. Refresh inventory before retrying."))),
    };
    // Capture BEFORE the read. A later pull must invalidate this observation.
    // Shutdown skips network repair; only bounded claim release is attempted.
    let observed = tokio::select! {
        biased;
        () = shutdown.cancelled() => None,
        observed = tokio::time::timeout(Duration::from_secs(15), async {
            let generation = crate::jobs::SnapshotGeneration::capture(
                platform_id, None, crate::jobs::ProjectionKind::Images).await;
            runtime.list_images(&cancel).await.map(|images| ImageDeletionObservation { generation, images })
        }) => observed.ok().and_then(Result::ok),
    };
    // Independent of browser/shutdown cancellation, but finite and still owned by
    // the admitted task. A timed-out release leaves an expiring durable claim.
    tokio::time::timeout(
        Duration::from_secs(2),
        store.complete_deletion(platform_id, &claim, observed.as_ref()),
    )
    .await
    .map_err(|_| {
        error(
            RuntimeErrorKind::Timeout,
            "Image cleanup timed out; the operation remains recoverable.",
        )
    })??;
    result
}

fn error(kind: RuntimeErrorKind, message: &str) -> RuntimeCapabilityError {
    RuntimeCapabilityError::new(kind, message, false)
}

/// Commit observed image state before publishing completion to subscribers.
#[allow(clippy::too_many_arguments)]
pub async fn pull_and_persist(
    store: &dyn ImageMutationStore,
    runtime: &(
         impl crate::image_pull::ImagePullPort
         + crate::ImageInventoryPort
         + crate::images::ImageInspectionPort
         + ?Sized
     ),
    input: &crate::image_pull::PullImageInput,
    image: &str,
    auth: Option<&str>,
    sender: &tokio::sync::mpsc::Sender<crate::image_pull::PullImageStreamItem>,
    cancel: &tokio_util::sync::CancellationToken,
    changed: impl Fn(&str, &str, &str),
) -> Result<RuntimeImageSummary, crate::RuntimeCapabilityError> {
    if let Some(source) = &input.source_image_id {
        let reference = crate::image_updates::canonical_reference(&input.image_tag);
        if !crate::image_updates::eligible(&input.image_tag)
            || crate::image_updates::canonical_reference(image) != reference
        {
            return Err(error(
                RuntimeErrorKind::InvalidRequest,
                "The selected Registry would change the requested image reference. Choose a matching Registry.",
            ));
        }
        let local = runtime.inspect_image(source, cancel).await?;
        if local.id != *source
            || !local
                .repo_tags
                .iter()
                .any(|tag| crate::image_updates::canonical_reference(tag) == reference)
        {
            return Err(error(
                RuntimeErrorKind::Conflict,
                "The selected tag no longer belongs to this image. Refresh inventory before pulling.",
            ));
        }
    }
    let observed =
        crate::image_pull::pull_and_observe(runtime, image, auth, sender, cancel).await?;
    store
        .persist_pull(input.platform_id, input.registry_id, &observed)
        .await?;
    changed("image", "create", &observed.id);
    if let Some(source) = &input.source_image_id {
        changed("image", "update", source);
    }
    changed("platform", "update", &input.platform_id.to_string());
    let digest = observed
        .repo_digests
        .first()
        .and_then(|v| v.split_once('@'))
        .map(|(_, d)| d.to_owned());
    let _ = sender
        .send(crate::image_pull::PullImageStreamItem {
            docker_image_id: Some(observed.id.clone()),
            digest,
            ..Default::default()
        })
        .await;
    Ok(observed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{
        Mutex,
        atomic::{AtomicUsize, Ordering},
    };
    #[derive(Default)]
    struct Fake {
        image: Option<crate::images::ImageInspection>,
        repair: bool,
        dispatched: tokio::sync::Notify,
        calls: AtomicUsize,
        completed: AtomicUsize,
        token: Mutex<Option<CancellationToken>>,
    }
    impl crate::image_usage::ImageUsageStore for Fake {
        fn summaries<'a>(
            &'a self,
            _: citadel_primitives::ActorId,
            _: bool,
            _: uuid::Uuid,
            _: &'a [crate::ImageDetails],
        ) -> BoxFuture<'a, Result<Vec<crate::image_usage::ImageUsageSummary>, RuntimeCapabilityError>>
        {
            unreachable!()
        }

        fn deletion_snapshots<'a>(
            &'a self,
            _: Uuid,
            _: &'a [String],
        ) -> BoxFuture<
            'a,
            Result<Vec<crate::image_usage::ImageDeletionSnapshot>, RuntimeCapabilityError>,
        > {
            unimplemented!()
        }
        fn usage<'a>(
            &'a self,
            _: citadel_primitives::ActorId,
            _: bool,
            _: Uuid,
            _: &'a crate::image_usage::ImageLocation,
            _: u32,
        ) -> BoxFuture<'a, Result<crate::image_usage::ImageUsage, RuntimeCapabilityError>> {
            unimplemented!()
        }
    }
    impl crate::images::ImageInspectionPort for Fake {
        fn exposed_ports<'a>(
            &'a self,
            _: &'a str,
            _: &'a CancellationToken,
        ) -> BoxFuture<'a, Result<Vec<String>, RuntimeCapabilityError>> {
            unimplemented!()
        }
        fn inspect_image<'a>(
            &'a self,
            _: &'a str,
            _: &'a CancellationToken,
        ) -> BoxFuture<'a, Result<crate::images::ImageInspection, RuntimeCapabilityError>> {
            Box::pin(async { Ok(self.image.clone().expect("unexpected image inspection")) })
        }
    }
    impl ImageMutationStore for Fake {
        fn prepare_pull<'a>(
            &'a self,
            _: Option<Uuid>,
            _: &'a str,
        ) -> BoxFuture<'a, Result<PreparedImagePull, RuntimeCapabilityError>> {
            unreachable!()
        }
        fn persist_pull<'a>(
            &'a self,
            _: Uuid,
            _: Option<Uuid>,
            _: &'a RuntimeImageSummary,
        ) -> BoxFuture<'a, Result<(), RuntimeCapabilityError>> {
            unreachable!()
        }
        fn claim_deletion<'a>(
            &'a self,
            _: Uuid,
            _: &'a [String],
            _: bool,
            _: &'a [crate::image_usage::ImageDeletionReview],
        ) -> BoxFuture<'a, Result<ImageDeletionClaim, RuntimeCapabilityError>> {
            Box::pin(async {
                Ok(ImageDeletionClaim {
                    ids: vec![Uuid::now_v7()],
                    versions: vec![1],
                    started: 1,
                })
            })
        }
        fn complete_deletion<'a>(
            &'a self,
            _: Uuid,
            _: &'a ImageDeletionClaim,
            observed: Option<&'a ImageDeletionObservation>,
        ) -> BoxFuture<'a, Result<(), RuntimeCapabilityError>> {
            Box::pin(async move {
                if !self.repair {
                    assert!(
                        observed.is_none(),
                        "shutdown must not perform network repair"
                    );
                }
                self.completed.fetch_add(1, Ordering::SeqCst);
                Ok(())
            })
        }
    }
    impl ImageDeletionPort for Fake {
        fn delete_image<'a>(
            &'a self,
            _: &'a str,
            _: bool,
            _: bool,
            token: &'a CancellationToken,
        ) -> BoxFuture<'a, Result<Vec<BTreeMap<String, String>>, RuntimeCapabilityError>> {
            Box::pin(async move {
                self.calls.fetch_add(1, Ordering::SeqCst);
                *self.token.lock().unwrap() = Some(token.clone());
                self.dispatched.notify_one();
                std::future::pending().await
            })
        }
    }
    impl ImageInventoryPort for Fake {
        fn list_images<'a>(
            &'a self,
            _: &'a CancellationToken,
        ) -> BoxFuture<'a, Result<Vec<RuntimeImageSummary>, RuntimeCapabilityError>> {
            assert!(self.repair, "network repair after shutdown");
            Box::pin(async { Ok(vec![]) })
        }
    }
    #[tokio::test]
    async fn shutdown_interrupts_blocked_dispatch_releases_claim_and_never_replays() {
        let fake = Fake {
            image: None,
            repair: false,
            dispatched: Default::default(),
            calls: AtomicUsize::new(0),
            completed: AtomicUsize::new(0),
            token: Mutex::new(None),
        };
        let shutdown = CancellationToken::new();
        let ids = vec!["first".into(), "second".into()];
        let operation = delete_images(
            &fake,
            Uuid::now_v7(),
            &ids,
            true,
            false,
            &[],
            &fake,
            &shutdown,
            || async { Ok(()) },
        );
        let stop = async {
            fake.dispatched.notified().await;
            shutdown.cancel();
        };
        let (result, ()) = tokio::time::timeout(Duration::from_secs(1), async {
            tokio::join!(operation, stop)
        })
        .await
        .unwrap();
        assert_eq!(result.unwrap_err().kind, RuntimeErrorKind::Cancelled);
        assert_eq!(fake.calls.load(Ordering::SeqCst), 1);
        assert_eq!(fake.completed.load(Ordering::SeqCst), 1);
        assert!(fake.token.lock().unwrap().as_ref().unwrap().is_cancelled());
    }
    #[tokio::test]
    async fn live_stopped_usage_changed_tags_and_revoked_authorization_block_dispatch() {
        use crate::images::{ImageContainer, ImageInspection};
        for case in ["stopped", "tags", "permission"] {
            let mut image = ImageInspection {
                id: "image".into(),
                repo_tags: vec!["app:latest".into()],
                ..Default::default()
            };
            if case == "stopped" {
                image.containers.push(ImageContainer {
                    id: "container".into(),
                    name: "stopped".into(),
                    state: "exited".into(),
                    volumes: vec![],
                    networks: Default::default(),
                    ports: serde_json::Value::Null,
                });
            }
            let fake = Fake {
                image: Some(image),
                repair: true,
                ..Default::default()
            };
            let review = crate::image_usage::ImageDeletionReview {
                docker_image_id: "image".into(),
                row_version: 1,
                tags: vec![
                    if case == "tags" {
                        "app:old"
                    } else {
                        "app:latest"
                    }
                    .into(),
                ],
            };
            let result = tokio::time::timeout(
                Duration::from_secs(1),
                delete_images(
                    &fake,
                    Uuid::now_v7(),
                    &["image".into()],
                    false,
                    true,
                    &[review],
                    &fake,
                    &CancellationToken::new(),
                    || async { Err(error(RuntimeErrorKind::PermissionDenied, "revoked")) },
                ),
            )
            .await
            .unwrap();
            assert_eq!(
                result.unwrap_err().kind,
                if case == "permission" {
                    RuntimeErrorKind::PermissionDenied
                } else {
                    RuntimeErrorKind::Conflict
                }
            );
            assert_eq!(fake.calls.load(Ordering::SeqCst), 0);
            assert_eq!(fake.completed.load(Ordering::SeqCst), 1);
        }
    }
}
