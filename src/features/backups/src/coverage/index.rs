use super::model::*;
use crate::{
    BackupCoverageStatus as Status, BackupRun, policies::read_models::BackupPreviewKind,
    spec::BackupSourceSpec,
};
use chrono::{DateTime, Utc};
use std::collections::{BTreeMap, BTreeSet};
use uuid::Uuid;

type Owner = (BackupPreviewKind, Uuid);

/// Request-local relationships over the authorized inventory. Positions refer to the owned input vectors.
#[derive(Default)]
pub struct IndexedCoverage {
    inputs: CoverageSnapshot,
    workload_positions: BTreeMap<Owner, usize>,
    container_positions: BTreeMap<(Uuid, Option<String>, String), usize>,
    resource_policies: BTreeMap<Owner, Vec<usize>>,
    target_policies: BTreeMap<StorageTarget, Vec<usize>>,
    bind_policies: BTreeMap<(Uuid, Option<String>, String), Vec<usize>>,
    policy_positions: BTreeMap<Uuid, usize>,
    uncertain: BTreeSet<Uuid>,
}
impl std::ops::Deref for IndexedCoverage {
    type Target = CoverageSnapshot;
    fn deref(&self) -> &Self::Target {
        &self.inputs
    }
}
impl CoverageSnapshot {
    pub fn indexed(self) -> IndexedCoverage {
        let mut index = IndexedCoverage {
            inputs: self,
            ..Default::default()
        };
        for (pos, w) in index.inputs.workloads.iter().enumerate() {
            index.workload_positions.insert((w.kind, w.id), pos);
        }
        for (pos, c) in index.inputs.containers.iter().enumerate() {
            index.container_positions.insert(
                (c.platform_id, c.docker_node_id.clone(), c.docker_id.clone()),
                pos,
            );
        }
        for (pos, policy) in index.inputs.policies.iter().enumerate() {
            index.policy_positions.insert(policy.id, pos);
            match &policy.source {
                BackupSourceSpec::DockerVolume {
                    platform_id,
                    volume_name,
                    docker_node_id,
                    ..
                } => {
                    index
                        .target_policies
                        .entry(StorageTarget {
                            platform_id: *platform_id,
                            name: volume_name.clone(),
                            host_path: None,
                            docker_node_id: docker_node_id.clone(),
                        })
                        .or_default()
                        .push(pos);
                }
                BackupSourceSpec::BindMount {
                    platform_id,
                    host_path,
                    docker_node_id,
                } => {
                    index
                        .bind_policies
                        .entry((*platform_id, docker_node_id.clone(), host_path.clone()))
                        .or_default()
                        .push(pos);
                }
                source => {
                    if let Some(key) = source_identity(source) {
                        index.resource_policies.entry(key).or_default().push(pos);
                        if let Some(w) = index
                            .workload_positions
                            .get(&key)
                            .map(|i| &index.inputs.workloads[*i])
                        {
                            if !w.complete {
                                index.uncertain.insert(w.platform_id);
                            }
                            for v in w.volumes.iter().filter(|v| v.included) {
                                let members =
                                    index.target_policies.entry(v.target.clone()).or_default();
                                if members.last() != Some(&pos) {
                                    members.push(pos);
                                }
                            }
                        }
                    }
                }
            }
        }
        index
    }
}
impl IndexedCoverage {
    pub fn workload(
        &self,
        kind: BackupPreviewKind,
        id: Uuid,
        platform: Uuid,
    ) -> Option<&WorkloadStorage> {
        self.workload_positions
            .get(&(kind, id))
            .map(|i| &self.inputs.workloads[*i])
            .filter(|w| w.platform_id == platform)
    }
    pub fn container(
        &self,
        platform: Uuid,
        node: Option<&str>,
        id: &str,
    ) -> Option<&ContainerStorage> {
        self.container_positions
            .get(&(platform, node.map(str::to_owned), id.to_owned()))
            .map(|i| &self.inputs.containers[*i])
    }
    pub fn workload_policy_ids(
        &self,
        key: (BackupPreviewKind, Uuid),
    ) -> impl Iterator<Item = Uuid> + '_ {
        self.resource_policies
            .get(&key)
            .into_iter()
            .flatten()
            .map(|i| self.inputs.policies[*i].id)
    }
    pub fn matching_policies(&self, target: &StorageTarget) -> Vec<&CoveragePolicy> {
        let mut indices = self
            .target_policies
            .get(target)
            .cloned()
            .unwrap_or_default();
        if let Some(path) = &target.host_path
            && let Some(direct) = self.bind_policies.get(&(
                target.platform_id,
                target.docker_node_id.clone(),
                path.clone(),
            ))
        {
            indices.extend(direct);
            indices.sort_unstable();
        }
        indices
            .into_iter()
            .map(|i| &self.inputs.policies[i])
            .collect()
    }
    pub fn record_success(&mut self, target: StorageTarget, at: DateTime<Utc>) {
        let latest = self.inputs.successes.entry(target).or_insert(at);
        *latest = (*latest).max(at);
    }
    pub fn attach_run(&mut self, run: BackupRun) {
        if let Some(i) = self.policy_positions.get(&run.backup_policy_id) {
            self.inputs.policies[*i].latest_run = Some(run);
        }
    }
}
impl IndexedCoverage {
    pub fn evaluate(
        &self,
        platform: Uuid,
        targets: &[ObservedStorage],
        resource: Option<(BackupPreviewKind, Uuid)>,
        complete: bool,
        scheduling_available: bool,
    ) -> CoverageSummary {
        let mut related = resource
            .and_then(|key| self.resource_policies.get(&key))
            .into_iter()
            .flatten()
            .map(|i| self.inputs.policies[*i].id)
            .collect::<BTreeSet<_>>();
        let has_resource_policy = !related.is_empty();
        let mut reasons = BTreeSet::new();
        if !complete {
            reasons.insert(CoverageReason::InventoryIncomplete);
        }
        let uncertain = self.uncertain.contains(&platform);
        let volumes = targets
            .iter()
            .map(|v| {
                let matched = self.matching_policies(&v.target);
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
                    last_successful_run_at: self.successes.get(&v.target).copied(),
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
