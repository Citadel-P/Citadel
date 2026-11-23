use super::*;
use citadel_primitives::AuthorizedResource;
impl StackService {
    pub(super) async fn operational_guardrails_enabled(&self) -> Result<bool, StackError> {
        match &self.entitlements {
            Some(entitlements) => entitlements.operational_guardrails().await,
            None => Ok(false),
        }
    }

    pub(super) async fn monitor_candidate(
        &self,
        stack: &crate::StackDriftContext,
        actor: ActorId,
        can_auto_fix: bool,
    ) -> Result<(StackDriftReport, bool), StackError> {
        let report = self.inspect_drift(stack, actor).await?;
        self.finish_drift_observation(stack, actor, can_auto_fix, report)
            .await
    }

    pub(super) async fn finish_drift_observation(
        &self,
        stack: &crate::StackDriftContext,
        actor: ActorId,
        can_auto_fix: bool,
        report: StackDriftReport,
    ) -> Result<(StackDriftReport, bool), StackError> {
        if report.has_drift
            && stack.drift_policy.mode == crate::StackDriftMode::AutoFix
            && can_auto_fix
        {
            let repair = self
                .reconcile_drift_with_mode(actor, true, stack.id, false)
                .await?;
            return Ok((
                repair.after_report.unwrap_or(repair.before_report),
                !repair.actions.is_empty(),
            ));
        }
        self.observe_drift(stack, &report).await;
        Ok((report, false))
    }

    pub async fn drift(
        &self,
        actor: ActorId,
        administrator: bool,
        id: Uuid,
    ) -> Result<StackDriftReport, StackError> {
        let stack = self.store.get_authorized(actor, administrator, id).await?;
        let context = stack.resource.into();
        let report = self.inspect_drift(&context, actor).await?;
        self.observe_drift(&context, &report).await;
        Ok(report)
    }

    async fn inspect_drift(
        &self,
        stack: &crate::StackDriftContext,
        actor: ActorId,
    ) -> Result<StackDriftReport, StackError> {
        let platform_id = stack.platform_id.ok_or(StackError::NotFound)?;
        let project = drift_project_name(stack.id, &stack.name, stack.spec.as_ref());
        let runtime = self
            .runtime
            .runtime_snapshot(
                platform_id,
                &project,
                orchestration(&stack.platform_type),
                &self.shutdown.child_token(),
            )
            .await?;
        if matches!(stack.spec, Some(StackSpec::Git { .. }))
            && stack.drift_policy.mode != crate::StackDriftMode::Disabled
            && !matches!(
                stack.status,
                StackReleaseStatus::Stopped | StackReleaseStatus::Paused
            )
        {
            let claim = drift_source_claim(stack, actor)?;
            let source = self.source_materializer.as_ref().ok_or_else(|| {
                StackError::Runtime("Git Stack source reading is unavailable.".into())
            })?;
            let files = source
                .compose_contents(&claim, &self.shutdown.child_token())
                .await?;
            compare_compose(stack.id, platform_id, &stack.drift_policy, &runtime, &files)
        } else {
            calculate_spec_drift(
                stack.id,
                stack.platform_id,
                stack.status,
                &stack.drift_policy,
                stack.spec.as_ref(),
                &runtime,
            )
        }
    }

    async fn observe_drift(&self, stack: &crate::StackDriftContext, report: &StackDriftReport) {
        if stack.drift_policy.alert_on_drift
            && let Some(alerts) = &self.alerts
        {
            let observation = stack_drift_observation(stack, report, "StackDriftDetected");
            if let Err(error) = alerts.observe(&observation).await {
                tracing::warn!(%error, stack_id=%stack.id, "Stack drift Alert evaluation failed");
            }
        }
    }

    pub async fn update_drift_policy(
        &self,
        actor: ActorId,
        administrator: bool,
        id: Uuid,
        policy: crate::StackDriftPolicy,
    ) -> Result<AuthorizedResource<crate::Stack>, StackError> {
        let current = self.store.get_authorized(actor, administrator, id).await?;
        let spec = current.spec.clone().ok_or(StackError::NotFound)?;
        let input = UpdateStack {
            name: None,
            platform_id: None,
            description: None,
            stack_source: None,
            spec: None,
            drift_policy: Some(policy.normalized()),
            row_version: Some(current.row_version),
        };
        let updated = self
            .store
            .update(actor, administrator, id, current.row_version, &input, &spec)
            .await?;
        self.notifier.changed(id, "driftPolicyUpdated");
        Ok(updated)
    }

    pub async fn reconcile_drift(
        &self,
        actor: ActorId,
        administrator: bool,
        id: Uuid,
    ) -> Result<crate::StackReconciliationResult, StackError> {
        self.reconcile_drift_with_mode(actor, administrator, id, true)
            .await
    }

    pub(super) async fn reconcile_drift_with_mode(
        &self,
        actor: ActorId,
        administrator: bool,
        id: Uuid,
        manual: bool,
    ) -> Result<crate::StackReconciliationResult, StackError> {
        let stack = self.store.get_authorized(actor, administrator, id).await?;
        if !self.operational_guardrails_enabled().await? {
            return Err(StackError::LicenseRequired("OperationalGuardrails"));
        }
        if stack.control_state == citadel_primitives::ResourceControlState::Processing
            || matches!(
                stack.status,
                StackReleaseStatus::Applying | StackReleaseStatus::Pending
            )
        {
            return Err(StackError::Conflict(
                "The Stack is currently applying and cannot be reconciled.".to_owned(),
            ));
        }
        if stack.drift_policy.mode == crate::StackDriftMode::Disabled {
            return Err(validation("Stack drift detection is disabled."));
        }
        if !manual && stack.drift_policy.mode != crate::StackDriftMode::AutoFix {
            return Err(validation("Stack drift auto-fix is not enabled."));
        }
        let before = self.drift(actor, administrator, id).await?;
        if !before.has_drift {
            return Ok(crate::StackReconciliationResult {
                stack_id: id,
                status: crate::StackReconciliationStatus::NoDrift,
                before_report: before,
                after_report: None,
                actions: Vec::new(),
            });
        }
        if before.has_structural_drift {
            return Ok(crate::StackReconciliationResult {
                stack_id: id,
                status: crate::StackReconciliationStatus::RequiresReapply,
                before_report: before,
                after_report: None,
                actions: Vec::new(),
            });
        }
        // Explicit manual repair does not change the saved automatic policy.
        // Destructive cleanup always remains opt-in.
        let mut policy = stack.drift_policy.clone();
        if manual {
            policy.auto_start_stopped_containers = true;
            policy.auto_resume_paused_containers = true;
        }
        let actions = self
            .runtime
            .reconcile(
                stack.platform_id.ok_or(StackError::NotFound)?,
                &before.drifts,
                &policy,
                &self.shutdown.child_token(),
            )
            .await?;
        let after = if actions.is_empty() {
            None
        } else {
            Some(self.drift(actor, administrator, id).await?)
        };
        let status = if after.as_ref().is_some_and(|report| !report.has_drift)
            && actions.iter().all(|action| action.succeeded)
        {
            crate::StackReconciliationStatus::Reconciled
        } else if after
            .as_ref()
            .map_or(before.has_structural_drift, |report| {
                report.has_structural_drift
            })
        {
            crate::StackReconciliationStatus::RequiresReapply
        } else {
            crate::StackReconciliationStatus::Partial
        };
        self.notifier.changed(id, "reconciled");
        if status == crate::StackReconciliationStatus::Reconciled
            && let Some(alerts) = &self.alerts
        {
            let observation = AlertObservation {
                alert_type: "StackDriftAutoReconciled".to_owned(),
                info: serde_json::json!({
                    "StackId": stack.id,
                    "StackName": &stack.name,
                    "ActionCount": actions.len(),
                    "HumanMessage": format!("Stack '{}' drift was reconciled.", stack.name),
                }),
                resource_id: stack.id,
                resource_name: stack.name.clone(),
                resource_type: "Stack".to_owned(),
                deduplication_component: "drift".to_owned(),
                observed_at: chrono::Utc::now(),
                value: None,
                matched: true,
            };
            if let Err(error) = alerts.observe(&observation).await {
                tracing::warn!(%error, stack_id=%stack.id, "Stack reconciliation Alert evaluation failed");
            }
        }
        Ok(crate::StackReconciliationResult {
            stack_id: id,
            status,
            before_report: before,
            after_report: after,
            actions,
        })
    }
}

pub(super) fn stable_runtime_status(snapshot: &StackRuntimeSnapshot) -> Option<StackReleaseStatus> {
    if snapshot.containers.is_empty() {
        return None;
    }
    if snapshot
        .containers
        .iter()
        .all(|container| container.state.eq_ignore_ascii_case("paused"))
    {
        return Some(StackReleaseStatus::Paused);
    }
    if snapshot.containers.iter().all(|container| {
        container.state.eq_ignore_ascii_case("exited")
            || container.state.eq_ignore_ascii_case("offline")
            || container.state.eq_ignore_ascii_case("stopped")
    }) {
        return Some(StackReleaseStatus::Stopped);
    }
    if snapshot.containers.iter().all(|container| {
        container.state.eq_ignore_ascii_case("running")
            && !container
                .health
                .as_deref()
                .is_some_and(|health| health.eq_ignore_ascii_case("unhealthy"))
    }) {
        return Some(StackReleaseStatus::Healthy);
    }
    None
}

pub(super) fn stack_drift_observation(
    stack: &crate::StackDriftContext,
    report: &StackDriftReport,
    alert_type: &str,
) -> AlertObservation {
    AlertObservation {
        alert_type: alert_type.to_owned(),
        info: serde_json::json!({
            "StackId": stack.id,
            "StackName": stack.name,
            "HasStructuralDrift": report.has_structural_drift,
            "DriftCount": report.drifts.len(),
            "Drifts": report.drifts,
            "HumanMessage": if report.has_drift {
                format!("Stack '{}' has configuration drift.", stack.name)
            } else {
                format!("Stack '{}' no longer has configuration drift.", stack.name)
            },
        }),
        resource_id: stack.id,
        resource_name: stack.name.clone(),
        resource_type: "Stack".to_owned(),
        deduplication_component: "drift".to_owned(),
        observed_at: chrono::Utc::now(),
        value: None,
        matched: report.has_drift,
    }
}

pub(super) fn event_drift_eligible(stack: &crate::Stack) -> bool {
    stack.drift_policy.mode != crate::StackDriftMode::Disabled
        && stack.control_state == citadel_primitives::ResourceControlState::Idle
        && matches!(
            stack.status,
            StackReleaseStatus::Healthy | StackReleaseStatus::Degraded
        )
}

pub(super) fn drift_status_update(
    stack: &crate::StackDriftContext,
    report: &StackDriftReport,
) -> Option<(StackReleaseStatus, ActivityEventInfo)> {
    if stack.control_state != citadel_primitives::ResourceControlState::Idle
        || stack.platform_status != citadel_primitives::PlatformStatus::Online
        || stack.drift_policy.mode == crate::StackDriftMode::Disabled
        || !matches!(
            stack.status,
            StackReleaseStatus::Healthy | StackReleaseStatus::Degraded
        )
    {
        return None;
    }
    let previous = stack.previous_fingerprint.as_deref();
    if !report.has_drift {
        return previous
            .filter(|_| stack.status == StackReleaseStatus::Degraded)
            .map(|value| {
                (
                    StackReleaseStatus::Healthy,
                    ActivityEventInfo::StackDriftResolved {
                        previous_fingerprint: value.into(),
                    },
                )
            });
    }
    let fingerprint =
        compose_digest(&[serde_json::to_string(&report.drifts).expect("drift report serializes")]);
    let status = if stack.drift_policy.mark_degraded {
        StackReleaseStatus::Degraded
    } else {
        stack.status
    };
    if previous == Some(fingerprint.as_str()) && status == stack.status {
        return None;
    }
    Some((
        status,
        ActivityEventInfo::StackDriftDetected {
            reason: format!(
                "Stack runtime drift detected ({} items).",
                report.drifts.len()
            ),
            fingerprint,
        },
    ))
}

pub fn calculate_drift(
    stack: &crate::Stack,
    runtime: &StackRuntimeSnapshot,
) -> Result<StackDriftReport, StackError> {
    calculate_spec_drift(
        stack.id,
        stack.platform_id,
        stack.status,
        &stack.drift_policy,
        stack.spec.as_ref(),
        runtime,
    )
}

fn calculate_spec_drift(
    id: Uuid,
    platform: Option<Uuid>,
    status: StackReleaseStatus,
    policy: &crate::StackDriftPolicy,
    spec: Option<&StackSpec>,
    runtime: &StackRuntimeSnapshot,
) -> Result<StackDriftReport, StackError> {
    let platform_id = platform.ok_or(StackError::NotFound)?;
    if policy.mode == crate::StackDriftMode::Disabled
        || matches!(
            status,
            StackReleaseStatus::Stopped | StackReleaseStatus::Paused
        )
    {
        return Ok(StackDriftReport {
            stack_id: id,
            platform_id,
            has_drift: false,
            has_auto_fixable_drift: false,
            has_structural_drift: false,
            drifts: Vec::new(),
        });
    }
    let compose = spec
        .ok_or(StackError::NotFound)?
        .compose_file()
        .ok_or_else(|| validation("Git Stack drift requires materialized source."))?;
    compare_compose(id, platform_id, policy, runtime, &[compose.to_owned()])
}

pub(super) fn compare_compose(
    id: Uuid,
    platform_id: Uuid,
    policy: &crate::StackDriftPolicy,
    runtime: &StackRuntimeSnapshot,
    files: &[String],
) -> Result<StackDriftReport, StackError> {
    let names = super::drift_inputs::compile(files)?;
    Ok(compare_names(id, platform_id, policy, runtime, &names))
}

pub(super) fn compare_names(
    id: Uuid,
    platform_id: Uuid,
    policy: &crate::StackDriftPolicy,
    runtime: &StackRuntimeSnapshot,
    desired_names: &BTreeSet<String>,
) -> StackDriftReport {
    let actual_names = runtime
        .containers
        .iter()
        .map(|container| container.service_name.as_str())
        .chain(runtime.services.iter().map(|service| {
            service
                .name
                .rsplit_once('_')
                .map_or(service.name.as_str(), |(_, name)| name)
        }))
        .collect::<BTreeSet<_>>();
    let mut drifts = Vec::new();
    for missing in desired_names
        .iter()
        .filter(|name| !actual_names.contains(name.as_str()))
    {
        drifts.push(StackDrift::MissingContainer {
            service_name: (*missing).to_owned(),
        });
    }
    for extra in actual_names
        .iter()
        .filter(|name| !desired_names.contains(**name))
    {
        for container in runtime
            .containers
            .iter()
            .filter(|item| item.service_name == *extra)
        {
            drifts.push(StackDrift::ExtraContainer {
                container_id: container.docker_container_id.clone(),
                service_name: (*extra).to_owned(),
            });
        }
    }
    for container in runtime
        .containers
        .iter()
        .filter(|item| desired_names.contains(item.service_name.as_str()))
    {
        match container.state.to_ascii_lowercase().as_str() {
            "created" | "dead" | "exited" | "offline" => {
                drifts.push(StackDrift::ContainerStopped {
                    container_id: container.docker_container_id.clone(),
                    service_name: container.service_name.clone(),
                })
            }
            "paused" => drifts.push(StackDrift::ContainerPaused {
                container_id: container.docker_container_id.clone(),
                service_name: container.service_name.clone(),
            }),
            _ if container
                .health
                .as_deref()
                .is_some_and(|health| health.eq_ignore_ascii_case("unhealthy")) =>
            {
                drifts.push(StackDrift::ContainerUnhealthy {
                    container_id: container.docker_container_id.clone(),
                    service_name: container.service_name.clone(),
                    health_status: container.health.clone(),
                })
            }
            _ => {}
        }
    }
    let has_structural = drifts
        .iter()
        .any(|drift| matches!(drift, StackDrift::MissingContainer { .. }));
    let has_auto = drifts.iter().any(|drift| match drift {
        StackDrift::ContainerStopped { .. } => policy.auto_start_stopped_containers,
        StackDrift::ContainerPaused { .. } => policy.auto_resume_paused_containers,
        StackDrift::ExtraContainer { .. } => policy.remove_extra_containers,
        _ => false,
    });
    StackDriftReport {
        stack_id: id,
        platform_id,
        has_drift: !drifts.is_empty(),
        has_auto_fixable_drift: has_auto,
        has_structural_drift: has_structural,
        drifts,
    }
}

/// Never compare a deployed release against a moving branch or an unapplied pin.
pub(super) fn drift_source_claim(
    stack: &crate::StackDriftContext,
    actor: ActorId,
) -> Result<StackOperationClaim, StackError> {
    let mut spec = stack.spec.clone().ok_or(StackError::NotFound)?;
    let source = stack.deployed_commit.as_ref().filter(|commit| !commit.trim().is_empty())
        .ok_or_else(|| StackError::Conflict("The deployed Git commit is unavailable. Reapply the Stack to establish its drift baseline.".into()))?;
    let StackSpec::Git { commit_sha, .. } = &mut spec else {
        return Err(validation("A Git Stack is required."));
    };
    *commit_sha = Some(source.clone());
    Ok(StackOperationClaim {
        stack_id: stack.id,
        release_id: stack.current_stack_release_id,
        platform_id: stack.platform_id.ok_or(StackError::NotFound)?,
        name: stack.name.clone(),
        project_name: drift_project_name(stack.id, &stack.name, stack.spec.as_ref()),
        platform_type: stack.platform_type,
        spec,
        row_version: stack.row_version,
        actor_id: actor.value(),
        operation: "Drift".into(),
        service_names: Vec::new(),
    })
}
