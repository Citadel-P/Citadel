use super::*;
use crate::StackDriftContext;

impl StackService {
    pub async fn monitor_drift(
        &self,
        after: Option<Uuid>,
        limit: i64,
    ) -> Result<StackDriftMonitorResult, StackError> {
        self.monitor_drift_page(after, limit, &mut DriftPass::default())
            .await
    }

    pub async fn monitor_drift_page(
        &self,
        after: Option<Uuid>,
        limit: i64,
        inputs: &mut DriftPass,
    ) -> Result<StackDriftMonitorResult, StackError> {
        if limit <= 0 {
            return Err(validation(
                "The Stack drift monitor limit must be positive.",
            ));
        }
        let limit = limit.min(100);
        let candidates = self.store.drift_monitor_candidates(after, limit).await?;
        let mut result = StackDriftMonitorResult {
            checked: 0,
            reconciled: 0,
            failures: vec![],
            next_cursor: (candidates.len() as i64 == limit).then(|| candidates.last().unwrap().id),
        };
        let can_fix = candidates
            .iter()
            .any(|s| s.drift_policy.mode == crate::StackDriftMode::AutoFix)
            && self.operational_guardrails_enabled().await?;
        let actor = ActorId::new(Uuid::from_u128(1));
        let mut groups = BTreeMap::<_, Vec<_>>::new();
        for stack in candidates {
            groups.entry(stack.platform_id).or_default().push(stack);
        }

        for (platform, group) in groups {
            let mut prepared = Vec::new();
            for stack in group {
                if self.shutdown.is_cancelled() {
                    break;
                }
                match inputs
                    .load(
                        self.source_materializer.as_deref(),
                        &self.shutdown,
                        &stack,
                        actor,
                    )
                    .await
                {
                    Ok(names) => prepared.push((
                        drift_project_name(stack.id, &stack.name, stack.spec.as_ref()),
                        stack,
                        names,
                    )),
                    Err(e) => failure(&mut result, stack.id, e),
                }
            }
            if self.shutdown.is_cancelled() {
                break;
            }
            let projects = prepared
                .iter()
                .filter(|(_, s, _)| s.platform_type == citadel_platforms::PlatformKind::Docker)
                .map(|(project, _, _)| project.clone())
                .collect::<BTreeSet<_>>();
            let snapshots = if projects.len() > 1
                && let Some(platform) = platform
            {
                match self
                    .runtime
                    .compose_snapshots(platform, &projects, &self.shutdown)
                    .await
                {
                    Ok(snapshots) => snapshots,
                    Err(error) => {
                        let message = error.to_string();
                        for (_, stack, _) in prepared {
                            result.failures.push(StackDriftMonitorFailure {
                                stack_id: stack.id,
                                message: message.clone(),
                            });
                        }
                        continue;
                    }
                }
            } else {
                None
            };
            // Consume the entire shared observation synchronously before any
            // further I/O. Repair always obtains an independent live observation.
            let reports = prepared
                .into_iter()
                .map(|(project, stack, names)| {
                    let report = snapshots
                        .as_ref()
                        .and_then(|m| m.get(&project))
                        .and_then(|r| {
                            platform.map(|id| {
                                compare_names(stack.id, id, &stack.drift_policy, r, &names)
                            })
                        });
                    (project, stack, names, report)
                })
                .collect::<Vec<_>>();
            drop(snapshots);
            if self.shutdown.is_cancelled() {
                break;
            }
            // Submit healthy results from this snapshot together, before any individual
            // read, repair, or status write can delay them behind a newer container event.
            if let Some(alerts) = &self.alerts {
                let healthy: Vec<_> = reports
                    .iter()
                    .filter_map(|(_, stack, _, report)| {
                        report
                            .as_ref()
                            .filter(|r| !r.has_drift && stack.drift_policy.alert_on_drift)
                            .map(|r| stack_drift_observation(stack, r, "StackDriftDetected"))
                    })
                    .collect();
                if !healthy.is_empty()
                    && let Err(error) = alerts.observe_healthy_stack_drift(&healthy).await
                {
                    tracing::warn!(%error, count=healthy.len(), "Stack drift Alert evaluation failed");
                }
            }
            for (project, stack, names, report) in reports {
                if self.shutdown.is_cancelled() {
                    break;
                }
                let outcome = async {
                    let platform = platform.ok_or(StackError::NotFound)?;
                    let already_evaluated = report.as_ref().is_some_and(|r| !r.has_drift);
                    let report = if let Some(report) = report {
                        report
                    } else {
                        let runtime = self
                            .runtime
                            .runtime_snapshot(
                                platform,
                                &project,
                                orchestration(&stack.platform_type),
                                &self.shutdown,
                            )
                            .await?;
                        compare_names(stack.id, platform, &stack.drift_policy, &runtime, &names)
                    };
                    let report = if already_evaluated {
                        report
                    } else {
                        let (report, repaired) = self
                            .finish_drift_observation(&stack, actor, can_fix, report)
                            .await?;
                        result.reconciled += usize::from(repaired);
                        report
                    };
                    self.record_monitor_result(&stack, &report).await
                }
                .await;
                match outcome {
                    Ok(()) => result.checked += 1,
                    Err(e) => failure(&mut result, stack.id, e),
                }
            }
        }
        if self.shutdown.is_cancelled() {
            result.next_cursor = None;
        }
        Ok(result)
    }

    async fn record_monitor_result(
        &self,
        stack: &StackDriftContext,
        report: &StackDriftReport,
    ) -> Result<(), StackError> {
        if let Some((status, info)) = drift_status_update(stack, report)
            && self.store.record_drift(stack, status, info).await?
        {
            self.notifier.changed(stack.id, "driftChanged");
        }
        Ok(())
    }
}

fn failure(result: &mut StackDriftMonitorResult, id: Uuid, error: StackError) {
    result.failures.push(StackDriftMonitorFailure {
        stack_id: id,
        message: error.to_string(),
    });
}
