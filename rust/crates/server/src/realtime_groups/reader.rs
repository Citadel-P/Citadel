use super::*;
use crate::realtime::topic::Topic;
use citadel_application::{ActivityFilter, ActivityService};
use citadel_automation::AutomationStore;
use citadel_backups::BackupStore;
use citadel_builds::BuildStore;
use citadel_deployments::DeploymentRepository;
use citadel_domain::{PermissionLevel, ResourceType, SpecificPermission};
use citadel_identity::IdentityService;
use citadel_platforms::PlatformReadService;
use citadel_resources::ResourceMetadataStore;
use citadel_stacks::StackRepository;
use citadel_swarm_services::SwarmServiceRepository;
use std::sync::Arc;

#[path = "logs.rs"]
mod logs;
#[path = "terminal.rs"]
mod terminal;
#[cfg(test)]
#[path = "reader_tests.rs"]
mod tests;

// Reuse the same authorized read models as HTTP. No internal HTTP requests,
// browser query policy, copied SQL schema, or second resource cache.
#[derive(Clone)]
pub struct ApplicationGroupReader {
    pub identity: Arc<IdentityService>,
    pub platforms: Arc<PlatformReadService>,
    pub deployments: Arc<dyn DeploymentRepository>,
    pub stacks: Arc<dyn StackRepository>,
    pub services: Arc<dyn SwarmServiceRepository>,
    pub resources: Arc<dyn ResourceMetadataStore>,
    pub automation: Arc<dyn AutomationStore>,
    pub builds: Arc<dyn BuildStore>,
    pub backups: Arc<dyn BackupStore>,
    pub activities: Arc<ActivityService>,
    pub alerts: Arc<dyn citadel_alerts::AlertStore>,
    pub docker: crate::platforms_http::PlatformsHttpState,
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
    async fn deployment_permission<P: citadel_domain::PermissionPolicy>(
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
        if p.is_administrator() {
            return Ok(());
        }
        use citadel_domain::PermissionPolicy;
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
            Topic::Activity { .. } => return self.activities(p, g).await,
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
            Topic::ContainerInfo(..) => return self.container_info(p, g, e).await,
            _ => id,
        };
        self.permission(
            p,
            kind,
            permission_id,
            matches!(
                g.topic(),
                Topic::BackupRestoreRun(..) | Topic::BackupRestoreRuns(..)
            )
            .then_some(SpecificPermission::Restore),
        )
        .await?;
        if matches!(g.topic(), Topic::SwarmServices(..)) {
            self.permission(p, Platform, id, None).await?;
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
            self.permission(p, Platform, Some(service.platform_id), None)
                .await?;
        }
        let sample = e.is_some_and(|e| e.payload["dockerResourceType"] == "containerStats");
        match g.topic() {
            Topic::Platforms => {
                if sample {
                    let platform = self
                        .platforms
                        .get_platform(e.unwrap().platform_id.unwrap())
                        .await
                        .map_err(failure)?
                        .ok_or(RealtimeReadError::Authorization)?;
                    if self
                        .permission(p, Platform, Some(platform.id), None)
                        .await
                        .is_err()
                    {
                        return Ok(GroupSnapshot::default());
                    }
                    let Some(stat) = platform.stats.as_ref().and_then(|s| s.first()) else {
                        return Ok(GroupSnapshot::default());
                    };
                    return event(
                        "PlatformStatsUpdated",
                        json!({"platformId":platform.id,"stat":stat,"memTotal":platform.mem_total,"networkCount":platform.network_count,"imageCount":platform.image_count,"volumeCount":platform.volume_count,
                        "containerCount":platform.platform_descriptor["containerCount"],"containersRunning":platform.platform_descriptor["containersRunning"],"containersPaused":platform.platform_descriptor["containersPaused"],"containersStopped":platform.platform_descriptor["containersStopped"]}),
                    );
                }
                rows(
                    "PlatformUpdated",
                    self.platforms
                        .list_authorized(actor, admin, &[])
                        .await
                        .map_err(failure)?,
                    RowStyle::Platforms,
                )
            }
            Topic::Containers(..) => {
                let containers = self
                    .platforms
                    .list_containers(id.unwrap())
                    .await
                    .map_err(failure)?;
                if sample {
                    let stats = map_stats(e.unwrap(), &containers);
                    return event("ContainersStatsUpdated", stats);
                }
                event("ContainersInfoUpdated", json!({"containers":containers}))
            }
            Topic::Images(..) => {
                if sample {
                    return Ok(GroupSnapshot::default());
                }
                event(
                    "ImagesInfoUpdated",
                    json!({"images":self.platforms.list_images(id.unwrap()).await.map_err(failure)?}),
                )
            }
            Topic::DockerDaemon(..) => {
                if sample {
                    return Ok(GroupSnapshot::default());
                }
                let mut result =
                    crate::platforms_http::realtime_daemon_snapshot(&self.docker, p, id.unwrap())
                        .await?;
                result.rows.extend(
                    rows(
                        "ContainerEventReceived",
                        self.platforms
                            .list_containers(id.unwrap())
                            .await
                            .map_err(failure)?,
                        RowStyle::DockerResource,
                    )?
                    .rows,
                );
                result.rows.extend(
                    rows(
                        "ImageEventReceived",
                        self.platforms
                            .list_images(id.unwrap())
                            .await
                            .map_err(failure)?,
                        RowStyle::DockerResource,
                    )?
                    .rows,
                );
                result.events.push(ClientEvent::new("SwarmInventoryUpdated",vec![json!({
                    "platformId":id,
                    "nodes":{"items":self.platforms.list_swarm_nodes(id.unwrap()).await.map_err(failure)?},
                    "services":{"items":self.platforms.list_swarm_services(id.unwrap()).await.map_err(failure)?},
                    "tasks":{"items":self.platforms.list_swarm_tasks(id.unwrap(),None,1000).await.map_err(failure)?},
                    "networks":{"items":self.platforms.list_swarm_networks(id.unwrap()).await.map_err(failure)?},
                    "secrets":{"items":self.platforms.list_swarm_secrets(id.unwrap()).await.map_err(failure)?},
                    "configs":{"items":self.platforms.list_swarm_configs(id.unwrap()).await.map_err(failure)?},
                })]));
                Ok(result)
            }
            Topic::Deployments => rows(
                "DeploymentInfoUpdated",
                self.deployments
                    .list_authorized(actor, admin, &Default::default())
                    .await
                    .map_err(failure)?
                    .into_iter()
                    .map(crate::api::deployments::views::DeploymentView::from)
                    .collect(),
                RowStyle::Update,
            ),
            Topic::Deployment(..) => rows(
                "DeploymentInfoUpdated",
                vec![crate::api::deployments::views::DeploymentView::from(
                    self.deployments
                        .get_authorized(actor, admin, id.unwrap())
                        .await
                        .map_err(failure)?,
                )],
                RowStyle::Update,
            ),
            Topic::Stacks => rows(
                "StackInfoUpdated",
                self.stacks
                    .list_authorized(actor, admin, &Default::default())
                    .await
                    .map_err(failure)?
                    .into_iter()
                    .map(crate::api::stacks::views::StackView::from)
                    .collect(),
                RowStyle::Update,
            ),
            Topic::Stack(..) => rows(
                "StackInfoUpdated",
                vec![crate::api::stacks::views::StackView::from(
                    self.stacks
                        .get_authorized(actor, admin, id.unwrap())
                        .await
                        .map_err(failure)?,
                )],
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
                    .list_containers(platform)
                    .await
                    .map_err(failure)?
                    .into_iter()
                    .filter(|c| c.stack_id == id)
                    .map(|c| container_data(c, e))
                    .collect::<Vec<_>>();
                event("ReceiveStackContainersInfo", containers)
            }
            Topic::SwarmService(..) => rows(
                "SwarmServiceInfoUpdated",
                vec![
                    crate::api::swarm_services::views::ManagedSwarmServiceView::from(
                        self.services
                            .get_authorized(actor, admin, id.unwrap())
                            .await
                            .map_err(failure)?,
                    ),
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
                    .map(crate::api::swarm_services::views::ManagedSwarmServiceView::from)
                    .collect(),
                RowStyle::Update,
            ),
            Topic::GitRepositories => rows(
                "GitRepositoryInfoUpdated",
                self.resources
                    .list_git_repositories(actor, admin)
                    .await
                    .map_err(failure)?,
                RowStyle::Update,
            ),
            Topic::GitRepo(..) => rows(
                "GitRepositoryInfoUpdated",
                vec![
                    self.resources
                        .get_git_repository(id.unwrap())
                        .await
                        .map_err(failure)?,
                ],
                RowStyle::Update,
            ),
            Topic::AutomationActions => rows(
                "AutomationActionInfoUpdated",
                crate::automation_http::authorized_actions(
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
                crate::automation_http::authorized_actions(
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
                    .map_err(failure)?,
                RowStyle::Update,
            ),
            Topic::BackupRepository(..) => rows(
                "BackupRepositoryInfoUpdated",
                vec![
                    self.backups
                        .get_repository(id.unwrap())
                        .await
                        .map_err(failure)?,
                ],
                RowStyle::Update,
            ),
            Topic::BackupPolicies => rows(
                "BackupPolicyInfoUpdated",
                self.backups
                    .list_policies(actor, admin)
                    .await
                    .map_err(failure)?,
                RowStyle::Update,
            ),
            Topic::BackupPolicy(..) => rows(
                "BackupPolicyInfoUpdated",
                vec![
                    self.backups
                        .get_policy(id.unwrap())
                        .await
                        .map_err(failure)?,
                ],
                RowStyle::Update,
            ),
            Topic::BackupRuns(..) => rows(
                "BackupRunInfoUpdated",
                self.backups
                    .list_runs(actor, admin, id, 100)
                    .await
                    .map_err(failure)?,
                RowStyle::Update,
            ),
            Topic::BackupRun(..) => rows(
                "BackupRunInfoUpdated",
                vec![self.backups.get_run(id.unwrap()).await.map_err(failure)?],
                RowStyle::Update,
            ),
            Topic::BackupRestoreRuns(..) => rows(
                "BackupRestoreRunInfoUpdated",
                self.backups
                    .list_restores(actor, admin, None, id, 100)
                    .await
                    .map_err(failure)?,
                RowStyle::Update,
            ),
            Topic::BackupRestoreRun(..) => rows(
                "BackupRestoreRunInfoUpdated",
                vec![
                    self.backups
                        .get_restore(id.unwrap())
                        .await
                        .map_err(failure)?,
                ],
                RowStyle::Update,
            ),
            Topic::BuildProjects => rows(
                "BuildProjectInfoUpdated",
                crate::builds_http::authorized_projects(
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
                crate::builds_http::authorized_projects(
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
                crate::builds_http::authorized_pools(
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
                crate::builds_http::authorized_pools(
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
                    .map_err(failure)?,
                RowStyle::Update,
            ),
            Topic::BuildRun(..) => rows(
                "BuildRunInfoUpdated",
                vec![self.builds.get_run(id.unwrap()).await.map_err(failure)?],
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
    ) -> Result<GroupSnapshot, RealtimeReadError> {
        let id = Uuid::parse_str(g.reference().unwrap())
            .map_err(|_| RealtimeReadError::Authorization)?;
        let container = self
            .platforms
            .get_container(id)
            .await
            .map_err(failure)?
            .ok_or(RealtimeReadError::Authorization)?;
        let mut allowed = p.is_administrator();
        for (kind, id) in [
            (ResourceType::Platform, Some(container.platform_id)),
            (ResourceType::Deployment, container.deployment_id),
            (ResourceType::Stack, container.stack_id),
        ] {
            if id.is_some() && self.permission(p, kind, id, None).await.is_ok() {
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
        event("ReceiveContainerInfo", container_data(container, e))
    }

    async fn activities(
        &self,
        p: &ActorPrincipal,
        g: &Group,
    ) -> Result<GroupSnapshot, RealtimeReadError> {
        let kind: citadel_domain::ActivityResourceType =
            serde_json::from_value(json!(g.reference()))
                .map_err(|_| RealtimeReadError::Authorization)?;
        use citadel_domain::ActivityResourceType as A;
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
            self.permission(p, resource, g.id(), None).await?;
        }
        let records = self
            .activities
            .list(
                p,
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
                .map(crate::activities_http::map_activity)
                .collect::<Result<Vec<_>, _>>()
                .map_err(failure)?,
            RowStyle::Activity,
        )
    }
}

impl GroupReadPort for ApplicationGroupReader {
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
        Box::pin(self.read_group(p, g, e))
    }
}

fn container_data(
    container: citadel_platforms::ContainerView,
    event: Option<&PublishedRuntimeEvent>,
) -> Value {
    let mut value = container.runtime_data();
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
        value["containerStat"] = stat.clone();
        value["containerStat"]["containerId"] = json!(container.id);
    }
    value
}

fn map_stats(
    event: &PublishedRuntimeEvent,
    containers: &[citadel_platforms::ContainerView],
) -> Vec<Value> {
    let Some(stats) = event.payload["stats"].as_array() else {
        return vec![];
    };
    stats
        .iter()
        .filter_map(|stat| {
            let id = stat["dockerContainerId"].as_str()?;
            let container = containers.iter().find(|c| {
                c.container_id == id
                    && c.docker_node_id.as_deref() == event.payload["dockerNodeId"].as_str()
            })?;
            let mut mapped = stat.clone();
            mapped["containerId"] = json!(container.id);
            Some(mapped)
        })
        .collect()
}
