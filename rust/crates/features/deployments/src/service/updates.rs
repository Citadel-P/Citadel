//! Image checks keep their own lease; they cannot be mistaken for a failed Apply.
use super::*;
use crate::UpdateBehavior;
use chrono::Utc;
use citadel_primitives::AuthorizedResource;
use citadel_primitives::{AutoUpdateState, AutoUpdateStatus};

pub struct DeploymentUpdateCheck {
    pub lease_id: Uuid,
    pub deployment: crate::Deployment,
}

pub fn checkable_deployment_image(
    deployment: &crate::Deployment,
) -> Result<(Uuid, &str, &str), DeploymentError> {
    if deployment.control_state != citadel_primitives::ResourceControlState::Idle {
        return Err(DeploymentError::Conflict(
            "The Deployment is processing another operation.".into(),
        ));
    }
    let DeploymentImageInfo::External {
        registry_id,
        image_tag,
        resolved_digest,
    } = &deployment.spec.image
    else {
        return Err(DeploymentError::Validation(
            "Only external tagged images support update checks.".into(),
        ));
    };
    if registry_id.is_nil() || image_tag.trim().is_empty() || image_tag.contains('@') {
        return Err(DeploymentError::Validation(
            "Only external tagged images support update checks.".into(),
        ));
    }
    let current = resolved_digest
        .as_deref()
        .filter(|digest| !digest.trim().is_empty())
        .ok_or_else(|| {
            DeploymentError::Conflict(
                "The Deployment has no applied image digest to compare.".into(),
            )
        })?;
    Ok((*registry_id, image_tag, current))
}

impl DeploymentService {
    pub async fn check_updates(
        &self,
        actor: ActorId,
        administrator: bool,
        id: Uuid,
        cancellation: &CancellationToken,
    ) -> Result<AuthorizedResource<crate::Deployment>, DeploymentError> {
        let snapshot = self.store.get_authorized(actor, administrator, id).await?;
        self.check_update_snapshot(actor, administrator, snapshot, None, cancellation)
            .await
    }

    async fn check_update_snapshot(
        &self,
        actor: ActorId,
        administrator: bool,
        snapshot: AuthorizedResource<crate::Deployment>,
        cached_digest: Option<String>,
        cancellation: &CancellationToken,
    ) -> Result<AuthorizedResource<crate::Deployment>, DeploymentError> {
        let (registry, reference, current) = checkable_deployment_image(&snapshot)?;
        let reference = reference.to_owned();
        let current = current.to_owned();
        let permit = self
            .apply_slots
            .clone()
            .try_acquire_owned()
            .map_err(|_| DeploymentError::Conflict("Deployment operations are busy.".into()))?;
        let service = self.clone();
        let cancel = cancellation.clone();
        self.spawn_result("deployment.update_check", async move {
            let _permit = permit;
            let claim = service.store.begin_update_check(actor, administrator, &snapshot).await?;
            service.notifier.changed(snapshot.id, "updated");
            let scan = if let Some(digest) = cached_digest { Ok(digest) } else { tokio::select! {
                biased;
                () = cancel.cancelled() => Err(DeploymentError::Cancelled),
                () = service.shutdown.cancelled() => Err(DeploymentError::Cancelled),
                result = tokio::time::timeout(Duration::from_secs(30), service.runtime.remote_image_digest(snapshot.platform_id, registry, &reference, &cancel)) =>
                    result.unwrap_or_else(|_| Err(DeploymentError::Runtime("Registry update check timed out.".into()))),
            } };
            let update = match &scan {
                Ok(remote) => Some(AutoUpdateState::checked(&current, remote, Utc::now())),
                Err(DeploymentError::Cancelled) => None,
                Err(_) => Some(AutoUpdateState {
                    last_checked_at: Utc::now(), status: AutoUpdateStatus::Failed, current_digest: Some(current),
                    remote_digest: snapshot.auto_update_state.as_ref().and_then(|state| state.remote_digest.clone()),
                    last_error: Some("Registry update check failed. Verify connectivity and credentials.".into()),
                }),
            };
            tokio::time::timeout(Duration::from_secs(5), service.store.complete_update_check(&claim, update.as_ref())).await
                .map_err(|_| DeploymentError::Runtime("Update check cleanup timed out; recovery will release the lease.".into()))??;
            service.notifier.changed(snapshot.id, "updated");
            scan.map_err(|error| if matches!(error, DeploymentError::Cancelled) { error } else {
                DeploymentError::Runtime("Registry update check failed. Verify connectivity and credentials.".into())
            })?;
            service.store.get_authorized(actor, administrator, snapshot.id).await
        }).ok_or(DeploymentError::Cancelled)?.await.map_err(|_| DeploymentError::Runtime("Update check was interrupted.".into()))?
    }

    pub async fn recover_update_checks(&self) -> Result<usize, DeploymentError> {
        let recovered = self
            .store
            .recover_update_checks(Utc::now().timestamp() - 60, 25)
            .await?;
        for id in &recovered {
            self.notifier.changed(*id, "updated");
        }
        Ok(recovered.len())
    }

    /// Bounded keyset sweep, shared with the on-demand evaluator. No overlapping
    /// mutation is retried; Apply's existing claim and reconciliation own it.
    pub async fn run_scheduled_update_checks(
        &self,
        cancellation: &CancellationToken,
    ) -> Result<(), DeploymentError> {
        // Detection/Notify is available in Community. Gate only automatic Apply.
        let actor = ActorId::new(Uuid::from_u128(1));
        let mut after = Uuid::nil();
        loop {
            let batch = self.store.scheduled_update_candidates(after, 25).await?;
            if batch.is_empty() {
                break;
            }
            for id in batch {
                after = id;
                if cancellation.is_cancelled() || self.shutdown.is_cancelled() {
                    return Ok(());
                }
                let result = async {
                    let before = self.store.get_authorized(actor, true, id).await?;
                    let (registry, reference, _) = checkable_deployment_image(&before)?;
                    let Some(digest) = self
                        .runtime
                        .cached_image_digest(before.platform_id, registry, reference, cancellation)
                        .await?
                    else {
                        return Ok(());
                    };
                    let checked = self
                        .check_update_snapshot(
                            actor,
                            true,
                            before.clone(),
                            Some(digest),
                            cancellation,
                        )
                        .await?;
                    if checked.spec != before.spec || checked.platform_id != before.platform_id {
                        return Ok(());
                    }
                    if !checked
                        .auto_update_state
                        .as_ref()
                        .is_some_and(|state| state.status == AutoUpdateStatus::UpdateAvailable)
                    {
                        return Ok(());
                    }
                    if checked.spec.update_behavior != UpdateBehavior::AutoDeploy
                        || !self
                            .entitlements
                            .enabled(LicenseCapability::AutomatedOperations)
                            .await?
                        || !self
                            .entitlements
                            .enabled(LicenseCapability::OperationalGuardrails)
                            .await?
                    {
                        self.report_image_update(&checked, "DeploymentImageUpdateAvailable")
                            .await;
                        return Ok(());
                    }
                    let applied = async {
                        let mut output = self
                            .apply_versioned(actor, true, id, false, Some(checked.row_version))
                            .await?;
                        while let Some(item) = output.recv().await {
                            if item.error.is_some() {
                                return Ok(false);
                            }
                        }
                        let after = self.store.get_authorized(actor, true, id).await?;
                        Ok::<_, DeploymentError>(
                            after.status == crate::DeploymentStatus::Healthy
                                && after.auto_update_state.as_ref().is_some_and(|state| {
                                    state.status == AutoUpdateStatus::UpToDate
                                }),
                        )
                    }
                    .await;
                    self.report_image_update(
                        &checked,
                        if matches!(applied, Ok(true)) {
                            "DeploymentAutoUpdated"
                        } else {
                            "DeploymentAutoDeployFailed"
                        },
                    )
                    .await;
                    applied?;
                    Ok::<_, DeploymentError>(())
                }
                .await;
                if let Err(error) = result {
                    tracing::warn!(%id, %error, "Deployment image update check failed");
                }
            }
        }
        Ok(())
    }

    async fn report_image_update(&self, deployment: &crate::Deployment, kind: &str) {
        let Some(alerts) = &self.alerts else {
            return;
        };
        let Some(state) = &deployment.auto_update_state else {
            return;
        };
        let reason = "Automatic Deployment Apply did not complete. Review the Deployment activity.";
        let observation = AlertObservation {
            alert_type: kind.into(),
            resource_id: deployment.id,
            resource_name: deployment.name.clone(),
            resource_type: "Deployment".into(),
            deduplication_component: state.remote_digest.clone().unwrap_or_default(),
            observed_at: Utc::now(),
            value: None,
            matched: true,
            info: serde_json::json!({ "DeploymentName":deployment.name,"CurrentImage":state.current_digest,"PreviousImage":state.current_digest,
                "LatestImage":state.remote_digest,"UpdatedImage":state.remote_digest,"Reason":reason,
                "HumanMessage": if kind.ends_with("Failed") { reason } else if kind.ends_with("Available") { "A Deployment image update is available." } else { "The Deployment was automatically updated." } }),
        };
        if let Err(error) = alerts.observe(&observation).await {
            tracing::warn!(%error,deployment_id=%deployment.id,"Deployment update Alert evaluation failed");
        }
    }
}
