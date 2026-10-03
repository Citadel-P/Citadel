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
        observed: Option<&'a ImageDeletionObservation>,
    ) -> BoxFuture<'a, Result<(), RuntimeCapabilityError>>;
}
#[allow(clippy::too_many_arguments)]
pub async fn delete_images(
    store: &dyn ImageMutationStore,
    platform_id: Uuid,
    ids: &[String],
    force: bool,
    no_prune: bool,
    runtime: &(impl ImageDeletionPort + ImageInventoryPort + ?Sized),
    shutdown: &CancellationToken,
) -> Result<Vec<BTreeMap<String, String>>, RuntimeCapabilityError> {
    let cancelled = || {
        error(
            RuntimeErrorKind::Cancelled,
            "Image deletion interrupted by shutdown; inventory will be reconciled.",
        )
    };
    let claim = tokio::select! {
        biased;
        () = shutdown.cancelled() => return Err(cancelled()),
        claim = store.claim_deletion(platform_id, ids) => claim?,
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{
        Mutex,
        atomic::{AtomicUsize, Ordering},
    };
    struct Fake {
        dispatched: tokio::sync::Notify,
        calls: AtomicUsize,
        completed: AtomicUsize,
        token: Mutex<Option<CancellationToken>>,
    }
    impl ImageMutationStore for Fake {
        fn prepare_pull<'a>(
            &'a self,
            _: Uuid,
            _: &'a str,
        ) -> BoxFuture<'a, Result<PreparedImagePull, RuntimeCapabilityError>> {
            unreachable!()
        }
        fn persist_pull<'a>(
            &'a self,
            _: Uuid,
            _: Uuid,
            _: &'a RuntimeImageSummary,
        ) -> BoxFuture<'a, Result<(), RuntimeCapabilityError>> {
            unreachable!()
        }
        fn claim_deletion<'a>(
            &'a self,
            _: Uuid,
            _: &'a [String],
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
                assert!(
                    observed.is_none(),
                    "shutdown must not perform network repair"
                );
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
            panic!("network repair after shutdown")
        }
    }
    #[tokio::test]
    async fn shutdown_interrupts_blocked_dispatch_releases_claim_and_never_replays() {
        let fake = Fake {
            dispatched: Default::default(),
            calls: AtomicUsize::new(0),
            completed: AtomicUsize::new(0),
            token: Mutex::new(None),
        };
        let shutdown = CancellationToken::new();
        let ids = vec!["first".into(), "second".into()];
        let operation = delete_images(&fake, Uuid::now_v7(), &ids, false, false, &fake, &shutdown);
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
}
