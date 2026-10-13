//! Presentation and authorization for feature-owned, persisted backup coverage.
use chrono::{DateTime, Utc};
use citadel_backups::{
    BackupCoverageStatus, BackupService, coverage::*, policies::read_models::BackupPreviewKind,
    runs::read_models::VolumeCoverageReader,
};
use citadel_identity::{ActorPrincipal, IdentityService};
use citadel_primitives::{PermissionLevel, ResourceType};
use serde::Serialize;
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, utoipa::ToSchema)]
pub enum BackupCoverageReason {
    InventoryUnavailable,
    InventoryIncomplete,
    SchedulingUnavailable,
}
impl From<CoverageReason> for BackupCoverageReason {
    fn from(value: CoverageReason) -> Self {
        match value {
            CoverageReason::InventoryUnavailable => Self::InventoryUnavailable,
            CoverageReason::InventoryIncomplete => Self::InventoryIncomplete,
            CoverageReason::SchedulingUnavailable => Self::SchedulingUnavailable,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct BackupPolicyReference {
    pub id: Uuid,
    pub name: String,
    pub enabled: Option<bool>,
    pub schedule_configured: Option<bool>,
    pub can_write: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ResourceBackupVolumeView {
    pub name: String,
    pub host_path: Option<String>,
    pub docker_node_id: Option<String>,
    pub node_hostname: Option<String>,
    pub included_in_resource_policy: bool,
    #[schema(value_type = Option<crate::api::resources::schema_models::backups::BackupCoverageStatusSchema>)]
    pub status: Option<BackupCoverageStatus>,
    pub policies: Vec<BackupPolicyReference>,
    pub last_successful_run_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ResourceBackupSummaryView {
    #[schema(value_type = Option<crate::api::resources::schema_models::backups::BackupCoverageStatusSchema>)]
    pub status: Option<BackupCoverageStatus>,
    pub platform_id: Uuid,
    pub volumes: Vec<ResourceBackupVolumeView>,
    pub policies: Vec<BackupPolicyReference>,
    pub has_resource_policy: bool,
    pub can_configure: bool,
    pub can_configure_bind_mount: Option<bool>,
    pub unavailable: bool,
    pub warnings: Vec<String>,
    pub reasons: Vec<BackupCoverageReason>,
}

pub struct BackupCoverageService {
    pub backups: Arc<BackupService>,
    pub volume_coverage: Arc<dyn VolumeCoverageReader>,
    pub identity: Arc<IdentityService>,
}
struct CoverageContext {
    snapshot: CoverageSnapshot,
    scheduling_available: bool,
    references: BTreeMap<Uuid, BackupPolicyReference>,
    can_configure: bool,
    writable_platforms: BTreeSet<Uuid>,
}
impl CoverageContext {
    fn present(&self, platform: Uuid, summary: CoverageSummary) -> ResourceBackupSummaryView {
        let can_bind = self.can_configure && self.writable_platforms.contains(&platform);
        let requires_host = summary
            .volumes
            .iter()
            .any(|v| v.included && v.target.host_path.is_some());
        let refs = |ids: Vec<Uuid>| {
            ids.into_iter()
                .filter_map(|id| self.references.get(&id).cloned())
                .collect()
        };
        ResourceBackupSummaryView {
            status: summary.status,
            platform_id: platform,
            volumes: summary
                .volumes
                .into_iter()
                .map(|v| ResourceBackupVolumeView {
                    name: v.target.name,
                    host_path: v.target.host_path,
                    docker_node_id: v.target.docker_node_id,
                    node_hostname: None,
                    included_in_resource_policy: v.included,
                    status: v.status,
                    policies: refs(v.policy_ids),
                    last_successful_run_at: v.last_successful_run_at,
                })
                .collect(),
            policies: refs(summary.policy_ids),
            has_resource_policy: summary.has_resource_policy,
            can_configure: self.can_configure && (!requires_host || can_bind),
            can_configure_bind_mount: Some(can_bind),
            unavailable: false,
            warnings: vec![],
            reasons: summary.reasons.into_iter().map(Into::into).collect(),
        }
    }
    fn workload(
        &self,
        kind: BackupPreviewKind,
        id: Uuid,
        platform: Uuid,
    ) -> ResourceBackupSummaryView {
        let Some(w) = self
            .snapshot
            .workloads
            .iter()
            .find(|w| w.kind == kind && w.id == id && w.platform_id == platform)
        else {
            return unavailable(platform);
        };
        self.present(
            platform,
            self.snapshot.evaluate(
                platform,
                &w.volumes,
                Some((kind, id)),
                w.complete,
                self.scheduling_available,
            ),
        )
    }
    fn container(
        &self,
        platform: Uuid,
        node: Option<&str>,
        docker: &str,
    ) -> ResourceBackupSummaryView {
        let Some(c) = self.snapshot.containers.iter().find(|c| {
            c.platform_id == platform
                && c.docker_node_id.as_deref() == node
                && c.docker_id == docker
        }) else {
            return unavailable(platform);
        };
        if !c.complete {
            return unavailable(platform);
        }
        self.present(
            platform,
            self.snapshot
                .evaluate(platform, &c.volumes, None, true, self.scheduling_available),
        )
    }
}
impl BackupCoverageService {
    async fn context(
        &self,
        principal: &ActorPrincipal,
        platforms: &[Uuid],
    ) -> Option<CoverageContext> {
        tokio::time::timeout(std::time::Duration::from_secs(3), async {
            let snapshot = self
                .volume_coverage
                .snapshot(principal.actor_id, principal.is_administrator(), platforms)
                .await
                .ok()?;
            let scheduling_available = self.backups.scheduling_available().await.ok()?;
            let ids = snapshot.policies.iter().map(|p| p.id).collect::<Vec<_>>();
            let permissions = self
                .identity
                .permissions_for_resources(principal, ResourceType::BackupPolicy, &ids)
                .await
                .ok()?;
            let platform_permissions = self
                .identity
                .permissions_for_resources(principal, ResourceType::Platform, platforms)
                .await
                .ok()?;
            let can_configure = principal.is_administrator()
                || self
                    .identity
                    .global_permission(principal, ResourceType::BackupPolicy)
                    .await
                    .ok()?
                    .is_some_and(|p| p.level.grants(PermissionLevel::Write));
            let references = snapshot
                .policies
                .iter()
                .map(|p| {
                    (
                        p.id,
                        BackupPolicyReference {
                            id: p.id,
                            name: p.name.clone(),
                            enabled: Some(p.enabled),
                            schedule_configured: Some(p.scheduled()),
                            can_write: Some(
                                principal.is_administrator()
                                    || permissions
                                        .get(&p.id)
                                        .and_then(|p| p.as_ref())
                                        .is_some_and(|p| p.level.grants(PermissionLevel::Write)),
                            ),
                        },
                    )
                })
                .collect();
            let writable_platforms = platforms
                .iter()
                .copied()
                .filter(|id| {
                    principal.is_administrator()
                        || platform_permissions
                            .get(id)
                            .and_then(|p| p.as_ref())
                            .is_some_and(|p| p.level.grants(PermissionLevel::Write))
                })
                .collect();
            Some(CoverageContext {
                snapshot,
                scheduling_available,
                references,
                can_configure,
                writable_platforms,
            })
        })
        .await
        .ok()
        .flatten()
    }
    pub async fn workload(
        &self,
        principal: &ActorPrincipal,
        kind: BackupPreviewKind,
        id: Uuid,
        platform: Uuid,
    ) -> ResourceBackupSummaryView {
        self.context(principal, &[platform])
            .await
            .map_or_else(|| unavailable(platform), |c| c.workload(kind, id, platform))
    }
    pub async fn container(
        &self,
        principal: &ActorPrincipal,
        platform: Uuid,
        node: Option<&str>,
        docker: &str,
    ) -> ResourceBackupSummaryView {
        self.context(principal, &[platform]).await.map_or_else(
            || unavailable(platform),
            |c| c.container(platform, node, docker),
        )
    }
    pub async fn inspected(
        &self,
        principal: &ActorPrincipal,
        platform: Uuid,
        node: Option<&str>,
        inspection: &serde_json::Value,
    ) -> ResourceBackupSummaryView {
        let id = inspection
            .get("id")
            .or_else(|| inspection.get("Id"))
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default();
        self.container(principal, platform, node, id).await
    }
    pub async fn volumes(
        &self,
        principal: &ActorPrincipal,
        platform: Uuid,
        keys: &[(String, Option<String>)],
    ) -> ResourceBackupSummaryView {
        let Some(c) = self.context(principal, &[platform]).await else {
            return unavailable(platform);
        };
        let targets = keys
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .map(|(name, node)| ObservedStorage {
                target: StorageTarget {
                    platform_id: platform,
                    name,
                    host_path: None,
                    docker_node_id: node,
                },
                included: false,
            })
            .collect::<Vec<_>>();
        c.present(
            platform,
            c.snapshot
                .evaluate(platform, &targets, None, true, c.scheduling_available),
        )
    }
    pub async fn enrich_deployments(
        &self,
        principal: &ActorPrincipal,
        mut views: Vec<crate::api::resources::deployments::views::DeploymentView>,
    ) -> Vec<crate::api::resources::deployments::views::DeploymentView> {
        if views.is_empty() {
            return views;
        }
        let platforms = views
            .iter()
            .map(|v| v.platform_id)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        let context = self.context(principal, &platforms).await;
        for view in &mut views {
            view.backup_coverage = Some(context.as_ref().map_or_else(
                || unavailable(view.platform_id),
                |c| c.workload(BackupPreviewKind::Deployment, view.id, view.platform_id),
            ));
        }
        views
    }
    pub async fn enrich_stacks(
        &self,
        principal: &ActorPrincipal,
        mut views: Vec<crate::api::resources::stacks::views::StackView>,
    ) -> Vec<crate::api::resources::stacks::views::StackView> {
        if views.is_empty() {
            return views;
        }
        let platforms = views
            .iter()
            .filter_map(|v| v.platform_id)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        let context = self.context(principal, &platforms).await;
        for view in &mut views {
            if let Some(platform) = view.platform_id {
                view.backup_coverage = Some(context.as_ref().map_or_else(
                    || unavailable(platform),
                    |c| c.workload(BackupPreviewKind::Stack, view.id, platform),
                ));
            }
        }
        views
    }
    pub async fn enrich_services(
        &self,
        principal: &ActorPrincipal,
        mut views: Vec<crate::api::resources::swarm_services::views::ManagedSwarmServiceView>,
    ) -> Vec<crate::api::resources::swarm_services::views::ManagedSwarmServiceView> {
        if views.is_empty() {
            return views;
        }
        let platforms = views
            .iter()
            .map(|v| v.platform_id)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        let context = self.context(principal, &platforms).await;
        for view in &mut views {
            view.backup_coverage = Some(context.as_ref().map_or_else(
                || unavailable(view.platform_id),
                |c| c.workload(BackupPreviewKind::SwarmService, view.id, view.platform_id),
            ));
        }
        views
    }
    pub async fn enrich_containers(
        &self,
        principal: &ActorPrincipal,
        mut views: Vec<crate::api::resources::platforms::views::ContainerView>,
    ) -> Vec<crate::api::resources::platforms::views::ContainerView> {
        if views.is_empty() {
            return views;
        }
        let platforms = views
            .iter()
            .map(|v| v.platform_id)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        let context = self.context(principal, &platforms).await;
        for view in &mut views {
            view.backup_coverage = Some(if view.projection_stale_since.is_some() {
                unavailable(view.platform_id)
            } else {
                context.as_ref().map_or_else(
                    || unavailable(view.platform_id),
                    |c| {
                        c.container(
                            view.platform_id,
                            view.docker_node_id.as_deref(),
                            &view.container_id,
                        )
                    },
                )
            });
        }
        views
    }
}
fn unavailable(platform_id: Uuid) -> ResourceBackupSummaryView {
    ResourceBackupSummaryView {
        status: None,
        platform_id,
        volumes: vec![],
        policies: vec![],
        has_resource_policy: false,
        can_configure: false,
        can_configure_bind_mount: Some(false),
        unavailable: true,
        warnings: vec![],
        reasons: vec![BackupCoverageReason::InventoryUnavailable],
    }
}
