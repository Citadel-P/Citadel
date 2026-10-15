use super::*;
use crate::StackUpdateBehavior;
use crate::StackUpdateState;
use citadel_primitives::AuthorizedResource;
use sha2::Digest;

#[derive(Clone, Copy)]
enum CheckMode {
    Manual,
    Git,
    Images,
}

pub trait StackUpdateScanner: Send + Sync {
    /// Read deployed images and current registry digests without checking Git.
    fn scan_images<'a>(
        &'a self,
        stack: &'a crate::Stack,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<StackUpdateState, StackError>>;
    fn scan_cached<'a>(
        &'a self,
        stack: &'a crate::Stack,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<StackUpdateState, StackError>> {
        self.scan(stack, cancellation)
    }
    fn scan_images_cached<'a>(
        &'a self,
        stack: &'a crate::Stack,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<StackUpdateState, StackError>> {
        self.scan_cached(stack, cancellation)
    }
    fn scan<'a>(
        &'a self,
        stack: &'a crate::Stack,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<StackUpdateState, StackError>>;
}
impl StackService {
    pub fn with_update_scanner(mut self, scanner: Arc<dyn StackUpdateScanner>) -> Self {
        self.update_scanner = Some(scanner);
        self
    }

    pub async fn check_updates(
        &self,
        actor: ActorId,
        administrator: bool,
        id: Uuid,
        cancellation: &CancellationToken,
    ) -> Result<AuthorizedResource<crate::Stack>, StackError> {
        self.check_updates_mode(actor, administrator, id, CheckMode::Manual, cancellation)
            .await
    }
    async fn check_updates_mode(
        &self,
        actor: ActorId,
        administrator: bool,
        id: Uuid,
        mode: CheckMode,
        cancellation: &CancellationToken,
    ) -> Result<AuthorizedResource<crate::Stack>, StackError> {
        let _permit = self
            .operations
            .clone()
            .try_acquire_owned()
            .map_err(|_| StackError::Conflict("Stack operations are busy.".into()))?;
        let snapshot = self.store.get_authorized(actor, administrator, id).await?;
        // Authorize before admission. Hold the key through the final CAS, not
        // just through the external scan; no persistent result is cached here.
        let _check = self.update_checks.try_enter(id).ok_or_else(|| {
            self.notifier.update_check_duplicate();
            StackError::Conflict("A Stack update check is already running.".into())
        })?;
        if snapshot.control_state != citadel_primitives::ResourceControlState::Idle
            || !matches!(
                snapshot.status,
                StackReleaseStatus::Healthy
                    | StackReleaseStatus::Degraded
                    | StackReleaseStatus::Stopped
                    | StackReleaseStatus::Paused
            )
        {
            return Err(StackError::Conflict(
                "The Stack release state does not support update checks.".into(),
            ));
        }
        let scanner = self
            .update_scanner
            .as_ref()
            .ok_or_else(|| StackError::Runtime("Stack update checks are unavailable.".into()))?;
        let cancel = cancellation.child_token();
        let _guard = cancel.clone().drop_guard();
        let scan = match mode {
            CheckMode::Manual => scanner.scan(&snapshot, &cancel),
            CheckMode::Git => scanner.scan_cached(&snapshot, &cancel),
            CheckMode::Images => scanner.scan_images_cached(&snapshot, &cancel),
        };
        let next = tokio::select! {
            ()=cancel.cancelled()=>return Err(StackError::Cancelled),
            ()=self.shutdown.cancelled()=>return Err(StackError::Cancelled),
            result=tokio::time::timeout(Duration::from_secs(60),scan)=>
                result.map_err(|_|StackError::Runtime("Stack update check timed out.".into()))??,
        };
        // The scan only reads Docker/Git. Compare-and-swap at commit rejects a
        // concurrent Apply/edit; no durable processing lease needs crash recovery.
        self.store
            .save_update_check(actor, administrator, &snapshot, &next)
            .await?;
        self.notifier.changed(id, "updated");
        let mut checked = snapshot;
        checked.resource.stack_update_state = next;
        checked.resource.row_version += 1;
        Ok(checked)
    }

    pub async fn run_update_checks(
        &self,
        images: bool,
        cancellation: &CancellationToken,
    ) -> Result<(), StackError> {
        self.run_update_check_pass(false, cancellation).await?;
        if images {
            self.run_update_check_pass(true, cancellation).await?;
        }
        Ok(())
    }

    async fn run_update_check_pass(
        &self,
        images: bool,
        cancellation: &CancellationToken,
    ) -> Result<(), StackError> {
        // Detection/Notify is available in Community. Gate only automatic Apply.
        let actor = ActorId::new(Uuid::from_u128(1));
        let mut after = Uuid::nil();
        loop {
            let ids = self
                .store
                .update_check_candidates(after, images, 25)
                .await?;
            if ids.is_empty() {
                break;
            }
            for id in ids {
                after = id;
                if cancellation.is_cancelled() || self.shutdown.is_cancelled() {
                    return Ok(());
                }
                let result = async {
                    let checked = self
                        .check_updates_mode(actor, true, id, if images { CheckMode::Images } else { CheckMode::Git }, cancellation)
                        .await?;
                    let behavior = match checked.spec.as_ref().ok_or(StackError::NotFound)? {
                        StackSpec::Git {
                            update_behavior, ..
                        }
                        | StackSpec::WebEditor {
                            update_behavior, ..
                        } => *update_behavior,
                    };
                    let available = if images {
                        let (StackUpdateState::Git { recreate_stack_on_new_image_state, .. }
                            | StackUpdateState::WebEditor { recreate_stack_on_new_image_state }) = &checked.stack_update_state;
                        recreate_stack_on_new_image_state.auto_update_states.iter().any(|image| image.update_available)
                    } else {
                        matches!(&checked.stack_update_state, StackUpdateState::Git { recreate_stack_on_new_commit_state: commit, .. }
                            if commit.remote_commit_sha.as_deref().is_some_and(|remote| !remote.eq_ignore_ascii_case(&commit.current_commit_sha)))
                    };
                    if !available {
                        return Ok(());
                    }
                    let git = !images;
                    let service_only_supported = checked.spec.as_ref().is_some_and(|spec| {
                        spec.supports_service_auto_deploy(checked.platform_type)
                    });
                    let auto = (behavior != StackUpdateBehavior::ServiceAutoDeploy
                        || service_only_supported)
                        && matches!(
                            behavior,
                            StackUpdateBehavior::StackAutoDeploy
                                | StackUpdateBehavior::ServiceAutoDeploy
                        )
                        && self.automated_operations_enabled().await?
                        && match &self.entitlements {
                            Some(entitlements) => entitlements.operational_guardrails().await?,
                            None => false,
                        };
                    if !auto {
                        self.report_update(
                            &checked,
                            if git {
                                "StackGitUpdateAvailable"
                            } else {
                                "StackImageUpdateAvailable"
                            },
                            None,
                        )
                        .await;
                        return Ok(());
                    }
                    let service_names =
                        if behavior == StackUpdateBehavior::ServiceAutoDeploy {
                            match &checked.stack_update_state {
                                StackUpdateState::WebEditor {
                                    recreate_stack_on_new_image_state,
                                } => recreate_stack_on_new_image_state
                                    .auto_update_states
                                    .iter()
                                    .filter(|image| image.update_available)
                                    .map(|image| image.service_name.clone())
                                    .collect(),
                                _ => return Err(StackError::Validation(
                                    "Service-only automatic updates require a Web Editor Stack."
                                        .into(),
                                )),
                            }
                        } else {
                            Vec::new()
                        };
                    let applied = async {
                        let mut output = self
                            .start_apply_versioned(
                                actor,
                                true,
                                id,
                                None,
                                None,
                                crate::StackApplyOptions {
                                    expected_version: Some(checked.row_version),
                                    service_names,
                                },
                                if images { checked.source.as_ref().map(|source| source.resolved_commit_sha.clone()).filter(|sha| !sha.is_empty()) } else { None },
                            )
                            .await?;
                        let mut success = false;
                        while let Some(item) = output.recv().await {
                            if item.exit_code.is_some_and(|code| code != 0) {
                                return Ok(false);
                            }
                            success |= item.exit_code == Some(0);
                        }
                        Ok::<_, StackError>(success)
                    }
                    .await;
                    let success = matches!(applied, Ok(true));
                    let kind = match (
                        git,
                        behavior == StackUpdateBehavior::ServiceAutoDeploy,
                        success,
                    ) {
                        (true, _, true) => "StackGitAutoUpdated",
                        (true, _, false) => "StackGitAutoDeployFailed",
                        (_, true, true) => "StackServiceAutoUpdated",
                        (_, true, false) => "StackServiceAutoDeployFailed",
                        (_, _, true) => "StackAutoUpdated",
                        _ => "StackAutoDeployFailed",
                    };
                    // The branch may advance between the check and Apply. Report
                    // the committed release, not the earlier candidate commit.
                    let applied_commit = if success && git {
                        self.store
                            .get_authorized(actor, true, id)
                            .await
                            .ok()
                            .filter(|view| {
                                view.current_stack_release_id == checked.current_stack_release_id
                            })
                            .and_then(|view| {
                                view.resource
                                    .source
                                    .map(|source| source.resolved_commit_sha)
                            })
                    } else {
                        None
                    };
                    self.report_update(&checked, kind, applied_commit.as_deref())
                        .await;
                    applied?;
                    Ok::<_, StackError>(())
                }
                .await;
                if let Err(error) = result {
                    tracing::warn!(%id,%error,"Stack update check failed");
                }
            }
        }
        Ok(())
    }

    async fn report_update(&self, stack: &crate::Stack, kind: &str, applied_commit: Option<&str>) {
        let Some(alerts) = &self.alerts else {
            return;
        };
        let state = &stack.stack_update_state;
        let images = match state {
            StackUpdateState::Git {
                recreate_stack_on_new_image_state,
                ..
            }
            | StackUpdateState::WebEditor {
                recreate_stack_on_new_image_state,
            } => &recreate_stack_on_new_image_state.auto_update_states,
        };
        let identity = if kind.starts_with("StackGit") {
            match state {
                StackUpdateState::Git {
                    recreate_stack_on_new_commit_state: commit,
                    ..
                } => commit.remote_commit_sha.clone().unwrap_or_default(),
                _ => String::new(),
            }
        } else {
            images
                .iter()
                .filter(|image| image.update_available)
                .map(|image| {
                    format!(
                        "{}:{}",
                        image.service_name,
                        image.remote_digest.as_deref().unwrap_or_default()
                    )
                })
                .collect::<Vec<_>>()
                .join("|")
        };
        let updates = images.iter().filter(|image| image.update_available).map(|image|serde_json::json!({
            "ServiceName":image.service_name,"ImageName":image.image_name,"CurrentDigest":image.current_digest,"LatestDigest":image.remote_digest,
        })).collect::<Vec<_>>();
        let (current, remote) = match state {
            StackUpdateState::Git {
                recreate_stack_on_new_commit_state: commit,
                ..
            } => (
                commit.current_commit_sha.as_str(),
                commit.remote_commit_sha.as_deref().unwrap_or_default(),
            ),
            _ => ("", ""),
        };
        let branch = match &stack.spec {
            Some(StackSpec::Git { branch, .. }) => branch.as_str(),
            _ => "",
        };
        let reason = "Automatic Stack Apply did not complete. Review the Stack activity.";
        let observation = AlertObservation {
            alert_type: kind.into(),
            resource_id: stack.id,
            resource_name: stack.name.clone(),
            resource_type: "Stack".into(),
            deduplication_component: sha2::Sha256::digest(
                applied_commit.unwrap_or(&identity).as_bytes(),
            )
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect(),
            observed_at: chrono::Utc::now(),
            value: None,
            matched: true,
            info: serde_json::json!({ "StackId":stack.id,"StackName":stack.name,"Updates":updates,
                "ServiceNames":images.iter().filter(|image|image.update_available).map(|image|&image.service_name).collect::<Vec<_>>(),
                "Branch":branch,"GitRepositoryName":stack.source.as_ref().and_then(|source|source.git_repository_name.as_deref()).unwrap_or_default(),
                "CurrentCommitSha":current,"PreviousCommitSha":current,"RemoteCommitSha":remote,"UpdatedCommitSha":applied_commit,"Reason":reason,
                "HumanMessage": if kind.ends_with("Failed") { reason }
                    else if kind.ends_with("Available") { "A Stack update is available." } else { "The Stack was automatically updated." } }),
        };
        if let Err(error) = alerts.observe(&observation).await {
            tracing::warn!(%error,stack_id=%stack.id,"Stack update Alert evaluation failed");
        }
    }
}

/// Refresh an observation only after the durable Apply outcome has committed.
/// A failed scan must not turn a successful deployment into a failed operation.
pub(super) async fn refresh_applied_images(
    store: &dyn StackRepository,
    scanner: Option<&dyn StackUpdateScanner>,
    actor: ActorId,
    administrator: bool,
    claim: &StackOperationClaim,
    cancellation: &CancellationToken,
) -> Result<(), StackError> {
    let Some(scanner) = scanner else {
        return Ok(());
    };
    let cancel = cancellation.child_token();
    let _guard = cancel.clone().drop_guard();
    let refresh = async {
        let snapshot = store
            .get_authorized(actor, administrator, claim.stack_id)
            .await?;
        if snapshot.current_stack_release_id != claim.release_id
            || snapshot.row_version != claim.row_version + 1
            || snapshot.control_state != citadel_primitives::ResourceControlState::Idle
        {
            return Err(StackError::Conflict(
                "The Stack changed after Apply.".into(),
            ));
        }
        if !snapshot
            .spec
            .as_ref()
            .is_some_and(|spec| spec.common().registry_id.is_some_and(|id| !id.is_nil()))
        {
            return Ok(());
        }
        // Compare against observed runtime digests, never assume the previously
        // advertised remote digest is what Docker actually deployed.
        let next = scanner.scan_images(&snapshot, &cancel).await?;
        store
            .save_update_check(actor, administrator, &snapshot, &next)
            .await
    };
    tokio::select! {
        () = cancel.cancelled() => Err(StackError::Cancelled),
        result = tokio::time::timeout(Duration::from_secs(60), refresh) =>
            result.map_err(|_| StackError::Runtime("Post-Apply image update check timed out.".into()))?,
    }
}
