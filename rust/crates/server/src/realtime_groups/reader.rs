use super::*;
use citadel_application::{ActivityFilter, ActivityService};
use citadel_automation::AutomationStore;
use citadel_backups::BackupStore;
use citadel_builds::BuildStore;
use citadel_deployments::DeploymentStore;
use citadel_domain::{PermissionLevel, ResourceType, SpecificPermission};
use citadel_identity::IdentityService;
use citadel_platforms::PlatformReadService;
use citadel_resources::ResourceMetadataStore;
use citadel_stacks::StackStore;
use citadel_swarm_services::SwarmServiceStore;
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
    pub deployments: Arc<dyn DeploymentStore>,
    pub stacks: Arc<dyn StackStore>,
    pub services: Arc<dyn SwarmServiceStore>,
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
        let grant = match id {
            Some(id) => self.identity.permission_for_resource(p, kind, id).await,
            None => self.identity.global_permission(p, kind).await,
        }
        .map_err(failure)?;
        if grant.is_some_and(|g| {
            g.level.grants(PermissionLevel::Read) && specific.is_none_or(|s| g.has_specific(s))
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
        if g.kind.ends_with("-exec") {
            self.terminal_target(p, g).await?;
            return Ok(GroupSnapshot::default());
        }
        if g.kind.ends_with("-log") {
            self.log_targets(p, g).await?;
            return Ok(GroupSnapshot::default());
        }
        use ResourceType::*;
        let id = g.id;
        let actor = p.actor_id;
        let admin = p.is_administrator();
        let kind = match g.kind.as_str() {
            "platforms" | "containers" | "images" | "docker-daemon" => Platform,
            "deployment" | "deployments" => Deployment,
            "stack" | "stacks" | "stack-info" => Stack,
            "swarm-service" | "swarm-services" => SwarmService,
            "git-repo" | "git-repositories" => GitRepository,
            "automation-action" | "automation-actions" => AutomationAction,
            "backup-repository" | "backup-repositories" => BackupRepository,
            "backup-policy"
            | "backup-policies"
            | "backup-runs"
            | "backup-run"
            | "backup-restore-runs"
            | "backup-restore-run" => BackupPolicy,
            "build-project" | "build-projects" | "build-runs" | "build-run" => Build,
            "build-agent-pool" | "build-agent-pools" => BuildAgentPool,
            "container-info" => Platform, // resolved through its actual owner below
            "activity" => return self.activities(p, g).await,
            "alert-events" => {
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
        let permission_id = match g.kind.as_str() {
            "swarm-services" => None,
            "backup-run" => Some(
                self.backups
                    .get_run(id.unwrap())
                    .await
                    .map_err(failure)?
                    .backup_policy_id,
            ),
            "backup-restore-run" => {
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
            "build-run" => Some(
                self.builds
                    .get_run(id.unwrap())
                    .await
                    .map_err(failure)?
                    .build_project_id,
            ),
            "container-info" => return self.container_info(p, g, e).await,
            _ => id,
        };
        self.permission(
            p,
            kind,
            permission_id,
            g.kind
                .starts_with("backup-restore")
                .then_some(SpecificPermission::Restore),
        )
        .await?;
        if g.kind == "swarm-services" {
            self.permission(p, Platform, id, None).await?;
        }
        if g.kind == "docker-daemon"
            && e.is_some_and(|event| event.payload["dockerResourceType"] == "nodeAgentCoverage")
        {
            return event("SwarmNodeAgentCoverageChanged", id.unwrap());
        }
        if g.kind == "build-run"
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
        if g.kind == "swarm-service" {
            let service = self
                .services
                .get_authorized(actor, admin, id.unwrap())
                .await
                .map_err(failure)?;
            self.permission(p, Platform, Some(service.platform_id), None)
                .await?;
        }
        let sample = e.is_some_and(|e| e.payload["dockerResourceType"] == "containerStats");
        match g.kind.as_str() {
            "platforms" => {
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
            "containers" => {
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
            "images" => {
                if sample {
                    return Ok(GroupSnapshot::default());
                }
                event(
                    "ImagesInfoUpdated",
                    json!({"images":self.platforms.list_images(id.unwrap()).await.map_err(failure)?}),
                )
            }
            "docker-daemon" => {
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
            "deployments" => rows(
                "DeploymentInfoUpdated",
                self.deployments
                    .list_authorized(actor, admin, &Default::default())
                    .await
                    .map_err(failure)?,
                RowStyle::Update,
            ),
            "deployment" => rows(
                "DeploymentInfoUpdated",
                vec![
                    self.deployments
                        .get_authorized(actor, admin, id.unwrap())
                        .await
                        .map_err(failure)?,
                ],
                RowStyle::Update,
            ),
            "stacks" => rows(
                "StackInfoUpdated",
                self.stacks
                    .list_authorized(actor, admin, &Default::default())
                    .await
                    .map_err(failure)?,
                RowStyle::Update,
            ),
            "stack" => rows(
                "StackInfoUpdated",
                vec![
                    self.stacks
                        .get_authorized(actor, admin, id.unwrap())
                        .await
                        .map_err(failure)?,
                ],
                RowStyle::Update,
            ),
            "stack-info" => {
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
            "swarm-service" => rows(
                "SwarmServiceInfoUpdated",
                vec![
                    self.services
                        .get_authorized(actor, admin, id.unwrap())
                        .await
                        .map_err(failure)?,
                ],
                RowStyle::Update,
            ),
            "swarm-services" => rows(
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
                    .map_err(failure)?,
                RowStyle::Update,
            ),
            "git-repositories" => rows(
                "GitRepositoryInfoUpdated",
                self.resources
                    .list_git_repositories(actor, admin)
                    .await
                    .map_err(failure)?,
                RowStyle::Update,
            ),
            "git-repo" => rows(
                "GitRepositoryInfoUpdated",
                vec![
                    self.resources
                        .get_git_repository(id.unwrap())
                        .await
                        .map_err(failure)?,
                ],
                RowStyle::Update,
            ),
            "automation-actions" => rows(
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
            "automation-action" => rows(
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
            "backup-repositories" => rows(
                "BackupRepositoryInfoUpdated",
                self.backups
                    .list_repositories(actor, admin)
                    .await
                    .map_err(failure)?,
                RowStyle::Update,
            ),
            "backup-repository" => rows(
                "BackupRepositoryInfoUpdated",
                vec![
                    self.backups
                        .get_repository(id.unwrap())
                        .await
                        .map_err(failure)?,
                ],
                RowStyle::Update,
            ),
            "backup-policies" => rows(
                "BackupPolicyInfoUpdated",
                self.backups
                    .list_policies(actor, admin)
                    .await
                    .map_err(failure)?,
                RowStyle::Update,
            ),
            "backup-policy" => rows(
                "BackupPolicyInfoUpdated",
                vec![
                    self.backups
                        .get_policy(id.unwrap())
                        .await
                        .map_err(failure)?,
                ],
                RowStyle::Update,
            ),
            "backup-runs" => rows(
                "BackupRunInfoUpdated",
                self.backups
                    .list_runs(actor, admin, id, 100)
                    .await
                    .map_err(failure)?,
                RowStyle::Update,
            ),
            "backup-run" => rows(
                "BackupRunInfoUpdated",
                vec![self.backups.get_run(id.unwrap()).await.map_err(failure)?],
                RowStyle::Update,
            ),
            "backup-restore-runs" => rows(
                "BackupRestoreRunInfoUpdated",
                self.backups
                    .list_restores(actor, admin, None, id, 100)
                    .await
                    .map_err(failure)?,
                RowStyle::Update,
            ),
            "backup-restore-run" => rows(
                "BackupRestoreRunInfoUpdated",
                vec![
                    self.backups
                        .get_restore(id.unwrap())
                        .await
                        .map_err(failure)?,
                ],
                RowStyle::Update,
            ),
            "build-projects" => rows(
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
            "build-project" => rows(
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
            "build-agent-pools" => rows(
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
            "build-agent-pool" => rows(
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
            "build-runs" => rows(
                "BuildRunInfoUpdated",
                self.builds
                    .list_runs(actor, admin, id, 100)
                    .await
                    .map_err(failure)?,
                RowStyle::Update,
            ),
            "build-run" => rows(
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
        let id = Uuid::parse_str(g.reference.as_deref().unwrap())
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
        let kind: citadel_domain::ActivityResourceType = serde_json::from_value(json!(g.reference))
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
            self.permission(p, resource, g.id, None).await?;
        }
        let records = self
            .activities
            .list(
                p,
                ActivityFilter {
                    resource_id: g.id,
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
    let mut value = json!({
        "id":container.container_id,"name":container.name,"platformId":container.platform_id,
        "image":container.docker_image_id,"imageId":container.docker_image_id,"state":container.state,
        "controlState":container.control_state,"created":container.created,"stack":container.stack,
        "ports":container.ports,"containerStat":container.last_stats,
        "isSystem":container.is_system,"systemRole":container.system_role,
        "hasCitadelOwnershipLabels":container.has_citadel_ownership_labels,
        "isSwarmTask":container.is_swarm_task,"dockerNodeId":container.docker_node_id,
        "deploymentId":container.deployment_id,"stackId":container.stack_id,
        "capabilities":container.capabilities,
    });
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
