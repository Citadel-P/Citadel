use crate::{ImageInventoryPort, RuntimeCapabilityError, RuntimeErrorKind, RuntimeImageSummary};
use futures_util::{Stream, StreamExt, future::BoxFuture};
use serde::{Deserialize, Serialize};
use std::pin::Pin;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PullImageInput {
    pub platform_id: Uuid,
    pub registry_id: Uuid,
    pub image_tag: String,
}
#[derive(Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PullImageStreamItem {
    pub id: Option<String>,
    pub from: Option<String>,
    pub stream: Option<String>,
    pub status: Option<String>,
    pub error_message: Option<String>,
    pub progress_message: Option<String>,
    pub docker_image_id: Option<String>,
    pub digest: Option<String>,
    pub progress: Option<ImagePullProgress>,
    pub error: Option<ImagePullError>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImagePullProgress {
    pub units: Option<String>,
    pub current: Option<i64>,
    pub total: Option<i64>,
    pub start: Option<i64>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImagePullError {
    pub code: Option<i64>,
    pub message: Option<String>,
}
pub type ImagePullStream =
    Pin<Box<dyn Stream<Item = Result<PullImageStreamItem, RuntimeCapabilityError>> + Send>>;
pub trait ImagePullPort: Send + Sync {
    fn pull_image_stream<'a>(
        &'a self,
        image: &'a str,
        auth: Option<&'a str>,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<ImagePullStream, RuntimeCapabilityError>>;
}

/// Forward bounded progress and confirm the requested image exists before persistence.
/// The caller owns admission, deadlines, credentials, and response-body cancellation.
pub async fn pull_and_observe(
    runtime: &(impl ImagePullPort + ImageInventoryPort + ?Sized),
    image: &str,
    auth: Option<&str>,
    sender: &tokio::sync::mpsc::Sender<PullImageStreamItem>,
    cancel: &CancellationToken,
) -> Result<RuntimeImageSummary, RuntimeCapabilityError> {
    let mut stream = runtime.pull_image_stream(image, auth, cancel).await?;
    while let Some(item) = tokio::select! {
        biased;
        () = cancel.cancelled() => return Err(cancelled()),
        item = stream.next() => item,
    } {
        let item = item?;
        if item.error_message.as_ref().is_some_and(|v| !v.is_empty())
            || item
                .error
                .as_ref()
                .and_then(|e| e.message.as_ref())
                .is_some_and(|v| !v.is_empty())
        {
            return Err(RuntimeCapabilityError::new(
                RuntimeErrorKind::Remote,
                "Image pull failed. Check the image reference and Registry credentials.",
                false,
            ));
        }
        tokio::select! {
            biased;
            () = cancel.cancelled() => return Err(cancelled()),
            result = sender.send(item) => if result.is_err() { return Err(cancelled()); }
        }
    }
    if cancel.is_cancelled() {
        return Err(cancelled());
    }
    runtime
        .list_images(cancel)
        .await?
        .into_iter()
        .find(|value| {
            value
                .repo_tags
                .iter()
                .chain(&value.repo_digests)
                .any(|reference| same_image_reference(reference, image))
        })
        .ok_or_else(|| {
            RuntimeCapabilityError::new(
                RuntimeErrorKind::NotFound,
                "Pull completed, but Docker did not report the requested image.",
                false,
            )
        })
}

fn cancelled() -> RuntimeCapabilityError {
    RuntimeCapabilityError::new(RuntimeErrorKind::Cancelled, "Image pull cancelled.", false)
}

fn same_image_reference(left: &str, right: &str) -> bool {
    fn docker_hub_reference(value: &str) -> &str {
        let value = value
            .strip_prefix("docker.io/")
            .or_else(|| value.strip_prefix("index.docker.io/"))
            .unwrap_or(value);
        value.strip_prefix("library/").unwrap_or(value)
    }
    docker_hub_reference(left) == docker_hub_reference(right)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pulled_image_lookup_accepts_hub_aliases_without_changing_tags_or_private_registries() {
        assert!(same_image_reference(
            "docker.io/library/nginx:ReleaseA",
            "nginx:ReleaseA"
        ));
        assert!(same_image_reference(
            "index.docker.io/team/image:Tag",
            "team/image:Tag"
        ));
        assert!(!same_image_reference("nginx:ReleaseA", "nginx:releasea"));
        assert!(!same_image_reference(
            "private.example/library/nginx:Tag",
            "nginx:Tag"
        ));
        assert!(!same_image_reference("other/nginx:Tag", "nginx:Tag"));
    }
    struct PullRuntime {
        failure: bool,
        stalled: bool,
        observed_tag: &'static str,
        reads: std::sync::atomic::AtomicUsize,
    }
    impl ImagePullPort for PullRuntime {
        fn pull_image_stream<'a>(
            &'a self,
            _: &'a str,
            _: Option<&'a str>,
            _: &'a CancellationToken,
        ) -> BoxFuture<'a, Result<ImagePullStream, RuntimeCapabilityError>> {
            Box::pin(async move {
                if self.stalled {
                    return Ok(Box::pin(futures_util::stream::pending()) as ImagePullStream);
                }
                Ok(
                    Box::pin(futures_util::stream::iter([Ok(PullImageStreamItem {
                        status: Some("Pulling".into()),
                        error_message: self
                            .failure
                            .then(|| "private registry credential error".into()),
                        ..Default::default()
                    })])) as ImagePullStream,
                )
            })
        }
    }
    impl ImageInventoryPort for PullRuntime {
        fn list_images<'a>(
            &'a self,
            _: &'a CancellationToken,
        ) -> BoxFuture<'a, Result<Vec<RuntimeImageSummary>, RuntimeCapabilityError>> {
            Box::pin(async move {
                self.reads.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                Ok(vec![RuntimeImageSummary {
                    id: "confirmed-image".into(),
                    repo_tags: vec![self.observed_tag.into()],
                    ..Default::default()
                }])
            })
        }
    }
    fn runtime() -> PullRuntime {
        PullRuntime {
            failure: false,
            stalled: false,
            observed_tag: "docker.io/library/nginx:ReleaseA",
            reads: Default::default(),
        }
    }

    #[tokio::test]
    async fn pull_requires_matching_inventory_and_sanitizes_embedded_errors() {
        for (failure, tag, expected) in [
            (false, "docker.io/library/nginx:ReleaseA", None),
            (false, "nginx:releasea", Some(RuntimeErrorKind::NotFound)),
            (true, "nginx:ReleaseA", Some(RuntimeErrorKind::Remote)),
        ] {
            let runtime = PullRuntime {
                failure,
                observed_tag: tag,
                ..runtime()
            };
            let (sender, mut receiver) = tokio::sync::mpsc::channel(1);
            let result = pull_and_observe(
                &runtime,
                "nginx:ReleaseA",
                None,
                &sender,
                &CancellationToken::new(),
            )
            .await;
            if let Some(kind) = expected {
                let error = result.unwrap_err();
                assert_eq!(error.kind, kind);
                assert!(!error.message.contains("private registry credential"));
            } else {
                assert_eq!(result.unwrap().id, "confirmed-image");
            }
            assert_eq!(
                runtime.reads.load(std::sync::atomic::Ordering::SeqCst),
                usize::from(!failure)
            );
            assert_eq!(receiver.try_recv().is_ok(), !failure);
        }
    }

    #[tokio::test(start_paused = true)]
    async fn cancellation_interrupts_a_stalled_stream_or_progress_queue_without_observation() {
        for stalled in [true, false] {
            let runtime = PullRuntime {
                stalled,
                ..runtime()
            };
            let (sender, _receiver) = tokio::sync::mpsc::channel(1);
            assert!(sender.try_send(PullImageStreamItem::default()).is_ok());
            let cancel = CancellationToken::new();
            let trigger = cancel.clone();
            tokio::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                trigger.cancel();
            });
            let result = tokio::time::timeout(
                std::time::Duration::from_secs(2),
                pull_and_observe(&runtime, "nginx:ReleaseA", None, &sender, &cancel),
            )
            .await
            .expect("cancellation must interrupt progress")
            .unwrap_err();
            assert_eq!(result.kind, RuntimeErrorKind::Cancelled);
            assert_eq!(runtime.reads.load(std::sync::atomic::Ordering::SeqCst), 0);
        }
    }
}
