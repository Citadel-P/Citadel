//! One-time composition graph, consumed into narrow HTTP states and owned jobs.
mod alerts;
mod automation;
mod backups;
mod builds;
mod catalog;
mod connectors;
mod identity;
mod licensing;
mod platforms;
mod workloads;
use citadel_activities::ActivityService;
use citadel_adapters::{
    connectors::{agent::client::AgentClient, docker::DockerClient},
    external::alerts::shoutrrr::ShoutrrrAlertDelivery,
    persistence::postgres::{
        activities::store::PostgresActivityStore, alerts::PostgresAlertRepository,
        connection as postgres_runtime, deployments::PostgresDeploymentRepository,
        git::repositories::PostgresGitRepositoryPersistence,
        licensing::store::PostgresLicenseEntitlementService, stacks::PostgresStackRepository,
        swarm_services::PostgresSwarmServiceRepository,
    },
};
use citadel_automation::AutomationService;
use citadel_backups::BackupService;
use citadel_bindings::SecretService;
use citadel_builds::BuildService;
use citadel_deployments::DeploymentService;
use citadel_git::{GitAccountService, GitRepositoryExecutionService};
use citadel_identity::{
    IdentityService, MfaService, OidcService, ProfileService, RoleMutationService, RoleReadService,
    ServiceAccountService, TeamMutationService, TeamReadService, UserReadService,
};
use citadel_licensing::LicenseService;
use citadel_server::{
    Readiness,
    api::routes::platforms as platforms_http,
    config::Config,
    license_realtime,
    metrics::Metrics,
    realtime::{IdentityRealtimeReader, RealtimeHub, RealtimeService},
};
use citadel_stacks::StackService;
use citadel_swarm_services::SwarmServiceService;
use sqlx::PgPool;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;
pub(super) struct RuntimeContext {
    pub pool: PgPool,
    pub docker: DockerClient,
    pub agent: Option<AgentClient>,
    pub edge_registry: citadel_adapters::connectors::edge::EdgeRegistry,
    pub cancellation: CancellationToken,
    pub dynamic_tasks: citadel_runtime::DynamicTasks,
    pub realtime_hub: Option<RealtimeHub>,
    pub container_runtime:
        citadel_adapters::connectors::routing::containers::ContainerRuntimeRouter,
}

pub struct ServerComponents {
    pub docker: DockerClient,
    pub pool: PgPool,
    pub cancellation: CancellationToken,
    pub dynamic_tasks: citadel_runtime::DynamicTasks,
    pub readiness: Arc<Readiness>,
    pub metrics: Arc<Metrics>,
    pub realtime_hub: Option<RealtimeHub>,
    pub identity: Arc<IdentityService>,
    pub licenses: Arc<LicenseService>,
    pub entitlements: Arc<PostgresLicenseEntitlementService>,
    pub mfa: Arc<MfaService>,
    pub oidc: Arc<OidcService>,
    pub service_accounts: Arc<ServiceAccountService>,
    pub profiles: Arc<ProfileService>,
    pub activities: Arc<ActivityService>,
    pub secrets: Arc<SecretService>,
    pub tags: Arc<dyn citadel_tags::TagRepository>,
    pub registries: Arc<dyn citadel_registries::RegistryRepository>,
    pub registry_connections: Arc<dyn citadel_registries::RegistryConnectionChecker>,
    pub git_accounts: Arc<GitAccountService>,
    pub git_execution: Arc<GitRepositoryExecutionService>,
    pub agent_setup: Arc<citadel_server::api::resources::platforms::views::AgentSetupView>,
    pub agent_setup_context: platforms_http::AgentSetupContext,
    pub edge_context: citadel_server::api::resources::platforms::edge::EdgeHttpContext,
    pub edge_registry: citadel_adapters::connectors::edge::EdgeRegistry,
    pub alert_store: Arc<PostgresAlertRepository>,
    pub alert_delivery: Arc<ShoutrrrAlertDelivery>,
    pub automation: Arc<AutomationService>,
    pub backups: Arc<BackupService>,
    pub users: Arc<UserReadService>,
    pub user_mutations: Arc<citadel_identity::UserMutationService>,
    pub teams: Arc<TeamReadService>,
    pub team_mutations: Arc<TeamMutationService>,
    pub roles: Arc<RoleReadService>,
    pub role_mutations: Arc<RoleMutationService>,
    pub builds: Arc<BuildService>,
    pub deployments: Arc<DeploymentService>,
    pub swarm_services: Arc<SwarmServiceService>,
    pub stacks: Arc<StackService>,
    pub platform_state: platforms_http::PlatformsHttpState,
    pub realtime: Option<RealtimeService>,
    pub license_realtime_service: Option<license_realtime::LicenseRealtimeService>,
    pub search:
        Arc<citadel_adapters::persistence::postgres::discovery::search::PostgresGlobalSearchStore>,
    pub actors:
        Arc<citadel_adapters::persistence::postgres::identity::actors::repository::PostgresActorRepository>,
    pub lookup:
        Arc<citadel_adapters::persistence::postgres::discovery::lookup::PostgresLookupStore>,
    pub audit: Arc<PostgresActivityStore>,
}

impl ServerComponents {
    /// Build shared services and the single-owner inputs for background jobs.
    /// The database schema must have been migrated first.
    pub async fn build(
        config: &Config,
    ) -> Result<(Self, crate::jobs::Jobs), Box<dyn std::error::Error>> {
        let pool = connect_database(config).await?;
        let docker = DockerClient::with_host_root(
            &config.docker_socket,
            config.docker_request_timeout,
            &config.execution.paths.host_root,
        )?;
        let cancellation = CancellationToken::new();
        let readiness = Arc::new(Readiness::default());
        let dynamic_tasks = citadel_runtime::DynamicTasks::new(cancellation.clone());
        let metrics = Arc::new(Metrics::default().with_dynamic_tasks(dynamic_tasks.clone()));
        let realtime_hub = config
            .realtime
            .as_ref()
            .map(|settings| RealtimeHub::new(settings.queue_capacity, Arc::clone(&metrics)));
        let licensing::LicensingComponents {
            licenses,
            entitlements,
            license_transition_monitor,
            license_realtime_hub,
        } = licensing::build(&pool, &realtime_hub);
        let identity::IdentityComponents {
            identity,
            mfa,
            oidc,
            service_accounts,
            profiles,
            users,
            user_mutations,
            teams,
            team_mutations,
            roles,
            role_mutations,
            secret_protector,
            last_used_worker,
        } = identity::build(config, &pool, &entitlements)?;
        let catalog::CatalogComponents {
            activities,
            tags,
            registries,
            registry_connections,
            secrets,
            git_accounts,
            git_execution,
            search,
            actors,
            lookup,
            audit,
        } = catalog::build(
            config,
            &pool,
            &secret_protector,
            &realtime_hub,
            &dynamic_tasks,
        )?;
        let connectors::ConnectorComponents {
            agent,
            agent_setup,
            agent_setup_context,
            edge_context,
            edge_registry,
            runtime_targets,
            container_runtime,
        } = connectors::build(config, &pool, &docker)?;
        let runtime = RuntimeContext {
            pool: pool.clone(),
            docker: docker.clone(),
            agent: agent.clone(),
            edge_registry: edge_registry.clone(),
            cancellation: cancellation.clone(),
            dynamic_tasks: dynamic_tasks.clone(),
            realtime_hub: realtime_hub.clone(),
            container_runtime,
        };
        let alerts::AlertComponents {
            alert_store,
            alert_delivery,
            alert_deliveries,
        } = alerts::build(config, &runtime, &entitlements);
        let automation =
            automation::build(config, &runtime, &identity, &entitlements, &alert_store)?;
        let backups = backups::build(
            config,
            &runtime,
            &secret_protector,
            &identity,
            &git_execution,
            &entitlements,
        )?;
        let builds = builds::build(
            config,
            &runtime,
            &secret_protector,
            &git_execution,
            &entitlements,
            &alert_store,
        )?;
        let workloads::WorkloadComponents {
            deployments,
            swarm_services,
            stacks,
            image_scanner,
        } = workloads::build(
            config,
            &runtime,
            &secret_protector,
            &git_execution,
            &entitlements,
            &alert_store,
        )?;
        let platforms::PlatformComponents {
            platform_state,
            platform_reads,
            volume_content,
        } = platforms::build(config, &runtime, &identity, &registries);
        let realtime = config.realtime.as_ref().map(|realtime_config| {
            RealtimeService::with_hub(
                realtime_config,
                Arc::new(IdentityRealtimeReader::new(
                    Arc::clone(&identity),
                    Arc::clone(&platform_reads),
                )),
                Arc::clone(&metrics),
                cancellation.clone(),
                realtime_hub
                    .clone()
                    .expect("configured realtime has a bounded hub"),
            )
            .with_groups(Arc::new(
                citadel_server::realtime_groups::ApplicationGroupReader {
                    git_repositories: Arc::new(PostgresGitRepositoryPersistence::new(pool.clone())),
                    identity: Arc::clone(&identity),
                    platforms: Arc::clone(&platform_reads),
                    deployments: Arc::new(PostgresDeploymentRepository::new(pool.clone())),
                    stacks: Arc::new(PostgresStackRepository::new(pool.clone())),
                    services: Arc::new(PostgresSwarmServiceRepository::new(pool.clone())),
                    automation: Arc::clone(automation.store()),
                    builds: Arc::clone(builds.store()),
                    backups: Arc::clone(backups.store()),
                    activities: Arc::clone(&activities),
                    alerts: alert_store.clone(),
                    runtime: platform_state.runtime.clone(),
                    statistics: platform_state.statistics.clone(),
                    realtime: platform_state.realtime.clone(),
                },
            ))
        });
        let license_realtime_service = realtime.is_none().then(|| {
            license_realtime::LicenseRealtimeService::new(
                Arc::clone(&identity),
                license_realtime_hub.clone(),
                cancellation.clone(),
            )
        });

        let jobs = crate::jobs::Jobs {
            docker: docker.clone(),
            agent,
            volume_content,
            runtime_targets,
            image_scanner,
            alert_deliveries,
            last_used_worker,
            license_transition_monitor,
            license_realtime_hub,
        };
        Ok((
            Self {
                docker,
                pool,
                cancellation,
                dynamic_tasks,
                readiness,
                metrics,
                realtime_hub,
                identity,
                licenses,
                entitlements,
                mfa,
                oidc,
                service_accounts,
                profiles,
                activities,
                secrets,
                tags,
                registries,
                registry_connections,
                git_accounts,
                git_execution,
                agent_setup,
                agent_setup_context,
                edge_context,
                edge_registry,
                alert_store,
                alert_delivery,
                automation,
                backups,
                users,
                user_mutations,
                teams,
                team_mutations,
                roles,
                role_mutations,
                builds,
                deployments,
                swarm_services,
                stacks,
                platform_state,
                realtime,
                license_realtime_service,
                search,
                actors,
                lookup,
                audit,
            },
            jobs,
        ))
    }
}

pub async fn connect_database(config: &Config) -> Result<PgPool, sqlx::Error> {
    postgres_runtime::connect_pool(
        &config.database_url,
        config.database_max_connections,
        config.docker_request_timeout,
    )
    .await
}
