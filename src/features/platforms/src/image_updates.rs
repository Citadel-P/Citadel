//! Image metadata checks never enqueue workload operations or download layers.
use crate::image_update_store::{ImageUpdateCandidate, ImageUpdateStore, StoredImageUpdate};
use crate::{
    ImageDetails, RuntimeCapabilityError, RuntimeErrorKind, image_usage::ImageLocation,
    runtime_provider::PlatformRuntimeProvider,
};
use chrono::{DateTime, Utc};
use citadel_primitives::ActorId;
use futures_util::future::BoxFuture;
use futures_util::{FutureExt, StreamExt, stream};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImageUpdateStatus {
    UpToDate,
    UpdateAvailable,
    NotChecked,
    CheckFailed,
    NotApplicable,
    Unknown,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageTagUpdate {
    pub reference: String,
    pub registry_id: Option<Uuid>,
    pub status: ImageUpdateStatus,
    pub reason: Option<String>,
    pub last_attempt_at: Option<DateTime<Utc>>,
    pub last_success_at: Option<DateTime<Utc>>,
    pub local_config_digest: Option<String>,
    pub remote_config_digest: Option<String>,
    pub remote_manifest_digest: Option<String>,
    pub is_stale: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ImagePlatform {
    pub os: String,
    pub architecture: String,
    pub variant: String,
}
#[derive(Clone)]
pub struct ImageUpdateRegistry {
    pub id: Uuid,
    pub host: String,
    pub revision: String,
}
#[derive(Debug, Clone)]
pub struct RegistryImageObservation {
    pub config_digest: String,
    pub manifest_digest: String,
    pub index_digest: String,
    pub observed_at: DateTime<Utc>,
}
pub trait ImageUpdateResolver: Send + Sync {
    fn registries(
        &self,
        actor: ActorId,
        administrator: bool,
    ) -> BoxFuture<'_, Result<Vec<ImageUpdateRegistry>, RuntimeCapabilityError>>;
    fn observe<'a>(
        &'a self,
        platform: Uuid,
        node: Option<&'a str>,
        registry: &'a ImageUpdateRegistry,
        reference: &'a str,
        target: &'a ImagePlatform,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<RegistryImageObservation, RuntimeCapabilityError>>;
}
pub struct ImageUpdateBatch {
    pub next: Option<Uuid>,
    pub checked: usize,
    pub failed: usize,
}
/// The local image and exact tag whose registry content is being compared.
pub struct ImageUpdateTarget<'a> {
    pub platform: Uuid,
    pub location: &'a ImageLocation,
    pub reference: &'a str,
}

type ChangeHandler = Arc<dyn Fn(Uuid, &str) + Send + Sync>;
pub struct ImageUpdateService {
    runtime: Arc<dyn PlatformRuntimeProvider>,
    resolver: Arc<dyn ImageUpdateResolver>,
    store: Arc<dyn ImageUpdateStore>,
    on_change: Option<ChangeHandler>,
    slots: tokio::sync::Semaphore,
}
impl ImageUpdateService {
    pub fn new(
        runtime: Arc<dyn PlatformRuntimeProvider>,
        resolver: Arc<dyn ImageUpdateResolver>,
        store: Arc<dyn ImageUpdateStore>,
    ) -> Self {
        Self {
            runtime,
            resolver,
            store,
            on_change: None,
            slots: tokio::sync::Semaphore::new(4),
        }
    }
    pub fn with_change_handler(mut self, handler: ChangeHandler) -> Self {
        self.on_change = Some(handler);
        self
    }

    /// Scan bounded inventory pages. Recent successes and failures both wait for
    /// the two-hour cadence; a new image/tag identity has no observation and is due.
    pub async fn run_scheduled_batch(
        &self,
        after: Uuid,
        cancel: &CancellationToken,
    ) -> Result<ImageUpdateBatch, RuntimeCapabilityError> {
        let candidates = self.store.candidates(after).await?;
        let next = candidates.last().map(|c| c.id);
        let actor = ActorId::new(Uuid::from_u128(1));
        let registries = self.resolver.registries(actor, true).await?;
        let tasks: Vec<_> = candidates
            .iter()
            .map(|candidate| self.check_candidate(candidate, &registries, cancel).boxed())
            .collect();
        let mut pending = stream::iter(tasks).buffer_unordered(2);
        let mut checked = 0;
        let mut failed = 0;
        while let Some(result) = tokio::select! {
            biased; () = cancel.cancelled() => return Err(failure(RuntimeErrorKind::Cancelled, "Image sweep cancelled.")),
            result = pending.next() => result,
        } {
            match result {
                Ok(count) => checked += count,
                Err(_) => failed += 1,
            }
        }
        Ok(ImageUpdateBatch {
            next,
            checked,
            failed,
        })
    }

    async fn check_candidate(
        &self,
        candidate: &ImageUpdateCandidate,
        registries: &[ImageUpdateRegistry],
        cancel: &CancellationToken,
    ) -> Result<usize, RuntimeCapabilityError> {
        let registry_ids: Vec<_> = registries
            .iter()
            .map(|r| r.id)
            .chain([Uuid::nil()])
            .collect();
        let previous = self
            .store
            .load(
                candidate.platform,
                std::slice::from_ref(&candidate.image),
                &registry_ids,
            )
            .await?;
        let mut checked = 0;
        for reference in candidate.tags.iter().filter(|tag| eligible(tag)) {
            if cancel.is_cancelled() {
                return Err(failure(
                    RuntimeErrorKind::Cancelled,
                    "Image sweep cancelled.",
                ));
            }
            let Some(registry) =
                scheduled_registry(candidate.registry_id, reference, registries, &previous)
            else {
                continue;
            };
            let recent = previous.iter().any(|v| {
                v.registry == registry.id
                    && v.revision == registry.revision
                    && v.update.reference == *reference
                    && v.update
                        .last_attempt_at
                        .is_some_and(|at| Utc::now() - at < chrono::Duration::hours(2))
            });
            if recent {
                continue;
            }
            self.check_resolved(
                ActorId::new(Uuid::from_u128(1)),
                true,
                ImageUpdateTarget {
                    platform: candidate.platform,
                    location: &ImageLocation {
                        docker_image_id: candidate.image.clone(),
                        docker_node_id: None,
                    },
                    reference,
                },
                &registry,
                false,
                cancel,
            )
            .await?;
            checked += 1;
        }
        Ok(checked)
    }

    pub async fn summaries(
        &self,
        actor: ActorId,
        admin: bool,
        platform: Uuid,
        images: &[ImageDetails],
    ) -> Result<Vec<Vec<ImageTagUpdate>>, RuntimeCapabilityError> {
        let registries = self.resolver.registries(actor, admin).await?;
        let registry_ids: Vec<_> = registries
            .iter()
            .map(|r| r.id)
            .chain([Uuid::nil()])
            .collect();
        let image_ids: Vec<_> = images.iter().map(|i| i.docker_image_id.clone()).collect();
        let results = self.store.load(platform, &image_ids, &registry_ids).await?;
        Ok(images.iter().map(|image| {
            if image.tags.is_empty() {
                return vec![empty("", ImageUpdateStatus::NotApplicable, Some("This image has no tag to track."))];
            }
            image.tags.iter().map(|reference| {
                if !eligible(reference) {
                    return empty(reference, ImageUpdateStatus::NotApplicable, Some("Digest-pinned references do not track tag updates."));
                }
                if image.docker_node_id.is_some() {
                    return empty(reference, ImageUpdateStatus::Unknown, Some("Registry checks are unavailable for node-local images."));
                }
                let found = results.iter().filter(|entry| {
                    entry.image == image.docker_image_id
                        && entry.update.reference == *reference
                        && (entry.registry.is_nil() || registries.iter().any(|registry| registry.id == entry.registry && registry.revision == entry.revision))
                }).max_by_key(|entry| entry.update.last_attempt_at);
                let mut result = found.map(|entry| entry.update.clone())
                    .unwrap_or_else(|| empty(reference, ImageUpdateStatus::NotChecked, None));
                let expired = result.last_success_at.is_some_and(|at| Utc::now() - at > chrono::Duration::days(1));
                if image.is_stale || expired {
                    result.is_stale = true;
                    if result.status != ImageUpdateStatus::CheckFailed {
                        result.status = ImageUpdateStatus::Unknown;
                        result.reason = Some("Inventory or registry observation is stale. Check again when the host is available.".into());
                    }
                }
                result
            }).collect()
        }).collect())
    }

    /// Remember an explicitly selected Registry only when the caller can pull
    /// images. Read-only callers can still refresh comparison metadata.
    pub async fn check(
        &self,
        actor: ActorId,
        admin: bool,
        target: ImageUpdateTarget<'_>,
        registry_id: Uuid,
        remember_registry: bool,
        cancel: &CancellationToken,
    ) -> Result<ImageTagUpdate, RuntimeCapabilityError> {
        let registry = self
            .resolver
            .registries(actor, admin)
            .await?
            .into_iter()
            .find(|r| r.id == registry_id)
            .ok_or_else(|| {
                failure(
                    RuntimeErrorKind::PermissionDenied,
                    "The selected Registry is not available.",
                )
            })?;
        self.check_resolved(actor, admin, target, &registry, remember_registry, cancel)
            .await
    }

    /// Refresh an existing comparison invalidated by a successful pull. New
    /// images without observations are picked up by the regular inventory sweep.
    #[allow(clippy::too_many_arguments)]
    pub async fn refresh_after_pull(
        &self,
        actor: ActorId,
        admin: bool,
        platform: Uuid,
        image: &crate::RuntimeImageSummary,
        registry_id: Option<Uuid>,
        reference: &str,
        cancel: &CancellationToken,
    ) -> Result<(), RuntimeCapabilityError> {
        let Some(reference) = image.repo_tags.iter().find(|tag| {
            eligible(tag) && canonical_reference(tag) == canonical_reference(reference)
        }) else {
            return Ok(());
        };
        let registries = self.resolver.registries(actor, admin).await?;
        let ids: Vec<_> = registries
            .iter()
            .map(|r| r.id)
            .chain([Uuid::nil()])
            .collect();
        let previous = self
            .store
            .load(platform, std::slice::from_ref(&image.id), &ids)
            .await?;
        if !previous
            .iter()
            .any(|entry| entry.update.reference == *reference)
        {
            return Ok(());
        }
        let registry = match registry_id {
            Some(id) => registries
                .into_iter()
                .find(|registry| registry.id == id)
                .ok_or_else(|| {
                    failure(
                        RuntimeErrorKind::PermissionDenied,
                        "Registry access changed after the pull.",
                    )
                })?,
            None => ImageUpdateRegistry {
                id: Uuid::nil(),
                host: reference_host(reference),
                revision: "anonymous".into(),
            },
        };
        self.check_resolved(
            actor,
            admin,
            ImageUpdateTarget {
                platform,
                location: &ImageLocation {
                    docker_image_id: image.id.clone(),
                    docker_node_id: None,
                },
                reference,
            },
            &registry,
            false,
            cancel,
        )
        .await?;
        Ok(())
    }

    async fn check_resolved(
        &self,
        actor: ActorId,
        admin: bool,
        target: ImageUpdateTarget<'_>,
        registry: &ImageUpdateRegistry,
        remember_registry: bool,
        cancel: &CancellationToken,
    ) -> Result<ImageTagUpdate, RuntimeCapabilityError> {
        let ImageUpdateTarget {
            platform,
            location,
            reference,
        } = target;
        if location.docker_node_id.is_some() {
            return Err(failure(
                RuntimeErrorKind::Unavailable,
                "Registry checks are unavailable for node-local images under the current Agent operation allowlist.",
            ));
        }
        if !eligible(reference) {
            return Err(failure(
                RuntimeErrorKind::InvalidRequest,
                "Select an exact repository/tag reference.",
            ));
        }
        let _permit = self.slots.try_acquire().map_err(|_| {
            failure(
                RuntimeErrorKind::Unavailable,
                "Image checks are busy. Try again shortly.",
            )
        })?;
        if reference_host(reference) != registry_host(&registry.host) {
            return Err(failure(
                RuntimeErrorKind::InvalidRequest,
                "Choose a Registry matching the image reference.",
            ));
        }
        let started = Utc::now();
        let mut result = empty(reference, ImageUpdateStatus::CheckFailed, None);
        result.registry_id = (!registry.id.is_nil()).then_some(registry.id);
        result.last_attempt_at = Some(started);
        let previous = self
            .store
            .load(
                platform,
                std::slice::from_ref(&location.docker_image_id),
                &[registry.id],
            )
            .await?
            .into_iter()
            .find(|entry| {
                entry.revision == registry.revision && entry.update.reference == reference
            })
            .map(|entry| entry.update);
        if let Some(previous) = previous {
            result.last_success_at = previous.last_success_at;
            result.remote_config_digest = previous.remote_config_digest;
            result.remote_manifest_digest = previous.remote_manifest_digest;
            result.local_config_digest = previous.local_config_digest;
        }
        let attempt = async {
            let runtime = self
                .runtime
                .images(platform, location.docker_node_id.as_deref(), cancel)
                .await?;
            let local = runtime
                .inspect_image(&location.docker_image_id, cancel)
                .await?;
            if local.id != location.docker_image_id
                || !local.repo_tags.iter().any(|t| t == reference)
            {
                return Err(failure(
                    RuntimeErrorKind::Conflict,
                    "The tag no longer belongs to this image. Refresh inventory.",
                ));
            }
            let target = ImagePlatform {
                os: local.os.clone(),
                architecture: local.architecture.clone(),
                variant: local.variant.clone().unwrap_or_default(),
            };
            if target.os.is_empty()
                || target.os == "unknown"
                || target.architecture.is_empty()
                || target.architecture == "unknown"
            {
                return Err(failure(
                    RuntimeErrorKind::Unavailable,
                    "The host did not supply comparable image platform metadata.",
                ));
            }
            let remote = self
                .resolver
                .observe(
                    platform,
                    location.docker_node_id.as_deref(),
                    registry,
                    reference,
                    &target,
                    cancel,
                )
                .await?;
            let local_config = match comparison_identity(&local, &remote)? {
                ComparisonIdentity::Config(digest) => digest.to_owned(),
                ComparisonIdentity::Content(digest) => {
                    // An index can change only because another architecture or
                    // attestation changed. Resolve the old immutable content for
                    // this platform before claiming the local image differs.
                    let repository = canonical_reference(reference);
                    let repository = repository.rsplit_once(':').unwrap().0;
                    let pinned = format!("{repository}@{digest}");
                    let observed = self
                        .resolver
                        .observe(
                            platform,
                            location.docker_node_id.as_deref(),
                            registry,
                            &pinned,
                            &target,
                            cancel,
                        )
                        .await?;
                    if observed.index_digest != digest {
                        return Err(failure(
                            RuntimeErrorKind::Unavailable,
                            "The Registry did not confirm the local image's immutable content.",
                        ));
                    }
                    observed.config_digest
                }
            };
            let current = self
                .runtime
                .images(platform, location.docker_node_id.as_deref(), cancel)
                .await?
                .inspect_image(&location.docker_image_id, cancel)
                .await?;
            if current.id != local.id || !current.repo_tags.iter().any(|t| t == reference) {
                return Err(failure(
                    RuntimeErrorKind::Conflict,
                    "The local tag changed during the check. Refresh inventory and check again.",
                ));
            }
            if !registry.id.is_nil()
                && !self
                    .resolver
                    .registries(actor, admin)
                    .await?
                    .iter()
                    .any(|r| r.id == registry.id && r.revision == registry.revision)
            {
                return Err(failure(
                    RuntimeErrorKind::PermissionDenied,
                    "Registry access or configuration changed during the check.",
                ));
            }
            Ok((local_config, remote))
        };
        let checked = tokio::select! { biased; ()=cancel.cancelled()=>return Err(failure(RuntimeErrorKind::Cancelled,"Image check cancelled.")), value=tokio::time::timeout(std::time::Duration::from_secs(45),attempt)=>value.unwrap_or_else(|_|Err(failure(RuntimeErrorKind::Timeout,"Image check timed out. Retry when the host and Registry are available."))) };
        match checked {
            Ok((local, remote)) => {
                result.status = if local == remote.config_digest {
                    ImageUpdateStatus::UpToDate
                } else {
                    ImageUpdateStatus::UpdateAvailable
                };
                result.reason = (result.status == ImageUpdateStatus::UpdateAvailable).then(|| {
                    "The registry currently publishes different content for this tag.".into()
                });
                result.local_config_digest = Some(local);
                result.remote_config_digest = Some(remote.config_digest);
                result.remote_manifest_digest = Some(remote.manifest_digest);
                result.last_success_at = Some(remote.observed_at);
            }
            Err(error) => {
                if error.kind == RuntimeErrorKind::PermissionDenied {
                    return Err(error);
                }
                result.status = if matches!(
                    error.kind,
                    RuntimeErrorKind::Unavailable | RuntimeErrorKind::Conflict
                ) {
                    ImageUpdateStatus::Unknown
                } else {
                    ImageUpdateStatus::CheckFailed
                };
                result.reason = Some(check_failure_reason(error));
                result.is_stale = result.last_success_at.is_some();
            }
        }
        if self
            .store
            .save(
                platform,
                &location.docker_image_id,
                registry,
                &result,
                remember_registry,
            )
            .await?
            && let Some(on_change) = &self.on_change
        {
            on_change(platform, &location.docker_image_id);
        }
        Ok(result)
    }
}
enum ComparisonIdentity<'a> {
    Config(&'a str),
    Content(&'a str),
}

fn comparison_identity<'a>(
    local: &'a crate::images::ImageInspection,
    remote: &'a RegistryImageObservation,
) -> Result<ComparisonIdentity<'a>, RuntimeCapabilityError> {
    if let Some(digest) = local.config_digest.as_deref().filter(|d| valid_digest(d)) {
        return Ok(ComparisonIdentity::Config(digest));
    }
    if let Some(digest) = local.content_digest.as_deref().filter(|d| valid_digest(d)) {
        if digest == remote.index_digest || digest == remote.manifest_digest {
            return Ok(ComparisonIdentity::Config(&remote.config_digest));
        }
        return Ok(ComparisonIdentity::Content(digest));
    }
    Err(failure(
        RuntimeErrorKind::Unavailable,
        "The host did not supply comparable image identity metadata. Update the Agent and check again.",
    ))
}

fn valid_digest(digest: &str) -> bool {
    digest
        .strip_prefix("sha256:")
        .is_some_and(|h| h.len() == 64 && h.bytes().all(|b| b.is_ascii_hexdigit()))
}

fn scheduled_registry(
    preferred: Option<Uuid>,
    reference: &str,
    registries: &[ImageUpdateRegistry],
    previous: &[StoredImageUpdate],
) -> Option<ImageUpdateRegistry> {
    let host = reference_host(reference);
    let matching: Vec<_> = registries
        .iter()
        .filter(|r| registry_host(&r.host) == host)
        .collect();
    if let Some(registry) = matching.iter().find(|r| Some(r.id) == preferred) {
        return Some((*registry).clone());
    }
    if let Some(observed) = previous
        .iter()
        .filter(|v| {
            v.update.reference == reference
                && matching
                    .iter()
                    .any(|r| r.id == v.registry && r.revision == v.revision)
        })
        .max_by_key(|v| v.update.last_attempt_at)
    {
        return matching
            .iter()
            .find(|r| r.id == observed.registry)
            .map(|r| (*r).clone());
    }
    match matching.as_slice() {
        [] => Some(ImageUpdateRegistry {
            id: Uuid::nil(),
            host,
            revision: "anonymous".into(),
        }),
        [registry] => Some((*registry).clone()),
        _ => None, // Do not guess which credentials to use.
    }
}

fn check_failure_reason(error: RuntimeCapabilityError) -> String {
    match error.kind {
        RuntimeErrorKind::Remote => "The host or Registry could not complete this check.".into(),
        RuntimeErrorKind::Authentication => "Access to the host or Registry was denied. Verify Registry credentials and that the repository/tag is published and readable.".into(),
        RuntimeErrorKind::NotFound => "The image/tag was not found on the host or selected Registry. Locally built images must be published before registry updates can be checked.".into(),
        RuntimeErrorKind::ResourceExhausted => "The host or Registry is limiting requests. Try again later.".into(),
        _ => error.message,
    }
}

fn empty(reference: &str, status: ImageUpdateStatus, reason: Option<&str>) -> ImageTagUpdate {
    ImageTagUpdate {
        reference: reference.into(),
        registry_id: None,
        status,
        reason: reason.map(str::to_owned),
        last_attempt_at: None,
        last_success_at: None,
        local_config_digest: None,
        remote_config_digest: None,
        remote_manifest_digest: None,
        is_stale: false,
    }
}
pub fn eligible(reference: &str) -> bool {
    !reference.is_empty() && !reference.contains('@') && !reference.contains('<')
}
pub fn registry_host(host: &str) -> String {
    let host = host.trim_end_matches('/').to_ascii_lowercase();
    match host.as_str() {
        "hub.docker.com" | "index.docker.io" | "registry-1.docker.io" => "docker.io".into(),
        _ => host,
    }
}
pub fn reference_host(reference: &str) -> String {
    let first = reference.split('/').next().unwrap_or_default();
    if reference.contains('/')
        && (first.contains('.') || first.contains(':') || first == "localhost")
    {
        registry_host(first)
    } else {
        "docker.io".into()
    }
}
fn failure(kind: RuntimeErrorKind, message: &str) -> RuntimeCapabilityError {
    RuntimeCapabilityError::new(kind, message, false)
}

/// Docker Hub aliases normalize without changing tag case or private registry identity.
pub fn canonical_reference(reference: &str) -> String {
    let reference = reference.trim();
    let host = reference_host(reference);
    let first = reference.split('/').next().unwrap_or_default();
    let explicit = reference.contains('/')
        && (first.contains('.') || first.contains(':') || first == "localhost");
    let name = if explicit {
        reference.split_once('/').unwrap().1
    } else {
        reference
    };
    let mut result = if host == "docker.io" && !name.contains('/') {
        format!("{host}/library/{name}")
    } else {
        format!("{host}/{name}")
    };
    if !result.contains('@') && !result.rsplit('/').next().unwrap_or_default().contains(':') {
        result.push_str(":latest");
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn comparisons_distinguish_classic_configs_from_containerd_indexes_and_manifests() {
        use crate::images::ImageInspection;
        let remote = RegistryImageObservation {
            config_digest: format!("sha256:{}", "a".repeat(64)),
            manifest_digest: format!("sha256:{}", "b".repeat(64)),
            index_digest: format!("sha256:{}", "c".repeat(64)),
            observed_at: Utc::now(),
        };
        for digest in [&remote.index_digest, &remote.manifest_digest] {
            let local = ImageInspection {
                id: digest.clone(),
                content_digest: Some(digest.clone()),
                ..Default::default()
            };
            assert!(matches!(comparison_identity(&local, &remote).unwrap(),
                ComparisonIdentity::Config(d) if d == remote.config_digest));
        }
        let old = format!("sha256:{}", "d".repeat(64));
        let mut local = ImageInspection {
            id: old.clone(),
            content_digest: Some(old.clone()),
            ..Default::default()
        };
        // A different index is not itself evidence of a host-platform update.
        assert!(matches!(comparison_identity(&local, &remote).unwrap(),
            ComparisonIdentity::Content(d) if d == old));
        local.content_digest = None;
        local.config_digest = Some(old.clone());
        assert!(matches!(comparison_identity(&local, &remote).unwrap(),
            ComparisonIdentity::Config(d) if d != remote.config_digest));
        // Older Agents omit explicit identities; never guess from the display ID.
        local.config_digest = None;
        assert_eq!(
            comparison_identity(&local, &remote).err().unwrap().kind,
            RuntimeErrorKind::Unavailable
        );
        local.content_digest = Some("not-a-digest".into());
        assert!(comparison_identity(&local, &remote).is_err());
    }
    #[test]
    fn scheduled_registry_selection_does_not_guess_credentials() {
        let a = ImageUpdateRegistry {
            id: Uuid::now_v7(),
            host: "docker.io".into(),
            revision: "a".into(),
        };
        let b = ImageUpdateRegistry {
            id: Uuid::now_v7(),
            host: "index.docker.io".into(),
            revision: "b".into(),
        };
        assert!(scheduled_registry(None, "nginx:17", &[a.clone(), b.clone()], &[]).is_none());
        assert_eq!(
            scheduled_registry(Some(a.id), "nginx:17", &[a.clone(), b.clone()], &[])
                .unwrap()
                .id,
            a.id
        );
        let anonymous = scheduled_registry(
            None,
            "ghcr.io/team/app:latest",
            std::slice::from_ref(&a),
            &[],
        )
        .unwrap();
        assert!(anonymous.id.is_nil());
        assert_eq!(anonymous.host, "ghcr.io");
        let previous = StoredImageUpdate {
            image: "image".into(),
            registry: b.id,
            revision: b.revision.clone(),
            update: empty("nginx:17", ImageUpdateStatus::UpToDate, None),
        };
        assert_eq!(
            scheduled_registry(
                None,
                "nginx:17",
                &[a.clone(), b.clone()],
                std::slice::from_ref(&previous)
            )
            .unwrap()
            .id,
            b.id
        );
        let rotated = ImageUpdateRegistry {
            revision: "rotated".into(),
            ..b
        };
        assert!(scheduled_registry(None, "nginx:17", &[a, rotated], &[previous]).is_none());
    }

    #[test]
    fn check_errors_explain_missing_or_inaccessible_references_without_exposing_secrets() {
        for (kind, explanation) in [
            (RuntimeErrorKind::Authentication, "published and readable"),
            (
                RuntimeErrorKind::NotFound,
                "Locally built images must be published",
            ),
            (RuntimeErrorKind::ResourceExhausted, "limiting requests"),
            (RuntimeErrorKind::Remote, "could not complete"),
        ] {
            let reason = check_failure_reason(failure(kind, "private-response-with-secret"));
            assert!(reason.contains(explanation));
            assert!(!reason.contains("secret"));
        }
        // A tag called local can be published; its name is not provenance.
        assert!(eligible("citadel-agent:local"));
        assert_eq!(
            canonical_reference("citadel-agent:local"),
            "docker.io/library/citadel-agent:local"
        );
    }

    #[test]
    fn contextual_pull_identity_preserves_tag_case_and_registry_ports() {
        assert_eq!(
            canonical_reference("nginx"),
            "docker.io/library/nginx:latest"
        );
        assert_eq!(
            canonical_reference("index.docker.io/nginx:ReleaseA"),
            "docker.io/library/nginx:ReleaseA"
        );
        assert_eq!(
            canonical_reference("registry.example:5000/team/image:ReleaseA"),
            "registry.example:5000/team/image:ReleaseA"
        );
        assert_ne!(
            canonical_reference("nginx:ReleaseA"),
            canonical_reference("nginx:releasea")
        );
        assert!(!eligible("nginx@sha256:abcd"));
        assert!(!eligible("<none>:<none>"));
    }
}
