//! Bounded image checks. The persistent lease is separate from the last Docker
//! operation, so cancellation/restart cannot turn a check into a deletion claim.
use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use futures_util::future::BoxFuture;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::{
    AutoUpdateState, SwarmServiceDetails, SwarmServiceError, SwarmServiceImageInfo,
    SwarmServiceService,
};
use citadel_domain::ActorId;

pub trait ServiceAutomationEntitlements: Send + Sync {
    fn enabled(
        &self,
        capability: citadel_domain::LicenseCapability,
    ) -> BoxFuture<'_, Result<bool, SwarmServiceError>>;
}

pub enum ServiceUpdateOutcome {
    UpdateAvailable,
    ApplyStarted,
    Noop(&'static str),
}

pub struct ServiceUpdateCheck {
    pub lease_id: Uuid,
    pub service: SwarmServiceDetails,
}

pub trait ServiceImageDigestPort: Send + Sync {
    fn cached_digest<'a>(
        &'a self,
        platform: Uuid,
        registry: Uuid,
        reference: &'a str,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Option<String>, SwarmServiceError>> {
        Box::pin(async move {
            self.digest(platform, registry, reference, cancellation)
                .await
                .map(Some)
        })
    }
    fn digest<'a>(
        &'a self,
        platform: Uuid,
        registry: Uuid,
        reference: &'a str,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<String, SwarmServiceError>>;
}

/// Only the configured external tag may be checked. A saved desired image is
/// not an applied baseline, and a digest-pinned image cannot have tag updates.
pub fn checkable_image(service: &SwarmServiceDetails) -> Result<(Uuid, &str), SwarmServiceError> {
    if service.control_state == "Processing" {
        return Err(SwarmServiceError::Conflict(
            "The Service is currently processing another operation.".into(),
        ));
    }
    let SwarmServiceImageInfo::External {
        registry_id,
        image_tag,
        ..
    } = &service.spec.image
    else {
        return Err(SwarmServiceError::Validation(
            "Only external tagged images support update checks.".into(),
        ));
    };
    if registry_id.is_nil() || image_tag.contains('@') || image_tag.trim().is_empty() {
        return Err(SwarmServiceError::Validation(
            "Only external tagged images support update checks.".into(),
        ));
    }
    if service
        .applied_image_digest
        .as_deref()
        .is_none_or(|value| value.trim().is_empty())
    {
        return Err(SwarmServiceError::Conflict(
            "The Service has no applied image digest to compare.".into(),
        ));
    }
    Ok((*registry_id, image_tag))
}

pub fn evaluate_digest(current: &str, remote: &str) -> AutoUpdateState {
    // Docker can return repository@digest for an applied image but only digest
    // from distribution inspection. Compare content, not these display forms.
    let digest = |reference: &str| reference.rsplit('@').next().unwrap_or(reference).to_owned();
    let current = digest(current);
    let remote = digest(remote);
    AutoUpdateState {
        last_checked_at: Utc::now(),
        status: if current.eq_ignore_ascii_case(&remote) {
            "UpToDate"
        } else {
            "UpdateAvailable"
        }
        .into(),
        current_digest: Some(current),
        remote_digest: Some(remote),
        last_error: None,
    }
}

impl SwarmServiceService {
    pub fn with_entitlements(
        mut self,
        entitlements: Arc<dyn ServiceAutomationEntitlements>,
    ) -> Self {
        self.entitlements = Some(entitlements);
        self
    }

    /// Shared by authenticated webhooks and periodic producers. The supplied
    /// snapshot must be the configuration used to authorize the trigger.
    pub async fn check_automated_updates(
        &self,
        snapshot: SwarmServiceDetails,
        cancellation: &CancellationToken,
    ) -> Result<ServiceUpdateOutcome, SwarmServiceError> {
        self.check_automated_updates_mode(snapshot, false, cancellation)
            .await
    }
    pub(crate) async fn check_automated_updates_mode(
        &self,
        snapshot: SwarmServiceDetails,
        scheduled: bool,
        cancellation: &CancellationToken,
    ) -> Result<ServiceUpdateOutcome, SwarmServiceError> {
        use crate::{ServiceOperationKind, ServiceOperationRequest, UpdateBehavior};
        use citadel_domain::LicenseCapability;
        if snapshot.spec.update_behavior == UpdateBehavior::Disabled {
            return Ok(ServiceUpdateOutcome::Noop(
                "Service image updates are disabled.",
            ));
        }
        // Webhook-triggered checks require automation entitlement before registry
        // I/O. Scheduled Notify checks consume the shared cache in Community too.
        if !scheduled {
            let Some(entitlements) = &self.entitlements else {
                return Ok(ServiceUpdateOutcome::Noop(
                    "Automated operations require an active license entitlement.",
                ));
            };
            if !entitlements
                .enabled(LicenseCapability::AutomatedOperations)
                .await?
            {
                return Ok(ServiceUpdateOutcome::Noop(
                    "Automated operations require an active license entitlement.",
                ));
            }
        }
        let actor = ActorId::new(Uuid::from_u128(1));
        let cached = if scheduled {
            let (registry, reference) = checkable_image(&snapshot)?;
            let Some(digests) = &self.image_digests else {
                return Ok(ServiceUpdateOutcome::Noop("Image scanner is unavailable."));
            };
            let Some(digest) = digests
                .cached_digest(snapshot.platform_id, registry, reference, cancellation)
                .await?
            else {
                return Ok(ServiceUpdateOutcome::Noop(
                    "No recent registry observation is available.",
                ));
            };
            Some(digest)
        } else {
            None
        };
        let checked = self
            .check_updates_snapshot(actor, true, snapshot.clone(), cached, cancellation)
            .await?;
        if checked.spec != snapshot.spec
            || checked.platform_id != snapshot.platform_id
            || checked.applied_image_digest != snapshot.applied_image_digest
        {
            return Err(SwarmServiceError::Conflict(
                "The Service configuration changed during the update check.".into(),
            ));
        }
        if checked.auto_update_state.status != "UpdateAvailable" {
            return Ok(ServiceUpdateOutcome::Noop("Service image is up to date."));
        }
        if checked.spec.update_behavior == UpdateBehavior::Notify {
            return Ok(ServiceUpdateOutcome::UpdateAvailable);
        }
        let Some(entitlements) = &self.entitlements else {
            return Ok(ServiceUpdateOutcome::Noop(
                "Automated operations require an active license entitlement.",
            ));
        };
        if !entitlements
            .enabled(LicenseCapability::AutomatedOperations)
            .await?
        {
            return Ok(ServiceUpdateOutcome::Noop(
                "Automated operations require an active license entitlement.",
            ));
        }
        // Re-evaluate before mutation; a license can change while registry I/O runs.
        if !entitlements
            .enabled(LicenseCapability::AutomatedOperations)
            .await?
            || !entitlements
                .enabled(LicenseCapability::OperationalGuardrails)
                .await?
        {
            return Ok(ServiceUpdateOutcome::Noop(
                "Automatic Apply is paused by license.",
            ));
        }
        let mut progress = self.run_requested_operation(
            actor,
            true,
            ServiceOperationRequest {
                id: checked.id,
                kind: ServiceOperationKind::Apply,
                replicas: None,
                expected_version: Some(checked.row_version),
            },
        );
        // Prepared is emitted only after the operation is durably claimed. The
        // existing recovery worker owns interrupted operations after acceptance.
        match progress.recv().await {
            Some(item) if item.stage == "Prepared" => Ok(ServiceUpdateOutcome::ApplyStarted),
            _ => Err(SwarmServiceError::Conflict(
                "Automatic Apply could not start; the Service is busy or changed.".into(),
            )),
        }
    }

    pub fn with_image_digests(mut self, digests: Arc<dyn ServiceImageDigestPort>) -> Self {
        self.image_digests = Some(digests);
        self
    }

    pub async fn check_updates(
        &self,
        actor: ActorId,
        administrator: bool,
        id: Uuid,
        cancellation: &CancellationToken,
    ) -> Result<SwarmServiceDetails, SwarmServiceError> {
        let snapshot = self.store.get_authorized(actor, administrator, id).await?;
        self.check_updates_snapshot(actor, administrator, snapshot, None, cancellation)
            .await
    }

    async fn check_updates_snapshot(
        &self,
        actor: ActorId,
        administrator: bool,
        snapshot: SwarmServiceDetails,
        cached_digest: Option<String>,
        cancellation: &CancellationToken,
    ) -> Result<SwarmServiceDetails, SwarmServiceError> {
        let id = snapshot.id;
        let digests = self.image_digests.as_ref().ok_or_else(|| {
            SwarmServiceError::Runtime("Image update checking is unavailable.".into())
        })?;
        let permit = Arc::clone(&self.slots).try_acquire_owned().map_err(|_| {
            SwarmServiceError::Conflict("Service operations are busy. Try again shortly.".into())
        })?;
        let (registry, reference) = checkable_image(&snapshot)?;
        let reference = reference.to_owned();
        // Claim and execution are owned by this bounded task, not the HTTP
        // future. Dropping a client must still release its database lease.
        let service = self.clone();
        let cancel = cancellation.clone();
        let digests = Arc::clone(digests);
        let task = self.spawn_result("swarm_service.update_check", async move {
            let _permit = permit;
            let claim = service
                .store
                .begin_update_check(actor, administrator, &snapshot)
                .await?;
            service.notifier.changed(id, "update");
            let scan = if let Some(digest) = cached_digest {
                Ok(digest)
            } else {
                tokio::select! {
                    biased;
                    () = cancel.cancelled() => Err(SwarmServiceError::Cancelled),
                    () = service.shutdown.cancelled() => Err(SwarmServiceError::Cancelled),
                    result = tokio::time::timeout(Duration::from_secs(30), digests.digest(snapshot.platform_id, registry, &reference, &cancel)) =>
                        result.unwrap_or_else(|_| Err(SwarmServiceError::Runtime("Registry update check timed out.".into()))),
                }
            };
            let update = match &scan {
                Ok(remote) => Some(evaluate_digest(snapshot.applied_image_digest.as_deref().unwrap(), remote)),
                Err(SwarmServiceError::Runtime(_)) => Some(AutoUpdateState {
                    last_checked_at: Utc::now(), status: "Failed".into(), current_digest: snapshot.applied_image_digest.clone(),
                    remote_digest: snapshot.auto_update_state.remote_digest.clone(),
                    last_error: Some("Registry update check failed. Verify registry connectivity and credentials.".into()),
                }),
                Err(_) => None,
            };
            tokio::time::timeout(
                Duration::from_secs(5),
                service.store.complete_update_check(&claim, update.as_ref()),
            )
            .await
            .map_err(|_| {
                SwarmServiceError::Storage(
                    "Update check cleanup timed out; recovery will release the lease.".into(),
                )
            })??;
            service.notifier.changed(id, "update");
            scan.map_err(|error| match error {
                SwarmServiceError::Runtime(_) => SwarmServiceError::Runtime(
                    "Registry update check failed. Verify registry connectivity and credentials."
                        .into(),
                ),
                other => other,
            })?;
            service.store.get_authorized(actor, administrator, id).await
        }).ok_or(SwarmServiceError::Cancelled)?;
        task.await
            .map_err(|_| SwarmServiceError::Runtime("Image update check was interrupted.".into()))?
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn digest_evaluation_matches_content_and_does_not_replace_the_applied_baseline() {
        let equal = evaluate_digest("registry/app@sha256:ABC", "sha256:abc");
        assert_eq!(equal.status, "UpToDate");
        let changed = evaluate_digest("sha256:old", "sha256:new");
        assert_eq!(changed.status, "UpdateAvailable");
        assert_eq!(changed.current_digest.as_deref(), Some("sha256:old"));
        assert_eq!(changed.remote_digest.as_deref(), Some("sha256:new"));
    }
}
