//! Revalidate group access without materializing list snapshots or enrichments.
use super::*;

impl ApplicationGroupReader {
    pub(super) async fn recheck_group(
        &self,
        p: &ActorPrincipal,
        g: &Group,
        lease: &crate::realtime::AuthorizationLease,
    ) -> Result<(), RealtimeReadError> {
        use Topic::*;
        let actor = p.actor_id;
        let admin = p.is_administrator();
        let id = g.id();
        let mut permission_id = id;
        let mut specific = None;
        let kind = match g.topic() {
            ContainerExec { .. } | SwarmTaskExec { .. } => {
                self.terminal_target(p, g).await?;
                return Ok(());
            }
            ContainerLog(..) | StackLog(..) => {
                self.log_targets(p, g).await?;
                return Ok(());
            }
            ContainerInfo(..) | ContainerHealth(..) | ContainerProcesses(..) => {
                let id = id
                    .or_else(|| g.reference().and_then(|value| Uuid::parse_str(value).ok()))
                    .ok_or(RealtimeReadError::Authorization)?;
                let container = self
                    .platforms
                    .get_container(id)
                    .await
                    .map_err(failure)?
                    .ok_or(RealtimeReadError::Authorization)?;
                if matches!(g.topic(), ContainerInfo(..)) {
                    self.container_access(
                        p,
                        container.platform_id,
                        container.deployment_id,
                        container.stack_id,
                        Some(lease),
                    )
                    .await?;
                } else {
                    self.permission_with_lease(
                        p,
                        ResourceType::Platform,
                        Some(container.platform_id),
                        matches!(g.topic(), ContainerProcesses(..))
                            .then_some(SpecificPermission::Inspect),
                        Some(lease),
                    )
                    .await?;
                }
                lease.set_scope(&g.name, container.platform_id);
                return Ok(());
            }
            Activity { .. } => {
                self.activity_kind(p, g, Some(lease)).await?;
                return Ok(());
            }
            AlertEvents => return Ok(()), // The snapshot's SQL filters by this authenticated actor.
            ResourceCopy(id) => {
                self.copy_eligibility
                    .as_ref()
                    .ok_or(RealtimeReadError::Authorization)?
                    .get(actor, *id)
                    .await
                    .map_err(failure)?;
                return Ok(());
            }
            Platforms => ResourceType::Platform,
            Containers(platform) | Images(platform) | DockerDaemon(platform) => {
                self.platforms
                    .platform_kind(*platform)
                    .await
                    .map_err(failure)?;
                ResourceType::Platform
            }
            Deployment(id) => {
                self.deployments
                    .get_authorized(actor, admin, *id)
                    .await
                    .map_err(failure)?;
                ResourceType::Deployment
            }
            Deployments => ResourceType::Deployment,
            Stack(id) | StackInfo(id) => {
                let stack = self
                    .stacks
                    .get_authorized(actor, admin, *id)
                    .await
                    .map_err(failure)?;
                if matches!(g.topic(), StackInfo(..)) {
                    lease.set_scope(
                        &g.name,
                        stack.platform_id.ok_or(RealtimeReadError::Authorization)?,
                    );
                }
                ResourceType::Stack
            }
            Stacks => ResourceType::Stack,
            SwarmServices(platform) => {
                self.permission_with_lease(
                    p,
                    ResourceType::Platform,
                    Some(*platform),
                    None,
                    Some(lease),
                )
                .await?;
                permission_id = None;
                ResourceType::SwarmService
            }
            SwarmService(id) => {
                let service = self
                    .services
                    .get_authorized(actor, admin, *id)
                    .await
                    .map_err(failure)?;
                self.permission_with_lease(
                    p,
                    ResourceType::Platform,
                    Some(service.platform_id),
                    None,
                    Some(lease),
                )
                .await?;
                ResourceType::SwarmService
            }
            GitRepositories => ResourceType::GitRepository,
            GitRepo(id) => {
                self.git_repositories
                    .get_git_repository(*id)
                    .await
                    .map_err(failure)?;
                ResourceType::GitRepository
            }
            Scripts => ResourceType::Script,
            Script(id) | ScriptRuns(id) => {
                self.scripts.get(*id).await.map_err(failure)?;
                ResourceType::Script
            }
            BackupRepositories => ResourceType::BackupRepository,
            BackupRepository(id) => {
                self.backups.get_repository(*id).await.map_err(failure)?;
                ResourceType::BackupRepository
            }
            BackupPolicies => ResourceType::BackupPolicy,
            BackupPolicy(id) | BackupRuns(id) | BackupRestoreRuns(id) => {
                self.backups.get_policy(*id).await.map_err(failure)?;
                specific = matches!(g.topic(), BackupRestoreRuns(..))
                    .then_some(SpecificPermission::Restore);
                ResourceType::BackupPolicy
            }
            BackupRun(id) => {
                permission_id = Some(
                    self.backups
                        .get_run(*id)
                        .await
                        .map_err(failure)?
                        .backup_policy_id,
                );
                ResourceType::BackupPolicy
            }
            BackupRestoreRun(id) => {
                let restore = self.backups.get_restore(*id).await.map_err(failure)?;
                permission_id = Some(
                    self.backups
                        .get_run(restore.backup_run_id)
                        .await
                        .map_err(failure)?
                        .backup_policy_id,
                );
                specific = Some(SpecificPermission::Restore);
                ResourceType::BackupPolicy
            }
            BuildProjects => ResourceType::Build,
            BuildProject(id) | BuildRuns(id) => {
                self.builds.get(*id).await.map_err(failure)?;
                ResourceType::Build
            }
            BuildRun(id) => {
                permission_id = Some(
                    self.builds
                        .get_run(*id)
                        .await
                        .map_err(failure)?
                        .build_project_id,
                );
                ResourceType::Build
            }
            BuildAgentPools => ResourceType::BuildAgentPool,
            BuildAgentPool(id) => {
                self.builds.get_pool(*id).await.map_err(failure)?;
                ResourceType::BuildAgentPool
            }
        };
        self.permission_with_lease(p, kind, permission_id, specific, Some(lease))
            .await
    }

    pub(super) async fn container_access(
        &self,
        p: &ActorPrincipal,
        platform: Uuid,
        deployment: Option<Uuid>,
        stack: Option<Uuid>,
        lease: Option<&crate::realtime::AuthorizationLease>,
    ) -> Result<(), RealtimeReadError> {
        for (kind, id) in [
            (ResourceType::Platform, Some(platform)),
            (ResourceType::Deployment, deployment),
            (ResourceType::Stack, stack),
        ] {
            if let Some(id) = id {
                match self
                    .permission_with_lease(p, kind, Some(id), None, lease)
                    .await
                {
                    Ok(()) => return Ok(()),
                    Err(RealtimeReadError::Authorization) => {}
                    Err(error) => return Err(error),
                }
            }
        }
        Err(RealtimeReadError::Authorization)
    }

    pub(super) async fn activity_kind(
        &self,
        p: &ActorPrincipal,
        g: &Group,
        lease: Option<&crate::realtime::AuthorizationLease>,
    ) -> Result<citadel_activities::ActivityResourceType, RealtimeReadError> {
        let kind: citadel_activities::ActivityResourceType =
            serde_json::from_value(json!(g.reference()))
                .map_err(|_| RealtimeReadError::Authorization)?;
        use citadel_activities::ActivityResourceType as A;
        if matches!(
            kind,
            A::User
                | A::Team
                | A::Role
                | A::ServiceAccount
                | A::OidcProvider
                | A::License
                | A::Updates
                | A::Instance
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
        Ok(kind)
    }
}
