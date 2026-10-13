//! Backup coverage is evaluated from authorized policies and persisted observations.
//! Readers must not contact Docker or execute source planning.
use crate::policies::read_models::BackupPreviewKind;
use crate::{
    BackupCoverageStatus as Status, BackupError, BackupRun, BackupRunItemStatus, BackupRunStatus,
    BackupSnapshotAvailability, spec::BackupSourceSpec,
};
use chrono::{DateTime, Utc};
use citadel_primitives::ActorId;
use futures_util::future::BoxFuture;
use std::collections::BTreeSet;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CoverageReason {
    InventoryUnavailable,
    InventoryIncomplete,
    SchedulingUnavailable,
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct StorageTarget {
    pub platform_id: Uuid,
    pub name: String,
    pub host_path: Option<String>,
    pub docker_node_id: Option<String>,
}
impl StorageTarget {
    pub fn supported(&self) -> bool {
        self.host_path
            .as_ref()
            .is_none_or(|p| crate::spec::validate_bind_path(p).is_ok())
    }
}
#[derive(Debug, Clone)]
pub struct ObservedStorage {
    pub target: StorageTarget,
    pub included: bool,
}
#[derive(Debug, Clone)]
pub struct WorkloadStorage {
    pub kind: BackupPreviewKind,
    pub id: Uuid,
    pub platform_id: Uuid,
    pub volumes: Vec<ObservedStorage>,
    pub complete: bool,
}
#[derive(Debug, Clone)]
pub struct ContainerStorage {
    pub id: Uuid,
    pub platform_id: Uuid,
    pub docker_id: String,
    pub docker_node_id: Option<String>,
    pub volumes: Vec<ObservedStorage>,
    pub complete: bool,
}
pub struct CoveragePolicy {
    pub id: Uuid,
    pub name: String,
    pub enabled: bool,
    pub cron: Option<String>,
    pub source: BackupSourceSpec,
    pub repository_ready: bool,
    pub latest_run: Option<BackupRun>,
}
impl CoveragePolicy {
    pub fn scheduled(&self) -> bool {
        self.cron.as_ref().is_some_and(|v| !v.trim().is_empty())
    }
}
#[derive(Default)]
pub struct CoverageSnapshot {
    pub policies: Vec<CoveragePolicy>,
    pub workloads: Vec<WorkloadStorage>,
    pub containers: Vec<ContainerStorage>,
    pub successes: Vec<(StorageTarget, DateTime<Utc>)>,
}
pub trait CoverageReader: Send + Sync {
    fn snapshot<'a>(
        &'a self,
        actor: ActorId,
        administrator: bool,
        platforms: &'a [Uuid],
    ) -> BoxFuture<'a, Result<CoverageSnapshot, BackupError>>;
}
#[derive(Debug)]
pub struct StorageCoverage {
    pub target: StorageTarget,
    pub included: bool,
    pub status: Option<Status>,
    pub policy_ids: Vec<Uuid>,
    pub last_successful_run_at: Option<DateTime<Utc>>,
}
#[derive(Debug)]
pub struct CoverageSummary {
    pub status: Option<Status>,
    pub volumes: Vec<StorageCoverage>,
    pub policy_ids: Vec<Uuid>,
    pub has_resource_policy: bool,
    pub reasons: Vec<CoverageReason>,
}
pub fn source_identity(source: &BackupSourceSpec) -> Option<(BackupPreviewKind, Uuid)> {
    match source {
        BackupSourceSpec::Deployment { deployment_id, .. } => {
            Some((BackupPreviewKind::Deployment, *deployment_id))
        }
        BackupSourceSpec::Stack { stack_id, .. } => Some((BackupPreviewKind::Stack, *stack_id)),
        BackupSourceSpec::SwarmService {
            swarm_service_id, ..
        } => Some((BackupPreviewKind::SwarmService, *swarm_service_id)),
        _ => None,
    }
}
pub fn policy_status(
    policy: &CoveragePolicy,
    target: &StorageTarget,
    scheduling_available: bool,
) -> Status {
    let run = policy.latest_run.as_ref();
    let item = run.and_then(|r| {
        r.items.iter().find(|i| {
            i.platform_id == target.platform_id
                && i.docker_node_id == target.docker_node_id
                && i.host_path == target.host_path
                && (target.host_path.is_some() || i.volume_name == target.name)
        })
    });
    if matches!(
        run.map(|r| r.status),
        Some(
            BackupRunStatus::Failed
                | BackupRunStatus::TimedOut
                | BackupRunStatus::Interrupted
                | BackupRunStatus::Rejected
        )
    ) || item.is_some_and(|i| i.status == BackupRunItemStatus::Failed)
    {
        return Status::Failed;
    }
    if policy.enabled
        && policy.scheduled()
        && scheduling_available
        && policy.repository_ready
        && run.is_some_and(|r| {
            r.source_snapshot == policy.source
                && r.status == BackupRunStatus::Succeeded
                && r.snapshot_availability == BackupSnapshotAvailability::Available
        })
        && item.is_some_and(|i| {
            i.status == BackupRunItemStatus::Succeeded && i.restic_snapshot_id.is_some()
        })
    {
        Status::Protected
    } else {
        Status::Warning
    }
}
pub fn aggregate(
    statuses: impl IntoIterator<Item = Option<Status>>,
    incomplete: bool,
) -> Option<Status> {
    let statuses = statuses
        .into_iter()
        .filter(|s| *s != Some(Status::NotApplicable))
        .collect::<Vec<_>>();
    if statuses.contains(&Some(Status::Failed)) {
        return Some(Status::Failed);
    }
    if incomplete || statuses.contains(&Some(Status::Warning)) {
        return Some(Status::Warning);
    }
    if statuses.contains(&None) {
        return None;
    }
    if statuses.is_empty() {
        return Some(Status::NotApplicable);
    }
    if statuses.iter().all(|s| *s == Some(Status::Protected)) {
        return Some(Status::Protected);
    }
    if statuses.iter().all(|s| *s == Some(Status::Unprotected)) {
        return Some(Status::Unprotected);
    }
    Some(Status::Warning)
}
impl CoverageSnapshot {
    pub fn evaluate(
        &self,
        platform: Uuid,
        targets: &[ObservedStorage],
        resource: Option<(BackupPreviewKind, Uuid)>,
        complete: bool,
        scheduling_available: bool,
    ) -> CoverageSummary {
        let mut related = BTreeSet::new();
        let mut has_resource_policy = false;
        for p in &self.policies {
            if resource.is_some() && source_identity(&p.source) == resource {
                related.insert(p.id);
                has_resource_policy = true;
            }
        }
        let mut reasons = BTreeSet::new();
        if !complete {
            reasons.insert(CoverageReason::InventoryIncomplete);
        }
        let uncertain = self
            .policies
            .iter()
            .filter_map(|p| source_identity(&p.source))
            .any(|(k, id)| {
                self.workloads
                    .iter()
                    .any(|w| w.kind == k && w.id == id && w.platform_id == platform && !w.complete)
            });
        let volumes = targets
            .iter()
            .map(|v| {
                let matched = self
                    .policies
                    .iter()
                    .filter(|p| match &p.source {
                        BackupSourceSpec::DockerVolume {
                            platform_id,
                            volume_name,
                            docker_node_id,
                            ..
                        } => {
                            v.target.host_path.is_none()
                                && v.target.platform_id == *platform_id
                                && v.target.name == *volume_name
                                && v.target.docker_node_id == *docker_node_id
                        }
                        BackupSourceSpec::BindMount {
                            platform_id,
                            host_path,
                            docker_node_id,
                        } => {
                            v.target.platform_id == *platform_id
                                && v.target.host_path.as_ref() == Some(host_path)
                                && v.target.docker_node_id == *docker_node_id
                        }
                        source => source_identity(source).is_some_and(|(kind, id)| {
                            self.workloads.iter().any(|w| {
                                w.kind == kind
                                    && w.id == id
                                    && w.volumes.iter().any(|candidate| {
                                        candidate.included && candidate.target == v.target
                                    })
                            })
                        }),
                    })
                    .collect::<Vec<_>>();
                let policy_ids = matched.iter().map(|p| p.id).collect::<Vec<_>>();
                related.extend(&policy_ids);
                if !scheduling_available && matched.iter().any(|p| p.enabled && p.scheduled()) {
                    reasons.insert(CoverageReason::SchedulingUnavailable);
                }
                let status = if !v.target.supported() {
                    Some(Status::NotApplicable)
                } else if matched.is_empty() {
                    if uncertain {
                        None
                    } else {
                        Some(Status::Unprotected)
                    }
                } else {
                    aggregate(
                        matched
                            .iter()
                            .map(|p| Some(policy_status(p, &v.target, scheduling_available))),
                        false,
                    )
                };
                StorageCoverage {
                    target: v.target.clone(),
                    included: v.included,
                    status,
                    policy_ids,
                    last_successful_run_at: self
                        .successes
                        .iter()
                        .filter(|(t, _)| *t == v.target)
                        .map(|(_, at)| *at)
                        .max(),
                }
            })
            .collect::<Vec<_>>();
        CoverageSummary {
            status: aggregate(volumes.iter().map(|v| v.status), !complete),
            volumes,
            policy_ids: related.into_iter().collect(),
            has_resource_policy,
            reasons: reasons.into_iter().collect(),
        }
    }
}
#[cfg(test)]
mod tests;

pub fn workload_includes(
    kind: BackupPreviewKind,
    target: &StorageTarget,
    declared_named: bool,
) -> bool {
    target.supported()
        && (kind != BackupPreviewKind::Deployment
            || (target.docker_node_id.is_none() && (target.host_path.is_some() || declared_named)))
}

/// Committed platform-scoped invalidations. None requests resynchronization after a listener gap.
pub trait CoverageChangeSource: Send {
    fn next(&mut self) -> BoxFuture<'_, Result<Option<CoverageInvalidation>, BackupError>>;
}
#[derive(Debug, Default)]
pub struct CoverageInvalidation {
    pub platform_id: Uuid,
    pub deployment_ids: Vec<Uuid>,
    pub stack_ids: Vec<Uuid>,
    pub service_ids: Vec<Uuid>,
    pub container_ids: Vec<Uuid>,
}
