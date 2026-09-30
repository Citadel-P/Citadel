use super::*;
use crate::realtime::topic::Topic;
use citadel_activities::{ActivityFilter, ActivityService};
use citadel_automation::AutomationRepository;
use citadel_backups::BackupPersistence;
use citadel_builds::BuildRepository;
use citadel_deployments::DeploymentRepository;
use citadel_identity::IdentityService;
use citadel_platforms::PlatformReadService;
use citadel_primitives::{PermissionLevel, ResourceType, SpecificPermission};
use citadel_stacks::StackRepository;
use citadel_swarm_services::SwarmServiceRepository;
use std::sync::Arc;

mod logs;
mod terminal;
#[cfg(test)]
mod tests;

// Reuse the same authorized read models as HTTP. No internal HTTP requests,
// browser query policy, or copied SQL schema. Container rows are shared only
// during delivery of one invalidation, after each reader checks authorization.
#[derive(Clone)]
pub struct ApplicationGroupReader {
    pub git_repositories: Arc<dyn citadel_git::GitRepositoryPersistence>,
    pub identity: Arc<IdentityService>,
    pub platforms: Arc<PlatformReadService>,
    pub deployments: Arc<dyn DeploymentRepository>,
    pub stacks: Arc<dyn StackRepository>,
    pub services: Arc<dyn SwarmServiceRepository>,
    pub automation: Arc<dyn AutomationRepository>,
    pub builds: Arc<dyn BuildRepository>,
    pub backups: Arc<dyn BackupPersistence>,
    pub activities: Arc<ActivityService>,
    pub alerts: Arc<dyn citadel_alerts::AlertRepository>,
    pub docker: crate::api::routes::platforms::PlatformsHttpState,
}

fn failure(error: impl std::fmt::Display) -> RealtimeReadError {
    RealtimeReadError::Storage(error.to_string())
}
fn rows<T: Serialize>(
    target: &'static str,
    values: Vec<T>,
    style: RowStyle,
) -> Result<GroupSnapshot, RealtimeReadError> {
    Ok(GroupSnapshot {
        rows: vec![GroupRows {
            target,
            rows: values
                .into_iter()
                .map(serde_json::to_value)
                .collect::<Result<_, _>>()
                .map_err(failure)?,
            style,
        }],
        events: vec![],
    })
}
fn event(target: &'static str, value: impl Serialize) -> Result<GroupSnapshot, RealtimeReadError> {
    Ok(GroupSnapshot {
        rows: vec![],
        events: vec![ClientEvent::new(
            target,
            vec![serde_json::to_value(value).map_err(failure)?],
        )],
    })
}

impl ApplicationGroupReader {
    fn container_patch_snapshot(
        group: &Group,
        event: Option<&PublishedRuntimeEvent>,
        platform: Option<Uuid>,
    ) -> Option<GroupSnapshot> {
        let event = event.filter(|event| event.platform_id == platform && platform.is_some())?;
        let patches = event.container_state_patches()?;
        match group.topic() {
            Topic::Containers(..) => Some(GroupSnapshot {
                rows: vec![],
                events: vec![ClientEvent::new(
                    "ContainersStateChanged",
                    vec![json!({"platformId":platform,"patches":patches})],
                )],
            }),
            Topic::DockerDaemon(..) => Some(GroupSnapshot {
                rows: vec![],
                events: vec![ClientEvent::new(
                    "ContainerStateChanged",
                    vec![json!(patches)],
                )],
            }),
            _ => None,
        }
    }

    async fn changed_containers(
        &self,
        event: &PublishedRuntimeEvent,
        platform: Uuid,
        ids: &[String],
    ) -> Result<Vec<crate::api::resources::platforms::views::ContainerView>, RealtimeReadError>
    {
        if event.platform_id != Some(platform) {
            return Ok(vec![]);
        }
        let containers = event
            .containers
            .get_or_try_init(|| async {
                let _read =
                    citadel_runtime::runtime_metrics::RuntimeWork::RealtimeSharedRead.start();
                self.platforms
                    .containers_by_runtime_ids(platform, ids)
                    .await
                    .map_err(failure)
            })
            .await?;
        Ok(containers
            .iter()
            .cloned()
            .map(crate::api::resources::platforms::views::ContainerView::from)
            .collect())
    }

    async fn deployment_permission<P: citadel_primitives::PermissionPolicy>(
        &self,
        principal: &ActorPrincipal,
        id: Uuid,
    ) -> Result<(), RealtimeReadError> {
        self.identity
            .require_resource::<P>(principal, id)
            .await
            .map_err(|error| match error {
                citadel_identity::IdentityError::Forbidden => RealtimeReadError::Authorization,
                other => failure(other),
            })
    }

    async fn permission(
        &self,
        p: &ActorPrincipal,
        kind: ResourceType,
        id: Option<Uuid>,
        specific: Option<SpecificPermission>,
    ) -> Result<(), RealtimeReadError> {
        self.permission_with_lease(p, kind, id, specific, None)
            .await
    }

    async fn permission_with_lease(
        &self,
        p: &ActorPrincipal,
        kind: ResourceType,
        id: Option<Uuid>,
        specific: Option<SpecificPermission>,
        lease: Option<&crate::realtime::AuthorizationLease>,
    ) -> Result<(), RealtimeReadError> {
        use citadel_primitives::PermissionPolicy;
        let workload_requirement = match (kind, specific) {
            (ResourceType::Stack, None) => {
                Some(citadel_stacks::permissions::ReadStack::REQUIREMENT)
            }
            (ResourceType::Stack, Some(SpecificPermission::Logs)) => {
                Some(citadel_stacks::permissions::ViewStackLogs::REQUIREMENT)
            }
            (ResourceType::Stack, Some(SpecificPermission::Inspect)) => {
                Some(citadel_stacks::permissions::InspectStack::REQUIREMENT)
            }
            (ResourceType::Stack, Some(SpecificPermission::Terminal)) => {
                Some(citadel_stacks::permissions::OpenStackTerminal::REQUIREMENT)
            }
            (ResourceType::SwarmService, None) => {
                Some(citadel_swarm_services::permissions::ReadSwarmService::REQUIREMENT)
            }
            (ResourceType::SwarmService, Some(SpecificPermission::Logs)) => {
                Some(citadel_swarm_services::permissions::ViewSwarmServiceLogs::REQUIREMENT)
            }
            (ResourceType::SwarmService, Some(SpecificPermission::Inspect)) => {
                Some(citadel_swarm_services::permissions::InspectSwarmService::REQUIREMENT)
            }
            _ => None,
        };
        if let Some(lease) = lease {
            return lease
                .authorize(
                    &self.identity,
                    p,
                    kind,
                    id,
                    workload_requirement.map_or(PermissionLevel::Read, |r| r.level),
                    workload_requirement.and_then(|r| r.specific).or(specific),
                )
                .await;
        }
        if p.is_administrator() {
            return Ok(());
        }
        let grant = match id {
            Some(id) => self.identity.permission_for_resource(p, kind, id).await,
            None => self.identity.global_permission(p, kind).await,
        }
        .map_err(failure)?;
        if grant.is_some_and(|g| {
            workload_requirement.map_or_else(
                || {
                    g.level.grants(PermissionLevel::Read)
                        && specific.is_none_or(|s| g.has_specific(s))
                },
                |requirement| {
                    g.level.grants(requirement.level)
                        && requirement.specific.is_none_or(|s| g.has_specific(s))
                },
            )
        }) {
            Ok(())
        } else {
            Err(RealtimeReadError::Authorization)
        }
    }

    async fn read_group(
        &self,
        p: &ActorPrincipal,
        g: &Group,
        e: Option<&PublishedRuntimeEvent>,
        lease: Option<&crate::realtime::AuthorizationLease>,
    ) -> Result<GroupSnapshot, RealtimeReadError> {
        if matches!(
            g.topic(),
            Topic::ContainerExec { .. } | Topic::SwarmTaskExec { .. }
        ) {
            self.terminal_target(p, g).await?;
            return Ok(GroupSnapshot::default());
        }
        if matches!(g.topic(), Topic::ContainerLog(..) | Topic::StackLog(..)) {
            self.log_targets(p, g).await?;
            return Ok(GroupSnapshot::default());
        }
        use ResourceType::*;
        let id = g.id();
        let actor = p.actor_id;
        let admin = p.is_administrator();
        let kind = match g.topic() {
            Topic::Platforms
            | Topic::Containers(..)
            | Topic::Images(..)
            | Topic::DockerDaemon(..) => Platform,
            Topic::Deployment(..) | Topic::Deployments => Deployment,
            Topic::Stack(..) | Topic::Stacks | Topic::StackInfo(..) => Stack,
            Topic::SwarmService(..) | Topic::SwarmServices(..) => SwarmService,
            Topic::GitRepo(..) | Topic::GitRepositories => GitRepository,
            Topic::AutomationAction(..) | Topic::AutomationActions => AutomationAction,
            Topic::BackupRepository(..) | Topic::BackupRepositories => BackupRepository,
            Topic::BackupPolicy(..)
            | Topic::BackupPolicies
            | Topic::BackupRuns(..)
            | Topic::BackupRun(..)
            | Topic::BackupRestoreRuns(..)
            | Topic::BackupRestoreRun(..) => BackupPolicy,
            Topic::BuildProject(..)
            | Topic::BuildProjects
            | Topic::BuildRuns(..)
            | Topic::BuildRun(..) => Build,
            Topic::BuildAgentPool(..) | Topic::BuildAgentPools => BuildAgentPool,
            Topic::ContainerInfo(..) => Platform, // resolved through its actual owner below
            Topic::Activity { .. } => return self.activities(p, g, lease).await,
            Topic::AlertEvents => {
                let filter = citadel_alerts::AlertEventFilter {
                    page: 1,
                    page_size: 100,
                    resource_id: None,
                    resource_type: None,
                    alert_type: None,
                    unresolved_only: false,
                };
                let events = self
                    .alerts
                    .list_events(actor, admin, &filter)
                    .await
                    .map_err(failure)?;
                let events: crate::api::resources::alerts::views::AlertEventPage =
                    events.try_into().map_err(failure)?;
                return Ok(GroupSnapshot {
                    rows: vec![GroupRows {
                        target: "AlertEventReceived",
                        rows: events
                            .items
                            .iter()
                            .map(serde_json::to_value)
                            .collect::<Result<_, _>>()
                            .map_err(failure)?,
                        style: RowStyle::Notification,
                    }],
                    events: vec![
                        ClientEvent::new(
                            "AlertEventsUpdated",
                            vec![serde_json::to_value(events.items).map_err(failure)?],
                        ),
                        ClientEvent::new(
                            "UnresolvedAlertCount",
                            vec![
                                json!({"count":self.alerts.unresolved_count(actor,admin).await.map_err(failure)?}),
                            ],
                        ),
                    ],
                });
            }
            _ => return Err(RealtimeReadError::Authorization),
        };
        let permission_id = match g.topic() {
            Topic::SwarmServices(..) => None,
            Topic::BackupRun(..) => Some(
                self.backups
                    .get_run(id.unwrap())
                    .await
                    .map_err(failure)?
                    .backup_policy_id,
            ),
            Topic::BackupRestoreRun(..) => {
                let restore = self
                    .backups
                    .get_restore(id.unwrap())
                    .await
                    .map_err(failure)?;
                Some(
                    self.backups
                        .get_run(restore.backup_run_id)
                        .await
                        .map_err(failure)?
                        .backup_policy_id,
                )
            }
            Topic::BuildRun(..) => Some(
                self.builds
                    .get_run(id.unwrap())
                    .await
                    .map_err(failure)?
                    .build_project_id,
            ),
            Topic::ContainerInfo(..) => return self.container_info(p, g, e, lease).await,
            _ => id,
        };
        self.permission_with_lease(
            p,
            kind,
            permission_id,
            matches!(
                g.topic(),
                Topic::BackupRestoreRun(..) | Topic::BackupRestoreRuns(..)
            )
            .then_some(SpecificPermission::Restore),
            lease,
        )
        .await?;
        if let Some(snapshot) = Self::container_patch_snapshot(g, e, id) {
            return Ok(snapshot);
        }
        if matches!(
            g.topic(),
            Topic::Deployment(..)
                | Topic::Deployments
                | Topic::Stack(..)
                | Topic::Stacks
                | Topic::StackInfo(..)
        ) && let Some(change) = e
            && let (Some(platform), Some(references)) = (change.platform_id, change.container_ids())
        {
            let containers = self
                .changed_containers(change, platform, &references)
                .await?;
            // A missing row may have belonged to this workload before deletion.
            // Keep the authoritative refresh in that case.
            if references
                .iter()
                .all(|reference| containers.iter().any(|c| &c.container_id == reference))
                && !containers.iter().any(|container| {
                    let owner = if matches!(g.topic(), Topic::Deployment(..) | Topic::Deployments) {
                        container.deployment_id
                    } else {
                        container.stack_id
                    };
                    owner.is_some_and(|owner| id.is_none_or(|id| id == owner))
                })
            {
                return Ok(GroupSnapshot::default());
            }
        }
        if matches!(g.topic(), Topic::SwarmServices(..)) {
            self.permission_with_lease(p, Platform, id, None, lease)
                .await?;
        }
        if matches!(g.topic(), Topic::DockerDaemon(..))
            && e.is_some_and(|event| event.payload["dockerResourceType"] == "nodeAgentCoverage")
        {
            return event("SwarmNodeAgentCoverageChanged", id.unwrap());
        }
        if matches!(g.topic(), Topic::BuildRun(..))
            && let Some(event) =
                e.filter(|event| event.event_kind == "buildLogs" && Some(event.resource_id) == id)
        {
            return Ok(GroupSnapshot {
                rows: vec![],
                events: vec![ClientEvent::new(
                    "BuildRunLogsAppended",
                    vec![json!(id.unwrap()), event.payload["entries"].clone()],
                )],
            });
        }
        if matches!(g.topic(), Topic::SwarmService(..)) {
            let service = self
                .services
                .get_authorized(actor, admin, id.unwrap())
                .await
                .map_err(failure)?;
            self.permission_with_lease(p, Platform, Some(service.platform_id), None, lease)
                .await?;
        }
        let sample = e.is_some_and(|e| e.payload["dockerResourceType"] == "containerStats");
        match g.topic() {
            Topic::Platforms => {
                if sample {
                    let platform_id = e.unwrap().platform_id.unwrap();
                    match self
                        .permission_with_lease(p, Platform, Some(platform_id), None, lease)
                        .await
                    {
                        Ok(()) => {}
                        Err(RealtimeReadError::Authorization) => {
                            return Ok(GroupSnapshot::default());
                        }
                        Err(error) => return Err(error),
                    }
                    let Some(context) =
                        crate::realtime::shared_reads::telemetry(&self.platforms, platform_id, e)
                            .await?
                    else {
                        return Ok(GroupSnapshot::default());
                    };
                    let Some(payload) = live_platform_stats(
                        platform_id,
                        &context,
                        &e.unwrap().payload["platformSample"],
                    ) else {
                        return Ok(GroupSnapshot::default());
                    };
                    return event("PlatformStatsUpdated", payload);
                }
                if let Some(platform_id) = e.and_then(|e| e.platform_id) {
                    let allowed = match self
                        .permission_with_lease(p, Platform, Some(platform_id), None, lease)
                        .await
                    {
                        Ok(()) => true,
                        Err(RealtimeReadError::Authorization) => false,
                        Err(error) => return Err(error),
                    };
                    let platform = if allowed {
                        crate::realtime::shared_reads::platform(&self.platforms, platform_id, e)
                            .await
                            .map_err(failure)?
                    } else {
                        None
                    };
                    return rows(
                        "PlatformUpdated",
                        platform
                            .into_iter()
                            .map(crate::api::resources::platforms::views::PlatformView::try_from)
                            .collect::<Result<Vec<_>, _>>().map_err(failure)?,
                        RowStyle::PlatformPatch(platform_id),
                    );
                }
                rows(
                    "PlatformUpdated",
                    self.platforms
                        .list_authorized(actor, admin, &[])
                        .await
                        .map(|value| {
                            value
                                .into_iter()
                                .map(crate::api::resources::platforms::views::PlatformView::try_from)
                                .collect::<Result<Vec<_>, _>>()
                        })
                        .map_err(failure)?.map_err(failure)?,
                    RowStyle::Platforms,
                )
            }
            Topic::Containers(..) => {
                if sample {
                    let identities =
                        crate::realtime::shared_reads::identities(&self.platforms, id.unwrap(), e)
                            .await?;
                    return event("ContainersStatsUpdated", map_stats(e.unwrap(), &identities));
                }
                if let Some(references) = e.and_then(PublishedRuntimeEvent::container_ids) {
                    let containers = self
                        .changed_containers(e.unwrap(), id.unwrap(), &references)
                        .await?;
                    let mut result = GroupSnapshot::default();
                    for reference in references {
                        result.events.extend(event(
                            "ContainersChanged",
                            json!({"platformId":id,"containerId":reference,"containers":containers.iter().filter(|c| c.container_id == reference).collect::<Vec<_>>()}),
                        )?.events);
                    }
                    return Ok(result);
                }
                let containers =
                    crate::realtime::shared_reads::containers(&self.platforms, id.unwrap(), e)
                        .await
                        .map(|value| {
                            value
                                .into_iter()
                                .map(crate::api::resources::platforms::views::ContainerView::from)
                                .collect::<Vec<_>>()
                        })
                        .map_err(failure)?;
                event("ContainersInfoUpdated", json!({"containers":containers}))
            }
            Topic::Images(..) => {
                if sample {
                    return Ok(GroupSnapshot::default());
                }
                event(
                    "ImagesInfoUpdated",
                    json!({"images":crate::realtime::shared_reads::images(&self.platforms, id.unwrap(), e).await.map(|value| value.into_iter().map(crate::api::resources::platforms::views::ImageView::from).collect::<Vec<_>>()).map_err(failure)?}),
                )
            }
            Topic::DockerDaemon(..) => {
                if sample {
                    return Ok(GroupSnapshot::default());
                }
                if let Some(references) = e.and_then(PublishedRuntimeEvent::container_ids) {
                    let containers = self
                        .changed_containers(e.unwrap(), id.unwrap(), &references)
                        .await?;
                    let mut result = GroupSnapshot::default();
                    for reference in references {
                        result.rows.extend(
                            rows(
                                "ContainerEventReceived",
                                containers
                                    .iter()
                                    .filter(|c| c.container_id == reference)
                                    .collect(),
                                RowStyle::DockerResourcePatch(reference),
                            )?
                            .rows,
                        );
                    }
                    return Ok(result);
                }
                if let Some(event) = e
                    && let Some(snapshot) =
                        crate::api::routes::platforms::realtime_resource_snapshot(
                            &self.docker,
                            p,
                            id.unwrap(),
                            lease,
                            event,
                        )
                        .await?
                {
                    return Ok(snapshot);
                }
                let mut result = crate::api::routes::platforms::realtime_daemon_snapshot(
                    &self.docker,
                    p,
                    id.unwrap(),
                    lease,
                )
                .await?;
                result.rows.extend(
                    rows(
                        "ContainerEventReceived",
                        crate::realtime::shared_reads::containers(&self.platforms, id.unwrap(), e)
                            .await
                            .map(|value| {
                                value
                                    .into_iter()
                                    .map(crate::api::resources::platforms::views::ContainerView::from)
                                    .collect::<Vec<_>>()
                            })
                            .map_err(failure)?,
                        RowStyle::DockerResource,
                    )?
                    .rows,
                );
                result.rows.extend(
                    rows(
                        "ImageEventReceived",
                        crate::realtime::shared_reads::images(&self.platforms, id.unwrap(), e)
                            .await
                            .map(|value| {
                                value
                                    .into_iter()
                                    .map(crate::api::resources::platforms::views::ImageView::from)
                                    .collect::<Vec<_>>()
                            })
                            .map_err(failure)?,
                        RowStyle::DockerResource,
                    )?
                    .rows,
                );
                if crate::realtime::shared_reads::platform(&self.platforms, id.unwrap(), e)
                    .await
                    .map_err(failure)?
                    .is_some_and(|platform| {
                        matches!(platform.platform_type.as_str(), "DockerSwarm" | "Swarm")
                    })
                {
                    result.events.push(ClientEvent::new("SwarmInventoryUpdated",vec![json!({
                    "platformId":id,
                    "nodes":{"items":self.platforms.list_swarm_nodes(id.unwrap()).await.map(|value| value.into_iter().map(crate::api::resources::platforms::views::SwarmNodeView::from).collect::<Vec<_>>()).map_err(failure)?},
                    "services":{"items":self.platforms.list_swarm_services(id.unwrap()).await.map(|value| value.into_iter().map(crate::api::resources::platforms::views::SwarmServiceView::from).collect::<Vec<_>>()).map_err(failure)?},
                    "tasks":{"items":self.platforms.list_swarm_tasks(id.unwrap(),None,1000).await.map(|value| value.into_iter().map(crate::api::resources::platforms::views::SwarmTaskView::from).collect::<Vec<_>>()).map_err(failure)?},
                    "networks":{"items":self.platforms.list_swarm_networks(id.unwrap()).await.map(|value| value.into_iter().map(crate::api::resources::platforms::views::SwarmNetworkView::from).collect::<Vec<_>>()).map_err(failure)?},
                    "secrets":{"items":self.platforms.list_swarm_secrets(id.unwrap()).await.map(|value| value.into_iter().map(crate::api::resources::platforms::views::SwarmSecretView::from).collect::<Vec<_>>()).map_err(failure)?},
                    "configs":{"items":self.platforms.list_swarm_configs(id.unwrap()).await.map(|value| value.into_iter().map(crate::api::resources::platforms::views::SwarmConfigView::from).collect::<Vec<_>>()).map_err(failure)?},
                })]));
                }
                Ok(result)
            }
            Topic::Deployments => rows(
                "DeploymentInfoUpdated",
                self.deployments
                    .list_authorized(actor, admin, &Default::default())
                    .await
                    .map_err(failure)?
                    .into_iter()
                    .map(crate::api::resources::deployments::views::DeploymentView::try_from)
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(failure)?,
                RowStyle::Update,
            ),
            Topic::Deployment(..) => rows(
                "DeploymentInfoUpdated",
                vec![
                    crate::api::resources::deployments::views::DeploymentView::try_from(
                        self.deployments
                            .get_authorized(actor, admin, id.unwrap())
                            .await
                            .map_err(failure)?,
                    )
                    .map_err(failure)?,
                ],
                RowStyle::Update,
            ),
            Topic::Stacks => rows(
                "StackInfoUpdated",
                self.stacks
                    .list_authorized(actor, admin, &Default::default())
                    .await
                    .map_err(failure)?
                    .into_iter()
                    .map(crate::api::resources::stacks::views::StackView::try_from)
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(failure)?,
                RowStyle::Update,
            ),
            Topic::Stack(..) => rows(
                "StackInfoUpdated",
                vec![
                    crate::api::resources::stacks::views::StackView::try_from(
                        self.stacks
                            .get_authorized(actor, admin, id.unwrap())
                            .await
                            .map_err(failure)?,
                    )
                    .map_err(failure)?,
                ],
                RowStyle::Update,
            ),
            Topic::StackInfo(..) => {
                let stack = self
                    .stacks
                    .get_authorized(actor, admin, id.unwrap())
                    .await
                    .map_err(failure)?;
                let platform = stack.platform_id.ok_or(RealtimeReadError::Authorization)?;
                if e.and_then(|e| e.platform_id)
                    .is_some_and(|id| id != platform)
                {
                    return Ok(GroupSnapshot::default());
                }
                let containers = self
                    .platforms
                    .list_stack_containers(id.unwrap())
                    .await
                    .map(|value| {
                        value
                            .into_iter()
                            .map(crate::api::resources::platforms::views::ContainerView::from)
                            .collect::<Vec<_>>()
                    })
                    .map_err(failure)?
                    .into_iter()
                    .filter(|c| c.stack_id == id)
                    .map(|c| container_data(c, e))
                    .collect::<Result<Vec<_>, _>>().map_err(failure)?;
                event("ReceiveStackContainersInfo", containers)
            }
            Topic::SwarmService(..) => rows(
                "SwarmServiceInfoUpdated",
                vec![
                    crate::api::resources::swarm_services::views::ManagedSwarmServiceView::try_from(
                        self.services
                            .get_authorized(actor, admin, id.unwrap())
                            .await
                            .map_err(failure)?,
                    ).map_err(failure)?,
                ],
                RowStyle::Update,
            ),
            Topic::SwarmServices(..) => rows(
                "SwarmServiceInfoUpdated",
                self.services
                    .list_authorized(
                        actor,
                        admin,
                        &citadel_swarm_services::SwarmServiceFilter {
                            platform_id: id,
                            ..Default::default()
                        },
                    )
                    .await
                    .map_err(failure)?
                    .into_iter()
                    .map(
                        crate::api::resources::swarm_services::views::ManagedSwarmServiceView::try_from,
                    )
                    .collect::<Result<Vec<_>, _>>().map_err(failure)?,
                RowStyle::Update,
            ),
            Topic::GitRepositories => rows(
                "GitRepositoryInfoUpdated",
                self.git_repositories
                    .list_git_repositories(actor, admin)
                    .await
                    .map_err(failure)?
                    .into_iter()
                    .map(crate::api::resources::git_repositories::views::GitRepositoryView::try_from)
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(failure)?,
                RowStyle::Update,
            ),
            Topic::GitRepo(..) => rows(
                "GitRepositoryInfoUpdated",
                vec![
                    self.git_repositories
                        .get_git_repository(id.unwrap())
                        .await
                        .map_err(failure)
                        .and_then(|repository| {
                            crate::api::resources::git_repositories::views::GitRepositoryView::try_from(repository)
                                .map_err(failure)
                        })?,
                ],
                RowStyle::Update,
            ),
            Topic::AutomationActions => rows(
                "AutomationActionInfoUpdated",
                crate::api::routes::automation::authorized_actions(
                    self.automation.as_ref(),
                    p,
                    self.automation.list(actor, admin).await.map_err(failure)?,
                )
                .await
                .map_err(failure)?,
                RowStyle::Update,
            ),
            Topic::AutomationAction(..) => rows(
                "AutomationActionInfoUpdated",
                crate::api::routes::automation::authorized_actions(
                    self.automation.as_ref(),
                    p,
                    vec![self.automation.get(id.unwrap()).await.map_err(failure)?],
                )
                .await
                .map_err(failure)?,
                RowStyle::Update,
            ),
            Topic::BackupRepositories => rows(
                "BackupRepositoryInfoUpdated",
                self.backups
                    .list_repositories(actor, admin)
                    .await
                    .map_err(failure)?
                    .into_iter()
                    .map(crate::api::resources::backups::views::BackupRepositoryView::try_from)
                    .collect::<Result<_, _>>().map_err(failure)?,
                RowStyle::Update,
            ),
            Topic::BackupRepository(..) => rows(
                "BackupRepositoryInfoUpdated",
                vec![
                    crate::api::resources::backups::views::BackupRepositoryView::try_from(
                        self.backups
                            .get_repository(id.unwrap())
                            .await
                            .map_err(failure)?,
                    ).map_err(failure)?,
                ],
                RowStyle::Update,
            ),
            Topic::BackupPolicies => rows(
                "BackupPolicyInfoUpdated",
                self.backups
                    .list_policies(actor, admin)
                    .await
                    .map_err(failure)?
                    .into_iter()
                    .map(crate::api::resources::backups::views::BackupPolicyView::try_from)
                    .collect::<Result<_, _>>().map_err(failure)?,
                RowStyle::Update,
            ),
            Topic::BackupPolicy(..) => rows(
                "BackupPolicyInfoUpdated",
                vec![
                    crate::api::resources::backups::views::BackupPolicyView::try_from(
                        self.backups
                            .get_policy(id.unwrap())
                            .await
                            .map_err(failure)?,
                    ).map_err(failure)?,
                ],
                RowStyle::Update,
            ),
            Topic::BackupRuns(..) => rows(
                "BackupRunInfoUpdated",
                self.backups
                    .list_runs(actor, admin, id, 100)
                    .await
                    .map_err(failure)?
                    .into_iter()
                    .map(crate::api::resources::backups::views::BackupRunView::try_from)
                    .collect::<Result<_, _>>().map_err(failure)?,
                RowStyle::Update,
            ),
            Topic::BackupRun(..) => rows(
                "BackupRunInfoUpdated",
                vec![crate::api::resources::backups::views::BackupRunView::try_from(
                    self.backups.get_run(id.unwrap()).await.map_err(failure)?,
                ).map_err(failure)?],
                RowStyle::Update,
            ),
            Topic::BackupRestoreRuns(..) => rows(
                "BackupRestoreRunInfoUpdated",
                self.backups
                    .list_restores(actor, admin, None, id, 100)
                    .await
                    .map_err(failure)?
                    .into_iter()
                    .map(crate::api::resources::backups::views::BackupRestoreRunView::try_from)
                    .collect::<Result<_, _>>().map_err(failure)?,
                RowStyle::Update,
            ),
            Topic::BackupRestoreRun(..) => rows(
                "BackupRestoreRunInfoUpdated",
                vec![
                    crate::api::resources::backups::views::BackupRestoreRunView::try_from(
                        self.backups
                            .get_restore(id.unwrap())
                            .await
                            .map_err(failure)?,
                    ).map_err(failure)?,
                ],
                RowStyle::Update,
            ),
            Topic::BuildProjects => rows(
                "BuildProjectInfoUpdated",
                crate::api::routes::builds::authorized_projects(
                    self.builds.as_ref(),
                    p,
                    self.builds.list(actor, admin).await.map_err(failure)?,
                )
                .await
                .map_err(failure)?,
                RowStyle::Update,
            ),
            Topic::BuildProject(..) => rows(
                "BuildProjectInfoUpdated",
                crate::api::routes::builds::authorized_projects(
                    self.builds.as_ref(),
                    p,
                    vec![self.builds.get(id.unwrap()).await.map_err(failure)?],
                )
                .await
                .map_err(failure)?,
                RowStyle::Update,
            ),
            Topic::BuildAgentPools => rows(
                "BuildAgentPoolInfoUpdated",
                crate::api::routes::builds::authorized_pools(
                    self.builds.as_ref(),
                    p,
                    self.builds
                        .list_pools(actor, admin)
                        .await
                        .map_err(failure)?,
                )
                .await
                .map_err(failure)?,
                RowStyle::Update,
            ),
            Topic::BuildAgentPool(..) => rows(
                "BuildAgentPoolInfoUpdated",
                crate::api::routes::builds::authorized_pools(
                    self.builds.as_ref(),
                    p,
                    vec![self.builds.get_pool(id.unwrap()).await.map_err(failure)?],
                )
                .await
                .map_err(failure)?,
                RowStyle::Update,
            ),
            Topic::BuildRuns(..) => rows(
                "BuildRunInfoUpdated",
                self.builds
                    .list_runs(actor, admin, id, 100)
                    .await
                    .map_err(failure)?
                    .into_iter()
                    .map(crate::api::resources::builds::views::BuildRunView::try_from)
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(failure)?,
                RowStyle::Update,
            ),
            Topic::BuildRun(..) => rows(
                "BuildRunInfoUpdated",
                vec![crate::api::resources::builds::views::BuildRunView::try_from(
                    self.builds.get_run(id.unwrap()).await.map_err(failure)?,
                )
                .map_err(failure)?],
                RowStyle::Update,
            ),
            _ => Err(RealtimeReadError::Authorization),
        }
    }

    async fn container_info(
        &self,
        p: &ActorPrincipal,
        g: &Group,
        e: Option<&PublishedRuntimeEvent>,
        lease: Option<&crate::realtime::AuthorizationLease>,
    ) -> Result<GroupSnapshot, RealtimeReadError> {
        let id = Uuid::parse_str(g.reference().unwrap())
            .map_err(|_| RealtimeReadError::Authorization)?;
        if let Some(change) = e.filter(|e| {
            e.payload["dockerResourceType"] == "containerStats"
                || e.payload["containerPatches"].is_array()
        }) {
            let platform = change.platform_id.ok_or(RealtimeReadError::Authorization)?;
            let identities =
                crate::realtime::shared_reads::identities(&self.platforms, platform, Some(change))
                    .await?;
            let Some(identity) = identities.iter().find(|c| c.id == id) else {
                return Ok(GroupSnapshot::default());
            };
            let mut allowed = p.is_administrator();
            for (kind, owner) in [
                (ResourceType::Platform, Some(identity.platform_id)),
                (ResourceType::Deployment, identity.deployment_id),
                (ResourceType::Stack, identity.stack_id),
            ] {
                if owner.is_some()
                    && self
                        .permission_with_lease(p, kind, owner, None, lease)
                        .await
                        .is_ok()
                {
                    allowed = true;
                    break;
                }
            }
            if !allowed {
                return Err(RealtimeReadError::Authorization);
            }
            if let Some(patch) = change.payload["containerPatches"]
                .as_array()
                .and_then(|patches| patches.iter().find(|patch| patch["id"] == json!(id)))
            {
                let mut value = patch.clone();
                value["id"] = json!(identity.container_id);
                value["resourceId"] = json!(id);
                return event("ReceiveContainerInfoPatch", value);
            }
            let stats = map_stats(change, &identities);
            return match stats
                .into_iter()
                .find(|stat| stat["containerId"] == json!(id))
            {
                Some(stat) => event(
                    "ReceiveContainerInfoPatch",
                    json!({"id":identity.container_id,"resourceId":id,"containerStat":stat}),
                ),
                None => Ok(GroupSnapshot::default()),
            };
        }
        let container = self
            .platforms
            .get_container(id)
            .await
            .map(|value| value.map(crate::api::resources::platforms::views::ContainerView::from))
            .map_err(failure)?
            .ok_or(RealtimeReadError::Authorization)?;
        let mut allowed = p.is_administrator();
        for (kind, id) in [
            (ResourceType::Platform, Some(container.platform_id)),
            (ResourceType::Deployment, container.deployment_id),
            (ResourceType::Stack, container.stack_id),
        ] {
            if id.is_some()
                && self
                    .permission_with_lease(p, kind, id, None, lease)
                    .await
                    .is_ok()
            {
                allowed = true;
                break;
            }
        }
        if !allowed {
            return Err(RealtimeReadError::Authorization);
        }
        if e.and_then(|e| e.platform_id)
            .is_some_and(|id| id != container.platform_id)
        {
            return Ok(GroupSnapshot::default());
        }
        event(
            "ReceiveContainerInfo",
            container_data(container, e).map_err(failure)?,
        )
    }

    async fn activities(
        &self,
        p: &ActorPrincipal,
        g: &Group,
        lease: Option<&crate::realtime::AuthorizationLease>,
    ) -> Result<GroupSnapshot, RealtimeReadError> {
        let kind: citadel_activities::ActivityResourceType =
            serde_json::from_value(json!(g.reference()))
                .map_err(|_| RealtimeReadError::Authorization)?;
        use citadel_activities::ActivityResourceType as A;
        if matches!(
            kind,
            A::User | A::Team | A::Role | A::ServiceAccount | A::OidcProvider | A::License
        ) {
            if !p.is_administrator() {
                return Err(RealtimeReadError::Authorization);
            }
        } else {
            let resource = serde_json::from_value(serde_json::to_value(kind).map_err(failure)?)
                .map_err(|_| RealtimeReadError::Authorization)?;
            self.permission_with_lease(p, resource, g.id(), None, lease)
                .await?;
        }
        let records = self
            .activities
            .list(
                &crate::api::routes::activities::activity_access(p),
                ActivityFilter {
                    resource_id: g.id(),
                    resource_type: Some(kind),
                    page: Some(1),
                    page_size: Some(10),
                    ..Default::default()
                },
            )
            .await
            .map_err(failure)?;
        rows(
            "ActivityEventReceived",
            records
                .items
                .into_iter()
                .map(crate::api::resources::activities::views::map_activity)
                .collect::<Result<Vec<_>, _>>()
                .map_err(failure)?,
            RowStyle::Activity,
        )
    }
}

impl GroupReadPort for ApplicationGroupReader {
    fn read_with_lease<'a>(
        &'a self,
        p: &'a ActorPrincipal,
        g: &'a Group,
        e: Option<&'a PublishedRuntimeEvent>,
        lease: &'a crate::realtime::AuthorizationLease,
    ) -> BoxFuture<'a, Result<GroupSnapshot, RealtimeReadError>> {
        Box::pin(self.read_group(p, g, e, Some(lease)))
    }

    fn terminal_invocation<'a>(
        &'a self,
        p: &'a ActorPrincipal,
        method: &'a str,
        args: &'a [Value],
    ) -> BoxFuture<'a, Result<Option<TerminalInvocation>, RealtimeReadError>> {
        Box::pin(self.resolve_terminal_invocation(p, method, args))
    }
    fn terminal<'a>(
        &'a self,
        p: &'a ActorPrincipal,
        group: &'a Group,
        shell: citadel_platforms::terminal::TerminalShell,
        cancel: &'a tokio_util::sync::CancellationToken,
    ) -> BoxFuture<'a, Result<GroupTerminal, RealtimeReadError>> {
        Box::pin(self.open_terminal(p, group, shell, cancel))
    }
    fn stream<'a>(
        &'a self,
        p: &'a ActorPrincipal,
        g: &'a Group,
        cancel: &'a tokio_util::sync::CancellationToken,
    ) -> BoxFuture<'a, Result<Option<GroupStream>, RealtimeReadError>> {
        Box::pin(self.logs(p, g, cancel))
    }
    fn invocation_group<'a>(
        &'a self,
        p: &'a ActorPrincipal,
        target: &'a str,
        args: &'a [Value],
    ) -> BoxFuture<'a, Result<Option<Group>, RealtimeReadError>> {
        Box::pin(self.log_invocation(p, target, args))
    }
    fn read<'a>(
        &'a self,
        p: &'a ActorPrincipal,
        g: &'a Group,
        e: Option<&'a PublishedRuntimeEvent>,
    ) -> BoxFuture<'a, Result<GroupSnapshot, RealtimeReadError>> {
        Box::pin(self.read_group(p, g, e, None))
    }
}

fn container_data(
    container: crate::api::resources::platforms::views::ContainerView,
    event: Option<&PublishedRuntimeEvent>,
) -> Result<
    crate::api::resources::platforms::container_views::ContainerRuntimeView,
    serde_json::Error,
> {
    let mut value = container.runtime_data()?;
    if let Some(stat) = event
        .filter(|event| {
            event.payload["dockerNodeId"].as_str() == container.docker_node_id.as_deref()
        })
        .and_then(|e| e.payload["stats"].as_array())
        .and_then(|stats| {
            stats
                .iter()
                .find(|s| s["dockerContainerId"] == container.container_id)
        })
    {
        let mut stat = stat.clone();
        stat["containerId"] = json!(container.id);
        value.container_stat = Some(serde_json::from_value(stat)?);
    }
    Ok(value)
}

fn map_stats(
    event: &PublishedRuntimeEvent,
    containers: &[citadel_platforms::ContainerIdentity],
) -> Vec<Value> {
    let Some(stats) = event.payload["stats"].as_array() else {
        return vec![];
    };
    let identities: std::collections::HashMap<_, _> = containers
        .iter()
        .map(|c| ((c.container_id.as_str(), c.docker_node_id.as_deref()), c.id))
        .collect();
    stats
        .iter()
        .filter_map(|stat| {
            let id = stat["dockerContainerId"].as_str()?;
            let container = identities.get(&(id, event.payload["dockerNodeId"].as_str()))?;
            let mut mapped = stat.clone();
            mapped["containerId"] = json!(container);
            Some(mapped)
        })
        .collect()
}

fn live_platform_stats(
    platform: Uuid,
    context: &citadel_platforms::PlatformTelemetryContext,
    sample: &Value,
) -> Option<Value> {
    let created = sample["created"].as_i64()?;
    let metadata = &sample["metadata"];
    let mem_total = metadata["memTotal"].as_i64().unwrap_or(context.mem_total);
    let cpu = sample["cpuUsage"].as_f64().unwrap_or_default();
    let memory = sample["memoryActive"].as_f64().unwrap_or_default();
    Some(json!({
        "platformId":platform,
        "stat": { "created":created, "cpuUsage":if context.cpu_count > 0 {cpu/context.cpu_count as f64} else {cpu},
            "memoryUsage":if mem_total > 0 {memory/mem_total as f64*100.0} else {0.0},
            "rxBytes":sample["rxBytes"], "txBytes":sample["txBytes"],
            "diskUsedBytes":metadata["diskUsedBytes"], "diskTotalBytes":metadata["diskTotalBytes"], "diskUsage":metadata["diskUsage"],
        },
        "memTotal":mem_total,
        "networkCount":metadata["networkCount"].as_i64().unwrap_or(i64::from(context.network_count)),
        "imageCount":metadata["imageCount"].as_i64().unwrap_or(context.image_count),
        "volumeCount":metadata["volumeCount"].as_i64().unwrap_or(i64::from(context.volume_count)),
        "containerCount":metadata.get("containerCount").unwrap_or(&context.descriptor["containerCount"]),
        "containersRunning":metadata.get("containersRunning").unwrap_or(&context.descriptor["containersRunning"]),
        "containersPaused":metadata.get("containersPaused").unwrap_or(&context.descriptor["containersPaused"]),
        "containersStopped":metadata.get("containersStopped").unwrap_or(&context.descriptor["containersStopped"]),
    }))
}
