use super::*;
use super::{platform_stats::live_platform_stats, runtime};
use crate::realtime::topic::Topic;
use citadel_activities::{ActivityFilter, ActivityService};
use citadel_backups::BackupPersistence;
use citadel_builds::BuildRepository;
use citadel_deployments::DeploymentRepository;
use citadel_identity::IdentityService;
use citadel_platforms::PlatformReadService;
use citadel_primitives::{PermissionLevel, ResourceType, SpecificPermission};
use citadel_scripts::ScriptRepository;
use citadel_stacks::StackRepository;
use citadel_swarm_services::SwarmServiceRepository;
use std::sync::Arc;

// Reuse the same authorized read models as HTTP. No internal HTTP requests,
// browser query policy, or copied SQL schema. Container rows are shared only
// during delivery of one invalidation, after each reader checks authorization.
#[derive(Clone)]
pub struct ApplicationGroupReader {
    pub snapshot_limit: usize,
    pub snapshot_timeout: std::time::Duration,
    pub instance: Option<Arc<citadel_instance::InstanceService>>,
    pub admission: citadel_runtime::admission::AdmissionGate,
    pub image_store: Arc<dyn citadel_platforms::image_mutations::ImageMutationStore>,
    pub image_updates: Arc<citadel_platforms::image_updates::ImageUpdateService>,
    pub registries: Arc<dyn citadel_registries::RegistryRepository>,
    pub copy_eligibility: Option<Arc<citadel_resource_copies::CopyService>>,
    pub backup_coverage:
        Option<Arc<crate::api::resources::backups::coverage::BackupCoverageService>>,
    pub git_repositories: Arc<dyn citadel_git::GitRepositoryPersistence>,
    pub identity: Arc<IdentityService>,
    pub platforms: Arc<PlatformReadService>,
    pub deployments: Arc<dyn DeploymentRepository>,
    pub stacks: Arc<dyn StackRepository>,
    pub services: Arc<dyn SwarmServiceRepository>,
    pub scripts: Arc<dyn ScriptRepository>,
    pub builds: Arc<dyn BuildRepository>,
    pub backups: Arc<dyn BackupPersistence>,
    pub activities: Arc<ActivityService>,
    pub alerts: Arc<dyn citadel_alerts::AlertRepository>,
    pub runtime: Arc<dyn citadel_platforms::runtime_provider::PlatformRuntimeProvider>,
    pub statistics: Arc<dyn citadel_platforms::StatisticsReader>,
    pub realtime: Option<crate::realtime::RealtimeHub>,
}

pub(super) fn failure(error: impl std::fmt::Display) -> RealtimeReadError {
    RealtimeReadError::Storage(error.to_string())
}
pub(super) fn rows<T: Serialize>(
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
// Health/state observations carry exact committed Docker identities. Keep list delivery
// scoped to their owners without treating omitted owners as deleted. Runtime
// reconciliation refreshes owners on that Platform; parent CRUD retains full snapshots.
// Compute event scope once, before authorized queries, enrichment and serialization.
pub(super) struct WorkloadSelection {
    pub platform: Option<Uuid>,
    references: Option<Vec<String>>,
    pub(super) owners: Option<Vec<Uuid>>,
}
impl WorkloadSelection {
    pub fn new(event: Option<&PublishedRuntimeEvent>, owner_key: &str) -> Self {
        let owners = Self::coverage_ids(event, owner_key);
        let event = event.filter(|e| e.resource_type == "Platform" && e.platform_id.is_some());
        Self {
            owners,
            platform: event.and_then(|e| e.platform_id),
            references: event.and_then(|e| {
                let action = e.payload["action"].as_str().unwrap_or("");
                (action.starts_with("health")
                    || matches!(
                        action,
                        "oom" | "start" | "stop" | "die" | "kill" | "pause" | "unpause"
                    ))
                .then(|| e.container_ids())
                .flatten()
            }),
        }
    }
    fn coverage_ids(event: Option<&PublishedRuntimeEvent>, key: &str) -> Option<Vec<Uuid>> {
        let event =
            event.filter(|e| e.resource_type == "BackupCoverage" && e.payload["resync"] != true)?;
        event.payload[key]
            .as_array()?
            .iter()
            .map(|id| id.as_str()?.parse().ok())
            .collect()
    }
    pub fn includes(
        &self,
        id: Uuid,
        platform: Option<Uuid>,
        health: &citadel_primitives::health::WorkloadHealth,
    ) -> bool {
        self.owners.as_ref().is_none_or(|ids| ids.contains(&id))
            && self.platform.is_none_or(|scope| platform == Some(scope))
            && self.references.as_ref().is_none_or(|references| {
                health.members_truncated
                    || health.members.iter().any(|member| {
                        member
                            .docker_container_id
                            .as_ref()
                            .is_some_and(|docker| references.iter().any(|id| id == docker))
                    })
            })
    }
    pub fn style(&self) -> RowStyle {
        if self.platform.is_some() || self.owners.is_some() {
            RowStyle::UpdatePatch
        } else {
            RowStyle::Update
        }
    }
}

pub(super) fn event(
    target: &'static str,
    value: impl Serialize,
) -> Result<GroupSnapshot, RealtimeReadError> {
    Ok(GroupSnapshot {
        rows: vec![],
        events: vec![ClientEvent::new(
            target,
            vec![serde_json::to_value(value).map_err(failure)?],
        )],
    })
}

impl ApplicationGroupReader {
    pub(super) async fn platform_capabilities(
        &self,
        principal: &ActorPrincipal,
        platform: Uuid,
        lease: Option<&crate::realtime::AuthorizationLease>,
    ) -> Result<crate::api::resources::platforms::views::PlatformCapabilitiesView, RealtimeReadError>
    {
        match lease {
            Some(lease) => {
                lease
                    .platform_capabilities(&self.identity, principal, platform)
                    .await
            }
            None => Ok(runtime::daemon_capabilities(self, principal, platform)
                .await?
                .0),
        }
    }

    pub(super) async fn platform_views(
        &self,
        principal: &ActorPrincipal,
        platforms: Vec<citadel_platforms::PlatformDetails>,
        lease: Option<&crate::realtime::AuthorizationLease>,
    ) -> Result<Vec<crate::api::resources::platforms::views::PlatformView>, RealtimeReadError> {
        use crate::api::resources::platforms::{
            capabilities::{permission_for, platform_capabilities},
            views::PlatformView,
        };
        let permissions = if lease.is_some() || principal.is_administrator() || platforms.is_empty()
        {
            Default::default()
        } else {
            let ids: Vec<_> = platforms.iter().map(|platform| platform.id).collect();
            self.platforms
                .permissions_for_platforms(principal.actor_id, &ids)
                .await
                .map_err(failure)?
        };
        let mut views = Vec::with_capacity(platforms.len());
        for platform in platforms {
            let mut view = PlatformView::try_from(platform).map_err(failure)?;
            // Shared observations never contain actor-specific permissions.
            let capabilities = match lease {
                Some(lease) => {
                    lease
                        .platform_capabilities(&self.identity, principal, view.id)
                        .await?
                }
                None => platform_capabilities(permission_for(
                    &permissions,
                    view.id,
                    principal.is_administrator(),
                )),
            };
            if !capabilities.can_read {
                return Err(RealtimeReadError::Authorization);
            }
            view.capabilities = Some(capabilities);
            views.push(view);
        }
        Ok(views)
    }

    pub(super) fn container_patch_snapshot(
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

    pub(super) async fn changed_containers(
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

    pub(super) async fn deployment_permission<P: citadel_primitives::PermissionPolicy>(
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

    pub(super) async fn permission(
        &self,
        p: &ActorPrincipal,
        kind: ResourceType,
        id: Option<Uuid>,
        specific: Option<SpecificPermission>,
    ) -> Result<(), RealtimeReadError> {
        self.permission_with_lease(p, kind, id, specific, None)
            .await
    }

    pub(super) async fn permission_with_lease(
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
        if let (Some(change), Some(lease)) = (e, lease)
            && change.payload["dockerResourceType"] == "containerStats"
            && let Some(platform) = change.platform_id
            && !lease.matches_scope(&g.name, platform)
        {
            return Ok(GroupSnapshot::default());
        }
        let mut snapshot = self.read_group_data(p, g, e, lease).await?;
        if let Topic::DockerDaemon(id) = g.topic() {
            runtime::attach_swarm_overview(self, p, *id, lease, e, &mut snapshot).await?;
        }
        Ok(snapshot)
    }

    async fn read_group_data(
        &self,
        p: &ActorPrincipal,
        g: &Group,
        e: Option<&PublishedRuntimeEvent>,
        lease: Option<&crate::realtime::AuthorizationLease>,
    ) -> Result<GroupSnapshot, RealtimeReadError> {
        // Leave time for mapping and delivering the authoritative rows even when
        // the base read has already used the optional hints' share of the deadline.
        let catalog_limit =
            i64::try_from(self.snapshot_limit.saturating_add(1)).unwrap_or(i64::MAX);
        let hint_deadline = tokio::time::Instant::now() + self.snapshot_timeout.mul_f32(0.8);
        if matches!(g.topic(), Topic::InstanceJobs | Topic::InstanceSchedules) {
            self.jobs_service(p)?;
            return Ok(GroupSnapshot::default());
        }
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
        if let Topic::ContainerProcesses(id) = g.topic() {
            return self.processes(p, *id, e, lease).await;
        }
        if let Topic::ContainerHealth(id) = g.topic() {
            let container = self
                .platforms
                .container_identity(*id)
                .await
                .map_err(failure)?
                .ok_or(RealtimeReadError::Authorization)?;
            // Match the HTTP history permission; workload-only access to
            // container-info must not grant Platform history access.
            self.permission_with_lease(
                p,
                ResourceType::Platform,
                Some(container.platform_id),
                None,
                lease,
            )
            .await?;
            if let Some(change) = e
                && (change
                    .platform_id
                    .is_some_and(|platform| platform != container.platform_id)
                    || (change.platform_id.is_none()
                        && change.resource_id != container.platform_id)
                    || change
                        .container_ids()
                        .is_some_and(|ids| !ids.iter().any(|id| id == &container.container_id)))
            {
                return Ok(GroupSnapshot::default());
            }
            // History is an HTTP read. Joining (including after reconnect) also
            // invalidates it to cover changes between the GET and subscription.
            return event("ContainerHealthHistoryChanged", json!({"resourceId": id}));
        }
        use ResourceType::*;
        let id = g.id();
        let actor = p.actor_id;
        let admin = p.is_administrator();
        let kind = match g.topic() {
            Topic::Platforms
            | Topic::Containers(..)
            | Topic::Images(..)
            | Topic::Volumes(..)
            | Topic::DockerDaemon(..) => Platform,
            Topic::Deployment(..) | Topic::Deployments => Deployment,
            Topic::Stack(..) | Topic::Stacks | Topic::StackInfo(..) => Stack,
            Topic::ManagedSwarmServices | Topic::SwarmService(..) | Topic::SwarmServices(..) => {
                SwarmService
            }
            Topic::Registries => {
                let resources = self
                    .registries
                    .list_registries(
                        actor,
                        admin,
                        citadel_registries::RegistryListFilter::Catalog {
                            include_disabled: g.include_disabled,
                            tag_names: &g.tags,
                        },
                    )
                    .await
                    .map_err(failure)?;
                let views = crate::api::resources::registries::views::authorized_registries(
                    &self.identity,
                    p,
                    resources,
                )
                .await
                .map_err(failure)?;
                return rows("RegistryInfoUpdated", views, RowStyle::Update);
            }
            Topic::GitRepo(..) | Topic::GitRepositories => GitRepository,
            Topic::Script(..) | Topic::Scripts | Topic::ScriptRuns(..) => Script,
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
            Topic::ResourceCopy(id) => {
                let copies = self
                    .copy_eligibility
                    .as_ref()
                    .ok_or(RealtimeReadError::Authorization)?;
                let progress = copies.get(actor, *id).await.map_err(|error| match error {
                    citadel_resource_copies::CopyError::NotFound => {
                        RealtimeReadError::Authorization
                    }
                    error => failure(error),
                })?;
                return event("ResourceCopyInfoUpdated", progress);
            }
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
                    .run_policy_id(id.unwrap())
                    .await
                    .map_err(failure)?,
            ),
            Topic::BackupRestoreRun(..) => Some(
                self.backups
                    .restore_policy_id(id.unwrap())
                    .await
                    .map_err(failure)?,
            ),
            Topic::BuildRun(run) => return self.build_run_snapshot(p, g, *run, e, lease).await,
            Topic::ContainerInfo(..) => return self.container_info(p, g, e, lease).await,
            _ => id,
        };
        if !g.uses_row_authorization() {
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
        }
        if e.is_none()
            && matches!(g.topic(), Topic::BackupRun(_) | Topic::BackupRestoreRun(_))
            && let (Some(lease), Some(owner)) = (lease, permission_id)
        {
            lease.set_scope(&g.name, owner);
        }
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
                    let shared_context =
                        crate::realtime::shared_reads::telemetry(&self.platforms, platform_id, e)
                            .await?;
                    let Some(context) = shared_context.as_ref().as_ref() else {
                        return Ok(GroupSnapshot::default());
                    };
                    let Some(payload) = live_platform_stats(
                        platform_id,
                        context,
                        &e.unwrap().payload["platformSample"],
                        e.unwrap().payload["dockerNodeId"].as_str(),
                    ) else {
                        return Ok(GroupSnapshot::default());
                    };
                    return event("PlatformStatsUpdated", payload);
                }
                if let Some(platform_id) =
                    e.and_then(|e| e.platform_id).filter(|_| g.tags.is_empty())
                {
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
                            .into_owned()
                    } else {
                        None
                    };
                    return rows(
                        "PlatformUpdated",
                        self.platform_views(p, platform.into_iter().collect(), lease)
                            .await?,
                        RowStyle::PlatformPatch(platform_id),
                    );
                }
                rows(
                    "PlatformUpdated",
                    self.platform_views(
                        p,
                        self.platforms
                            .list_authorized(actor, admin, &g.tags)
                            .await
                            .map_err(failure)?,
                        lease,
                    )
                    .await?,
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
                let capabilities = self.platform_capabilities(p, id.unwrap(), lease).await?;
                if let Some(references) = e.and_then(PublishedRuntimeEvent::container_ids) {
                    let mut containers = self
                        .changed_containers(e.unwrap(), id.unwrap(), &references)
                        .await?;
                    for container in &mut containers {
                        container.capabilities = Some(
                            crate::api::resources::platforms::capabilities::container_capabilities(
                                capabilities,
                                container,
                                p.is_administrator(),
                            ),
                        );
                    }
                    let stack_coverage = match &self.backup_coverage {
                        Some(service) => {
                            service
                                .enrich_containers(
                                    p,
                                    id.unwrap(),
                                    &e.unwrap().reads.affected_stacks,
                                    &mut containers,
                                )
                                .await
                        }
                        None => Default::default(),
                    };
                    crate::api::resources::platforms::container_views::enrich_container_owner_names(&self.platforms, p, &mut containers).await.map_err(failure)?;
                    let mut changes = std::collections::BTreeMap::new();
                    let stack_ids: Vec<_> = stack_coverage.keys().copied().collect();
                    for c in containers {
                        changes
                            .entry(c.container_id.clone())
                            .or_insert_with(Vec::new)
                            .push(c);
                    }
                    let mut result = GroupSnapshot::default();
                    let mut references = references.into_iter().peekable();
                    while let Some(reference) = references.next() {
                        let mut patch = json!({"platformId":id,"containerId":reference,"containers":changes.remove(&reference).unwrap_or_default()});
                        // Deliver the authorized badge changes once, after the coalesced membership changes.
                        if references.peek().is_none() {
                            patch["stackIds"] = json!(stack_ids);
                            patch["stackBackupCoverage"] = json!(stack_coverage);
                        }
                        result
                            .events
                            .extend(event("ContainersChanged", patch)?.events);
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
                let mut containers = containers;
                let stack_coverage = match &self.backup_coverage {
                    Some(service) => {
                        service
                            .enrich_containers(p, id.unwrap(), &[], &mut containers)
                            .await
                    }
                    None => Default::default(),
                };
                for container in &mut containers {
                    container.capabilities = Some(
                        crate::api::resources::platforms::capabilities::container_capabilities(
                            capabilities,
                            container,
                            p.is_administrator(),
                        ),
                    );
                }
                crate::api::resources::platforms::container_views::enrich_container_owner_names(
                    &self.platforms,
                    p,
                    &mut containers,
                )
                .await
                .map_err(failure)?;
                event(
                    "ContainersInfoUpdated",
                    json!({"platformId":id, "containers":containers,"stackBackupCoverage":stack_coverage,"capabilities":capabilities}),
                )
            }
            Topic::Images(..) => {
                if sample {
                    return Ok(GroupSnapshot::default());
                }
                let platform = id.unwrap();
                let (capabilities, _) = match lease {
                    Some(lease) => {
                        lease
                            .daemon_capabilities(&self.identity, p, platform)
                            .await?
                    }
                    None => runtime::daemon_capabilities(self, p, platform).await?,
                };
                let inventory =
                    crate::realtime::shared_reads::images(&self.platforms, platform, e).await?;
                let response = crate::api::resources::platforms::image_views::image_list_view(
                    p,
                    platform,
                    inventory,
                    crate::api::resources::platforms::runtime_views::image_capabilities(
                        capabilities,
                    ),
                    self.image_store.as_ref(),
                    self.image_updates.as_ref(),
                    self.registries.as_ref(),
                )
                .await
                .map_err(failure)?;
                event(
                    "ImagesInfoUpdated",
                    json!({"platformId":platform,"images":response.images,"capabilities":response.capabilities}),
                )
            }
            Topic::Volumes(platform) => volumes::snapshot(self, p, *platform, lease, e).await,
            Topic::DockerDaemon(..) => {
                if e.is_some_and(|event| {
                    matches!(event.resource_type, "Deployment" | "Stack" | "SwarmService")
                        && event.event_kind != "runtimeChanged"
                }) {
                    return runtime::network_reference_snapshot(
                        self,
                        p,
                        id.unwrap(),
                        lease,
                        e.unwrap(),
                    )
                    .await;
                }
                if sample {
                    return Ok(GroupSnapshot::default());
                }
                if let Some(references) = e.and_then(PublishedRuntimeEvent::container_ids) {
                    let capabilities = self.platform_capabilities(p, id.unwrap(), lease).await?;
                    let mut containers = self
                        .changed_containers(e.unwrap(), id.unwrap(), &references)
                        .await?;
                    for container in &mut containers {
                        container.capabilities = Some(
                            crate::api::resources::platforms::capabilities::container_capabilities(
                                capabilities,
                                container,
                                p.is_administrator(),
                            ),
                        );
                    }
                    crate::api::resources::platforms::container_views::enrich_container_owner_names(&self.platforms, p, &mut containers).await.map_err(failure)?;
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
                    && let Some(snapshot) = runtime::realtime_resource_snapshot(
                        self,
                        p,
                        id.unwrap(),
                        lease,
                        event,
                        g.topic(),
                    )
                    .await?
                {
                    return Ok(snapshot);
                }
                let mut result =
                    runtime::realtime_daemon_snapshot(self, p, id.unwrap(), lease).await?;
                let capabilities = self.platform_capabilities(p, id.unwrap(), lease).await?;
                let mut containers =
                    crate::realtime::shared_reads::containers(&self.platforms, id.unwrap(), e)
                        .await
                        .map_err(failure)?
                        .into_iter()
                        .map(crate::api::resources::platforms::views::ContainerView::from)
                        .collect::<Vec<_>>();
                for container in &mut containers {
                    container.capabilities = Some(
                        crate::api::resources::platforms::capabilities::container_capabilities(
                            capabilities,
                            container,
                            p.is_administrator(),
                        ),
                    );
                }
                crate::api::resources::platforms::container_views::enrich_container_owner_names(
                    &self.platforms,
                    p,
                    &mut containers,
                )
                .await
                .map_err(failure)?;
                result.rows.extend(
                    rows(
                        "ContainerEventReceived",
                        containers,
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
                                    .map(|image| {
                                        let mut view = crate::api::resources::platforms::views::ImageView::from(image);
                                        view.capabilities = Some(crate::api::resources::platforms::runtime_views::image_capabilities(capabilities));
                                        view
                                    })
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
                    .as_ref()
                    .as_ref()
                    .is_some_and(|platform| {
                        platform.platform_type == citadel_platforms::PlatformKind::DockerSwarm
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
            Topic::Deployments => {
                let selection = WorkloadSelection::new(e, "deploymentIds");
                let style = selection.style();
                let resources = self
                    .deployments
                    .list_authorized(
                        actor,
                        admin,
                        &citadel_deployments::DeploymentFilter {
                            platform_id: g.platform_filter.or(selection.platform),
                            tags: g.tags.clone(),
                            resource_ids: selection.owners,
                            container_ids: selection.references,
                            limit: Some(catalog_limit),
                            ..Default::default()
                        },
                    )
                    .await
                    .map_err(failure)?;
                if resources.len() > self.snapshot_limit {
                    return Err(RealtimeReadError::SnapshotCapacity);
                }
                let views = resources
                    .into_iter()
                    .map(crate::api::resources::deployments::views::DeploymentView::try_from)
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(failure)?;
                let views = crate::api::resources::deployments::views::decorate_list(
                    p,
                    views,
                    self.backup_coverage.as_deref(),
                    self.copy_eligibility.as_deref(),
                    Some(hint_deadline),
                )
                .await;
                rows("DeploymentInfoUpdated", views, style)
            }
            Topic::Deployment(..) => {
                let resource = self
                    .deployments
                    .get_authorized(actor, admin, id.unwrap())
                    .await
                    .map_err(failure)?;
                let selection = WorkloadSelection::new(e, "deploymentIds");
                if !selection.includes(
                    resource.id,
                    Some(resource.platform_id),
                    &resource.probe_health,
                ) {
                    return Ok(GroupSnapshot::default());
                }
                let view =
                    crate::api::resources::deployments::views::DeploymentView::try_from(resource)
                        .map_err(failure)?;
                let view = crate::api::resources::deployments::views::decorate_detail(
                    p,
                    view,
                    self.backup_coverage.as_deref(),
                    self.copy_eligibility.as_deref(),
                    Some(hint_deadline),
                )
                .await;
                rows("DeploymentInfoUpdated", vec![view], selection.style())
            }
            Topic::Stacks => {
                let selection = WorkloadSelection::new(e, "stackIds");
                let style = selection.style();
                let resources = self
                    .stacks
                    .list_authorized(
                        actor,
                        admin,
                        &citadel_stacks::StackFilter {
                            catalog: true,
                            platform_id: g.platform_filter.or(selection.platform),
                            tags: g.tags.clone(),
                            resource_ids: selection.owners,
                            container_ids: selection.references,
                            limit: Some(catalog_limit),
                            ..Default::default()
                        },
                    )
                    .await
                    .map_err(failure)?;
                if resources.len() > self.snapshot_limit {
                    return Err(RealtimeReadError::SnapshotCapacity);
                }
                let views = resources
                    .into_iter()
                    .map(crate::api::resources::stacks::views::StackView::try_from)
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(failure)?;
                let views = crate::api::resources::stacks::views::decorate_list(
                    p,
                    views,
                    self.backup_coverage.as_deref(),
                    self.copy_eligibility.as_deref(),
                    Some(hint_deadline),
                )
                .await;
                rows("StackCatalogUpdated", views, style)
            }
            Topic::Stack(..) => {
                let resource = self
                    .stacks
                    .get_authorized(actor, admin, id.unwrap())
                    .await
                    .map_err(failure)?;
                let selection = WorkloadSelection::new(e, "stackIds");
                if !selection.includes(resource.id, resource.platform_id, &resource.probe_health) {
                    return Ok(GroupSnapshot::default());
                }
                let view = crate::api::resources::stacks::views::StackView::try_from(resource)
                    .map_err(failure)?;
                let view = crate::api::resources::stacks::views::decorate_detail(
                    p,
                    view,
                    self.backup_coverage.as_deref(),
                    self.copy_eligibility.as_deref(),
                    Some(hint_deadline),
                )
                .await;
                rows("StackInfoUpdated", vec![view], selection.style())
            }
            Topic::StackInfo(..) => {
                if sample {
                    let change = e.unwrap();
                    let platform = change.platform_id.ok_or(RealtimeReadError::Authorization)?;
                    let identities =
                        crate::realtime::shared_reads::identities(&self.platforms, platform, e)
                            .await?;
                    let values =
                        map_stats_for(change, identities.iter().filter(|c| c.stack_id == id));
                    return event(
                        "ReceiveStackContainerStats",
                        values
                            .into_iter()
                            .filter_map(|stat| {
                                let docker = stat["dockerContainerId"].as_str()?.to_owned();
                                Some(json!({"id":docker,"containerStat":stat}))
                            })
                            .collect::<Vec<_>>(),
                    );
                }
                let stack = self
                    .stacks
                    .get_authorized(actor, admin, id.unwrap())
                    .await
                    .map_err(failure)?;
                let platform = stack.platform_id.ok_or(RealtimeReadError::Authorization)?;
                if let Some(lease) = lease {
                    lease.set_scope(&g.name, platform);
                }
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
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(failure)?;
                event("ReceiveStackContainersInfo", containers)
            }
            Topic::SwarmService(..) => {
                let resource = self
                    .services
                    .get_authorized(actor, admin, id.unwrap())
                    .await
                    .map_err(failure)?;
                self.permission_with_lease(p, Platform, Some(resource.platform_id), None, lease)
                    .await?;
                let selection = WorkloadSelection::new(e, "serviceIds");
                if !selection.includes(
                    resource.id,
                    Some(resource.platform_id),
                    &resource.probe_health,
                ) {
                    return Ok(GroupSnapshot::default());
                }
                let view = crate::api::resources::swarm_services::views::ManagedSwarmServiceView::try_from(resource).map_err(failure)?;
                let view = crate::api::resources::swarm_services::views::decorate_detail(
                    p,
                    view,
                    self.backup_coverage.as_deref(),
                    self.copy_eligibility.as_deref(),
                    Some(hint_deadline),
                )
                .await;
                rows("SwarmServiceInfoUpdated", vec![view], selection.style())
            }
            Topic::SwarmServices(..) | Topic::ManagedSwarmServices => {
                let selection = WorkloadSelection::new(e, "serviceIds");
                let style = selection.style();
                let resources = self
                    .services
                    .list_authorized(
                        actor,
                        admin,
                        &citadel_swarm_services::SwarmServiceFilter {
                            platform_id: id.or(g.platform_filter).or(selection.platform),
                            tags: g.tags.clone(),
                            resource_ids: selection.owners,
                            container_ids: selection.references,
                            limit: Some(catalog_limit),
                            ..Default::default()
                        },
                    )
                    .await
                    .map_err(failure)?;
                if resources.len() > self.snapshot_limit {
                    return Err(RealtimeReadError::SnapshotCapacity);
                }
                let views = resources.into_iter()
                    .map(
                        crate::api::resources::swarm_services::views::ManagedSwarmServiceView::try_from,
                    )
                    .collect::<Result<Vec<_>, _>>().map_err(failure)?;
                let views = crate::api::resources::swarm_services::views::decorate_list(
                    p,
                    views,
                    self.backup_coverage.as_deref(),
                    self.copy_eligibility.as_deref(),
                    Some(hint_deadline),
                )
                .await;
                rows(
                    if matches!(g.topic(), Topic::ManagedSwarmServices) {
                        "ManagedSwarmServiceUpdated"
                    } else {
                        "SwarmServiceInfoUpdated"
                    },
                    views,
                    style,
                )
            }
            Topic::GitRepositories => {
                let owner = e.and_then(|event| event.changed_owner("GitRepository"));
                rows(
                    "GitRepositoryInfoUpdated",
                    crate::api::resources::git_repositories::views::authorized_repositories(
                        &self.identity,
                        p,
                        self.git_repositories
                            .list_git_repositories(actor, admin, &g.tags, owner)
                            .await
                            .map_err(failure)?,
                    )
                    .await
                    .map_err(failure)?,
                    owner.map_or(RowStyle::Update, RowStyle::OwnerPatch),
                )
            }
            Topic::GitRepo(..) => rows(
                "GitRepositoryInfoUpdated",
                crate::api::resources::git_repositories::views::authorized_repositories(
                    &self.identity,
                    p,
                    match self.git_repositories.get_git_repository(id.unwrap()).await {
                        Ok(value) => vec![value],
                        Err(citadel_git::GitRepositoryError::NotFound) if e.is_some() => vec![],
                        Err(error) => return Err(failure(error)),
                    },
                )
                .await
                .map_err(failure)?,
                RowStyle::Update,
            ),
            Topic::ScriptRuns(script_id) => {
                let runs = self
                    .scripts
                    .list_runs(*script_id, 50)
                    .await
                    .map_err(failure)?
                    .into_iter()
                    .map(crate::api::resources::scripts::views::ScriptRunView::try_from)
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(failure)?;
                // Send the window identities before the standard row events. This
                // removes stale rows on rejoin without putting 50 argument payloads
                // into one WebSocket frame.
                let ids: Vec<_> = runs.iter().map(|run| run.id).collect();
                let mut snapshot = rows("ScriptRunInfoUpdated", runs, RowStyle::Update)?;
                snapshot.events.push(ClientEvent::new(
                    "ScriptRunWindow",
                    vec![json!({"scriptId": script_id, "ids": ids})],
                ));
                Ok(snapshot)
            }
            Topic::Scripts => rows(
                "ScriptSummaryUpdated",
                crate::api::resources::scripts::authorized::script_summaries(
                    self.scripts.as_ref(),
                    p,
                    self.scripts
                        .list(actor, admin, &g.tags)
                        .await
                        .map_err(failure)?,
                )
                .await
                .map_err(failure)?,
                RowStyle::Update,
            ),
            Topic::Script(..) => rows(
                "ScriptInfoUpdated",
                crate::api::resources::scripts::authorized::authorized_scripts(
                    self.scripts.as_ref(),
                    p,
                    vec![self.scripts.get(id.unwrap()).await.map_err(failure)?],
                )
                .await
                .map_err(failure)?,
                RowStyle::Update,
            ),
            Topic::BackupRepositories => rows(
                "BackupRepositoryInfoUpdated",
                crate::api::resources::backups::views::authorized_repositories(
                    &self.identity,
                    p,
                    self.backups
                        .list_repositories(actor, admin)
                        .await
                        .map_err(failure)?,
                )
                .await
                .map_err(failure)?,
                RowStyle::Update,
            ),
            Topic::BackupRepository(..) => rows(
                "BackupRepositoryInfoUpdated",
                crate::api::resources::backups::views::authorized_repositories(
                    &self.identity,
                    p,
                    vec![
                        self.backups
                            .get_repository(id.unwrap())
                            .await
                            .map_err(failure)?,
                    ],
                )
                .await
                .map_err(failure)?,
                RowStyle::Update,
            ),
            Topic::BackupPolicies => {
                let owner = e.and_then(|event| event.changed_owner("BackupPolicy"));
                rows(
                    "BackupPolicyInfoUpdated",
                    crate::api::resources::backups::views::authorized_policies(
                        &self.identity,
                        p,
                        self.backups
                            .list_policies(actor, admin, owner, &g.tags)
                            .await
                            .map_err(failure)?,
                    )
                    .await
                    .map_err(failure)?,
                    owner.map_or(RowStyle::Update, RowStyle::OwnerPatch),
                )
            }
            Topic::BackupPolicy(..) => rows(
                "BackupPolicyInfoUpdated",
                crate::api::resources::backups::views::authorized_policies(
                    &self.identity,
                    p,
                    match self.backups.get_policy(id.unwrap()).await {
                        Ok(value) => vec![value],
                        Err(citadel_backups::BackupError::NotFound) if e.is_some() => vec![],
                        Err(error) => return Err(failure(error)),
                    },
                )
                .await
                .map_err(failure)?,
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
                    .collect::<Result<_, _>>()
                    .map_err(failure)?,
                RowStyle::Update,
            ),
            Topic::BackupRun(..) => rows(
                "BackupRunInfoUpdated",
                vec![
                    crate::api::resources::backups::views::BackupRunView::try_from(
                        self.backups.get_run(id.unwrap()).await.map_err(failure)?,
                    )
                    .map_err(failure)?,
                ],
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
                    .collect::<Result<_, _>>()
                    .map_err(failure)?,
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
                    )
                    .map_err(failure)?,
                ],
                RowStyle::Update,
            ),
            Topic::BuildProjects
                if g.tags.is_empty()
                    && let Some(project) =
                        e.and_then(PublishedRuntimeEvent::changed_build_project) =>
            {
                self.build_project_patch(p, project, lease).await
            }
            Topic::BuildProjects => rows(
                "BuildProjectInfoUpdated",
                crate::api::resources::builds::authorized::authorized_projects(
                    self.builds.as_ref(),
                    p,
                    self.builds
                        .list(actor, admin, &g.tags)
                        .await
                        .map_err(failure)?,
                )
                .await
                .map_err(failure)?,
                RowStyle::Update,
            ),
            Topic::BuildProject(..) => rows(
                "BuildProjectInfoUpdated",
                crate::api::resources::builds::authorized::authorized_projects(
                    self.builds.as_ref(),
                    p,
                    match self.builds.get(id.unwrap()).await {
                        Ok(value) => vec![value],
                        Err(citadel_builds::BuildError::NotFound) if e.is_some() => vec![],
                        Err(error) => return Err(failure(error)),
                    },
                )
                .await
                .map_err(failure)?,
                RowStyle::Update,
            ),
            Topic::BuildAgentPools => rows(
                "BuildAgentPoolInfoUpdated",
                crate::api::resources::builds::authorized::authorized_pools(
                    self.builds.as_ref(),
                    p,
                    self.builds
                        .list_pools(actor, admin, &g.tags)
                        .await
                        .map_err(failure)?,
                )
                .await
                .map_err(failure)?,
                RowStyle::Update,
            ),
            Topic::BuildAgentPool(..) => rows(
                "BuildAgentPoolInfoUpdated",
                crate::api::resources::builds::authorized::authorized_pools(
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
            self.container_access(
                p,
                identity.platform_id,
                identity.deployment_id,
                identity.stack_id,
                lease,
            )
            .await?;
            if let Some(patch) = change.payload["containerPatches"]
                .as_array()
                .and_then(|patches| patches.iter().find(|patch| patch["id"] == json!(id)))
            {
                let mut value = patch.clone();
                value["id"] = json!(identity.container_id);
                value["resourceId"] = json!(id);
                return event("ReceiveContainerInfoPatch", value);
            }
            return match map_stat(change, identity) {
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
        self.container_access(
            p,
            container.platform_id,
            container.deployment_id,
            container.stack_id,
            lease,
        )
        .await?;
        if let Some(lease) = lease {
            lease.set_scope(&g.name, container.platform_id);
        }
        if e.and_then(|e| e.platform_id)
            .is_some_and(|id| id != container.platform_id)
        {
            return Ok(GroupSnapshot::default());
        }
        let mut data = container_data(container.clone(), e).map_err(failure)?;
        if let Some(service) = &self.backup_coverage {
            data.backup_coverage = Some(
                service
                    .container(
                        p,
                        container.platform_id,
                        container.docker_node_id.as_deref(),
                        &container.container_id,
                    )
                    .await,
            );
        }
        event("ReceiveContainerInfo", data)
    }

    async fn activities(
        &self,
        p: &ActorPrincipal,
        g: &Group,
        lease: Option<&crate::realtime::AuthorizationLease>,
    ) -> Result<GroupSnapshot, RealtimeReadError> {
        let kind = self.activity_kind(p, g, lease).await?;
        let records = self
            .activities
            .list(
                &crate::api::resources::activities::authorized::activity_access(p),
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
    fn recheck<'a>(
        &'a self,
        p: &'a ActorPrincipal,
        g: &'a Group,
        lease: &'a crate::realtime::AuthorizationLease,
    ) -> BoxFuture<'a, Result<(), RealtimeReadError>> {
        Box::pin(self.recheck_group(p, g, lease))
    }

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
        Box::pin(async move {
            if matches!(g.topic(), Topic::InstanceSchedules) {
                return super::schedules::stream(self.jobs_service(p)?.as_ref(), p, cancel)
                    .map(Some);
            }
            if matches!(g.topic(), Topic::InstanceJobs) {
                return self.jobs_stream(p, cancel).map(Some);
            }
            self.logs(p, g, cancel).await
        })
    }
    fn invocation_group<'a>(
        &'a self,
        p: &'a ActorPrincipal,
        target: &'a str,
        args: &'a [Value],
    ) -> BoxFuture<'a, Result<Option<Group>, RealtimeReadError>> {
        Box::pin(async move {
            if matches!(target, "StartInstanceJobs" | "StartInstanceSchedules") {
                if !args.is_empty() {
                    return Err(RealtimeReadError::Authorization);
                }
                self.jobs_service(p)?;
                return Ok(Group::parse(if target == "StartInstanceSchedules" {
                    "instance-schedules"
                } else {
                    "instance-jobs"
                }));
            }
            self.log_invocation(p, target, args).await
        })
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

pub(super) fn container_data(
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

pub(super) fn map_stats(
    event: &PublishedRuntimeEvent,
    containers: &[citadel_platforms::ContainerIdentity],
) -> Vec<Value> {
    map_stats_for(event, containers.iter())
}
pub(super) fn map_stat(
    event: &PublishedRuntimeEvent,
    container: &citadel_platforms::ContainerIdentity,
) -> Option<Value> {
    if event.payload["dockerNodeId"].as_str() != container.docker_node_id.as_deref() {
        return None;
    }
    let stat = event.payload["stats"]
        .as_array()?
        .iter()
        .find(|s| s["dockerContainerId"].as_str() == Some(container.container_id.as_str()))?;
    let mut mapped = stat.clone();
    mapped["containerId"] = json!(container.id);
    Some(mapped)
}
pub(super) fn map_stats_for<'a>(
    event: &PublishedRuntimeEvent,
    containers: impl Iterator<Item = &'a citadel_platforms::ContainerIdentity>,
) -> Vec<Value> {
    let Some(stats) = event.payload["stats"].as_array() else {
        return vec![];
    };
    let identities: std::collections::HashMap<_, _> = containers
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
