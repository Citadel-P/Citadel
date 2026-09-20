//! Explicit dependency wiring shared by HTTP handlers and supervised workers.
use citadel_server::api::stacks as stacks_http;

use citadel_server::api::swarm_services as swarm_services_http;

use std::sync::Arc;

use std::time::Duration;

use citadel_adapters::activity_store::PostgresActivityStore;

use citadel_adapters::agent::{AgentClient, AgentRequestSigner};

use citadel_adapters::automation_token::IdentityAutomationRunTokenIssuer;

use citadel_adapters::backup_authorization::IdentityBackupRunAuthorizer;

use citadel_adapters::backup_executor::{DockerResticBackupExecutor, PostgresBackupSecretResolver};

use citadel_adapters::backup_source_planner::PostgresBackupSourcePlanner;

use citadel_adapters::build_executor::{
    AgentDockerBuildExecutor, LocalDockerBuildExecutor, PlatformBuildExecutor,
    PostgresBuildRegistryCredentialResolver, PostgresBuildSecretResolver,
};

use citadel_adapters::citadel_system_backup::PostgresCitadelSystemBackupBuilder;

use citadel_adapters::crypto::{
    AesGcmSecretProtector, Argon2PasswordHasher, JwtSessionTokenCodec,
    OpaqueServiceAccountTokenCodec,
};

use citadel_adapters::deployment_runtime::DeploymentRuntimeRouter;

use citadel_adapters::docker::DockerClient;

use citadel_adapters::identity_store::PostgresIdentityStore;

use citadel_adapters::license::{
    Ed25519LicenseVerifier, PostgresLicenseEntitlementService, PostgresLicenseStore,
};

use citadel_adapters::mfa::{HmacRecoveryCodeService, PostgresMfaStore, Sha1TotpService};

use citadel_adapters::oidc_protocol::OidcHttpProtocol;

use citadel_adapters::oidc_store::PostgresOidcStore;

use citadel_adapters::postgres::platforms::PostgresPlatformReader;

use citadel_adapters::platform_registration::{
    PlatformRegistrationRuntimeRouter, PostgresPlatformRegistrationRepository,
};

use citadel_adapters::postgres::automation::PostgresAutomationRepository;

use citadel_adapters::postgres::backups::PostgresBackupPersistence;

use citadel_adapters::postgres::builds::PostgresBuildRepository;

use citadel_adapters::postgres::deployments::PostgresDeploymentRepository;

use citadel_adapters::postgres::deployments::bindings::PostgresDeploymentBindingResolver;

use citadel_adapters::postgres::git::accounts::PostgresGitAccountRepository;

use citadel_adapters::postgres::git::repositories::PostgresGitRepositoryExecutionPersistence;

use citadel_adapters::postgres::stacks::PostgresStackRepository;

use citadel_adapters::postgres::stacks::bindings::PostgresStackBindingResolver;

use citadel_adapters::postgres::swarm_services::PostgresSwarmServiceRepository;

use citadel_adapters::postgres::swarm_services::bindings::PostgresSwarmServiceBindingResolver;

use citadel_adapters::postgres_runtime;

use citadel_adapters::profile_store::PostgresProfileStore;

use citadel_adapters::postgres::bindings::PostgresBindingRepository;

use citadel_adapters::role_store::PostgresRoleStore;

use citadel_adapters::service_account_store::PostgresServiceAccountStore;

use citadel_adapters::stack_build_images::PostgresStackBuildImageResolver;

use citadel_adapters::stack_runtime::StackRuntimeRouter;

use citadel_adapters::stack_source_materializer::GitStackSourceMaterializer;

use citadel_adapters::swarm_service_runtime::SwarmServiceRuntimeRouter;

use citadel_adapters::team_store::PostgresTeamStore;

use citadel_adapters::user_store::PostgresUserReadStore;

use citadel_adapters::{
    alert_delivery::ShoutrrrAlertDelivery, postgres::alerts::PostgresAlertRepository,
};

use citadel_alerts::AlertDeliveryService;

use citadel_application::{ActivityService, LicenseService, LicenseTransitionMonitor};
use citadel_runtime::service_account_last_used_channel;

use citadel_automation::{AutomationRuntimeConfig, AutomationService};

use citadel_backups::BackupService;

use citadel_builds::BuildService;

use citadel_deployments::DeploymentService;

use citadel_git::{GitAccountService, GitCli, GitRepositoryExecutionService};

use citadel_identity::{
    IdentityService, MfaConfiguration, MfaService, OidcService, ProfileService,
    RoleMutationService, RoleReadService, ServiceAccountService, SystemClock, TeamMutationService,
    TeamReadService, UserReadService,
};

use citadel_platforms::{PlatformReadService, PlatformRegistrationService};

use citadel_bindings::SecretService;

use citadel_server::config::Config;

use citadel_server::metrics::Metrics;

use citadel_server::realtime::{IdentityRealtimeReader, RealtimeHub, RealtimeService};

use citadel_server::{Readiness, application_info_http, license_realtime, platforms_http};

use citadel_stacks::StackService;

use citadel_swarm_services::SwarmServiceService;

use sqlx::PgPool;

use tokio_util::sync::CancellationToken;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub docker: DockerClient,
    pub cancellation: CancellationToken,
    pub dynamic_tasks: citadel_runtime::DynamicTasks,
    pub runtime_targets: Arc<citadel_server::runtime_targets::PlatformRuntimeRegistry>,
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
    pub git_accounts: Arc<GitAccountService>,
    pub git_execution: Arc<GitRepositoryExecutionService>,
    pub agent: Option<AgentClient>,
    pub agent_setup: Arc<citadel_server::platforms_http::dto::AgentSetupView>,
    pub agent_setup_context: platforms_http::AgentSetupContext,
    pub edge_context: platforms_http::EdgeHttpContext,
    pub edge_registry: citadel_adapters::edge::EdgeRegistry,
    pub alert_store: Arc<PostgresAlertRepository>,
    pub alert_delivery: Arc<ShoutrrrAlertDelivery>,
    pub alert_deliveries: Arc<AlertDeliveryService>,
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
    pub image_scanner: Arc<citadel_adapters::image_scanner::ImageScanner>,
    pub swarm_services: Arc<SwarmServiceService>,
    pub stacks: Arc<StackService>,
    pub platform_state: platforms_http::PlatformsHttpState,
    pub realtime: Option<RealtimeService>,
    pub license_realtime_service: Option<license_realtime::LicenseRealtimeService>,
    pub search: Arc<citadel_adapters::global_search::PostgresGlobalSearchStore>,
    pub actors: Arc<citadel_adapters::actor_store::PostgresActorStore>,
    pub lookup: Arc<citadel_adapters::lookup_store::PostgresLookupStore>,
    pub audit: Arc<PostgresActivityStore>,
}

impl AppState {
    /// Build shared services and the single-owner inputs for background jobs.
    /// The database schema must have been migrated first.
    pub async fn build(
        config: &Config,
    ) -> Result<(Self, crate::jobs::Jobs), Box<dyn std::error::Error>> {
        let pool = connect_database(config).await?;
        let docker = DockerClient::new(&config.docker_socket, config.docker_request_timeout)?;
        let cancellation = CancellationToken::new();
        let readiness = Arc::new(Readiness::default());
        let dynamic_tasks = citadel_runtime::DynamicTasks::new(cancellation.clone());
        let metrics = Arc::new(Metrics::default().with_dynamic_tasks(dynamic_tasks.clone()));
        let realtime_hub = config
            .realtime
            .as_ref()
            .map(|settings| RealtimeHub::new(settings.queue_capacity, Arc::clone(&metrics)));
        let token_codec = Arc::new(JwtSessionTokenCodec::new(
            config.identity.jwt_key.expose(),
            config.identity.issuer.clone(),
            config.identity.audience.clone(),
        )?);
        let service_account_tokens = Arc::new(OpaqueServiceAccountTokenCodec);
        let license_store = Arc::new(PostgresLicenseStore::new(pool.clone()));
        let license_verifier = Arc::new(Ed25519LicenseVerifier::default());
        let entitlements = Arc::new(PostgresLicenseEntitlementService::with(
            license_store.clone(),
            license_verifier.clone(),
        ));
        let clock = Arc::new(SystemClock);
        let license_realtime_hub =
            license_realtime::LicenseRealtimeHub::default().with_realtime(realtime_hub.clone());
        let licenses = Arc::new(
            LicenseService::new(
                license_store.clone(),
                license_verifier.clone(),
                clock.clone(),
                application_info_http::application_info().version.to_owned(),
            )
            .with_notifier(Arc::new(license_realtime_hub.clone())),
        );
        let license_transition_monitor =
            LicenseTransitionMonitor::new(license_store, license_verifier, clock.clone());
        let password_hasher = Arc::new(Argon2PasswordHasher::default());
        let identity_store = Arc::new(PostgresIdentityStore::new(pool.clone()));
        let (last_used_tracker, last_used_worker) = service_account_last_used_channel(
            identity_store.clone(),
            config.identity.service_account_last_used_capacity,
            Duration::from_secs(30),
            Duration::from_secs(5 * 60),
        );
        let identity = Arc::new(IdentityService::new(
            identity_store.clone(),
            password_hasher.clone(),
            token_codec,
            service_account_tokens.clone(),
            entitlements.clone(),
            clock.clone(),
            Arc::new(last_used_tracker),
            chrono::Duration::from_std(config.identity.access_token_lifetime)?,
            chrono::Duration::from_std(config.identity.refresh_token_lifetime)?,
        ));
        let secret_protector = Arc::new(AesGcmSecretProtector::new(
            config.identity.secret_encryption_key.expose(),
        )?);
        let mfa = Arc::new(MfaService::new(
            Arc::new(PostgresMfaStore::new(pool.clone())),
            Arc::clone(&identity),
            Arc::new(Sha1TotpService),
            secret_protector.clone(),
            Arc::new(HmacRecoveryCodeService::new(
                config.identity.secret_encryption_key.expose(),
            )?),
            clock.clone(),
            MfaConfiguration {
                policy: config.identity.mfa.policy,
                challenge_lifetime: chrono::Duration::from_std(
                    config.identity.mfa.challenge_lifetime,
                )?,
                setup_lifetime: chrono::Duration::from_std(config.identity.mfa.setup_lifetime)?,
                maximum_failed_attempts: config.identity.mfa.maximum_failed_attempts,
                recovery_code_count: config.identity.mfa.recovery_code_count,
            },
        ));
        let oidc = Arc::new(OidcService::new(
            Arc::new(PostgresOidcStore::new(pool.clone())),
            Arc::new(OidcHttpProtocol::new(Duration::from_secs(15))?),
            secret_protector.clone(),
            Arc::clone(&identity),
            clock.clone(),
            chrono::Duration::minutes(10),
        ));
        let service_accounts = Arc::new(ServiceAccountService::new(
            Arc::new(PostgresServiceAccountStore::new(pool.clone())),
            service_account_tokens,
            entitlements.clone(),
            clock.clone(),
        ));
        let profiles = Arc::new(ProfileService::new(
            Arc::new(PostgresProfileStore::new(pool.clone())),
            Arc::clone(&identity),
            clock.clone(),
        ));
        let activities = Arc::new(ActivityService::new(Arc::new(PostgresActivityStore::new(
            pool.clone(),
        ))));
        let tags: Arc<dyn citadel_tags::TagRepository> =
            Arc::new(citadel_adapters::postgres::tags::PostgresTagRepository::new(pool.clone()));
        let registries: Arc<dyn citadel_registries::RegistryRepository> = Arc::new(
            citadel_adapters::postgres::registries::PostgresRegistryRepository::new(pool.clone()),
        );
        let binding_store = Arc::new(PostgresBindingRepository::new(pool.clone()));
        let secrets = Arc::new(
            SecretService::new(binding_store, secret_protector.clone())
                .with_secret_provider_tester(Arc::new(
                    citadel_adapters::secret_value_resolver::PostgresSecretValueResolver::new(
                        pool.clone(),
                        secret_protector.clone(),
                    )?,
                )),
        );
        let git_accounts = Arc::new(GitAccountService::new(
            Arc::new(PostgresGitAccountRepository::new(pool.clone())),
            secret_protector.clone(),
        ));
        let data_root = std::env::var_os("CITADEL_DATA_ROOT")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| std::path::PathBuf::from("/app/data"));
        let git_execution = Arc::new(
            GitRepositoryExecutionService::new(
                Arc::new(PostgresGitRepositoryExecutionPersistence::new(pool.clone())),
                Arc::clone(&git_accounts),
                Arc::new(GitCli::new(
                    std::sync::Arc::new(citadel_processes::SystemProcess),
                    Duration::from_secs(120),
                )),
                data_root.join("git-repositories"),
                Duration::from_secs(10 * 60),
            )
            .with_change_notifier(citadel_server::realtime::change_callback(
                realtime_hub.clone(),
                "GitRepository",
            )),
        );
        let agent_signer = if let Some(agent) = &config.agent {
            AgentRequestSigner::from_file(&agent.private_key_path)?
        } else {
            AgentRequestSigner::load_or_create(&data_root.join("agent/signing-key"))?
        };
        let agent_image = std::env::var("CITADEL_EDGE_AGENT_IMAGE")
            .unwrap_or_else(|_| "ghcr.io/citadel-p/citadel.agent:latest".into());
        let agent_setup = Arc::new(citadel_server::platforms_http::dto::AgentSetupView::new(
            agent_signer.public_key_base64(),
            agent_image.clone(),
            !citadel_server::config::agent_allows_insecure()?,
        ));
        let agent = if let Some(agent) = &config.agent {
            Some(AgentClient::lazy(
                &agent.address,
                agent_signer.clone(),
                agent.operation_timeout,
                agent.allow_insecure,
            )?)
        } else if citadel_server::config::agent_allows_insecure()? {
            // Persisted Platforms supply the actual endpoint. No job probes
            // this signing context's placeholder address. Direct TLS is not
            // supported yet, so keep Direct transport disabled when h2c is off.
            Some(AgentClient::lazy(
                "http://localhost",
                agent_signer.clone(),
                config.docker_request_timeout,
                true,
            )?)
        } else {
            None
        };
        let runtime_targets = citadel_server::runtime_targets::PlatformRuntimeRegistry::new(
            pool.clone(),
            agent.clone(),
        );
        let alert_store = Arc::new(
            PostgresAlertRepository::new(pool.clone())
                .with_entitlements(entitlements.clone())
                .with_change_notifier(citadel_server::realtime::change_callback(
                    realtime_hub.clone(),
                    "Alert",
                )),
        );
        let alert_delivery = Arc::new(ShoutrrrAlertDelivery::new(
            std::env::var_os("CITADEL_SHOUTRRR_PATH").unwrap_or_else(|| "shoutrrr".into()),
            Duration::from_secs(15),
        ));
        let alert_deliveries = Arc::new(AlertDeliveryService::new(
            alert_store.clone(),
            alert_delivery.clone(),
        ));
        let automation = Arc::new(
            AutomationService::new(
                std::sync::Arc::new(citadel_processes::SystemProcess),
                Arc::new(
                    citadel_server::api::automation::TrackedAutomationTasks::new(
                        dynamic_tasks.clone(),
                    ),
                ),
                cancellation.clone(),
                Arc::new(PostgresAutomationRepository::new(pool.clone())),
                Arc::new(IdentityAutomationRunTokenIssuer::new(Arc::clone(&identity))),
                AutomationRuntimeConfig {
                    deno_path: std::env::var_os("Automations__DenoPath")
                        .or_else(|| std::env::var_os("CITADEL_DENO_PATH"))
                        .unwrap_or_else(|| "deno".into()),
                    work_root: std::path::absolute(
                        std::env::var_os("Automations__WorkDir")
                            .map(std::path::PathBuf::from)
                            .unwrap_or_else(|| data_root.join("automations/runs")),
                    )?,
                    internal_base_url: std::env::var("Automations__InternalBaseUrl")
                        .or_else(|_| std::env::var("CITADEL_INTERNAL_BASE_URL"))
                        .unwrap_or_else(|_| "http://127.0.0.1:8000".to_owned()),
                    endpoint_catalog_json: citadel_server::automation_endpoint_catalog_json(),
                    maximum_log_bytes: std::env::var("Automations__MaxLogBytes")
                        .ok()
                        .map(|value| value.parse::<usize>())
                        .transpose()?
                        .unwrap_or(1024 * 1024)
                        .clamp(1024, 16 * 1024 * 1024),
                    stale_after: Duration::from_secs(10 * 60),
                },
            )
            .with_options(config.automation)?
            .with_sandbox(
                std::path::absolute(
                    std::env::var_os("Automations__DenoCacheDir")
                        .map(std::path::PathBuf::from)
                        .unwrap_or_else(|| data_root.join("automations/deno-cache")),
                )?,
                std::env::var("Automations__AllowNet").ok(),
            )
            .with_entitlements(entitlements.clone())
            .with_alerts(alert_store.clone())
            .with_change_notifier(citadel_server::realtime::change_callback(
                realtime_hub.clone(),
                "AutomationAction",
            )),
        );
        let edge_registry = citadel_adapters::edge::EdgeRegistry::default();
        let container_runtime = citadel_adapters::container_mutations::ContainerRuntimeRouter::new(
            pool.clone(),
            docker.clone(),
            agent.clone(),
            edge_registry.clone(),
        );
        let backups = Arc::new(
            BackupService::new(
                Arc::new(PostgresBackupPersistence::new(pool.clone())),
                Arc::new(
                    DockerResticBackupExecutor::new(
                        std::env::var_os("CITADEL_DOCKER_PATH").unwrap_or_else(|| "docker".into()),
                        std::env::var("CITADEL_RESTIC_IMAGE")
                            .unwrap_or_else(|_| "restic/restic:0.18.1".to_owned()),
                        Arc::new(PostgresBackupSecretResolver::new(
                            pool.clone(),
                            secret_protector.clone(),
                        )?),
                        4 * 1024 * 1024,
                        pool.clone(),
                    )
                    .with_agent(agent.clone())
                    .with_edge(edge_registry.clone()),
                ),
                Arc::new(
                    PostgresBackupSourcePlanner::new(pool.clone())
                        .with_runtime(container_runtime.clone())
                        .with_git_execution(Arc::clone(&git_execution))
                        .with_system_builder(Arc::new(PostgresCitadelSystemBackupBuilder::new(
                            pool.clone(),
                            config.database_url.clone(),
                            std::env::var_os("CITADEL_PG_DUMP_PATH")
                                .unwrap_or_else(|| "pg_dump".into()),
                            data_root.join("backups/citadel-system"),
                            1024 * 1024,
                        ))),
                ),
                chrono::Duration::minutes(10),
                Arc::new(IdentityBackupRunAuthorizer::new(Arc::clone(&identity))),
            )
            .with_entitlements(entitlements.clone())
            .with_change_notifier(citadel_server::realtime::change_callback(
                realtime_hub.clone(),
                "BackupPolicy",
            )),
        );
        let platform_reads = Arc::new(PlatformReadService::new(Arc::new(
            PostgresPlatformReader::new(pool.clone()),
        )));
        let user_store = Arc::new(PostgresUserReadStore::new(pool.clone()));
        let users = Arc::new(UserReadService::new(user_store.clone()));
        let user_mutations = Arc::new(citadel_identity::UserMutationService::new(
            user_store,
            password_hasher,
            entitlements.clone(),
            clock.clone(),
        ));
        let team_store = Arc::new(PostgresTeamStore::new(pool.clone()));
        let teams = Arc::new(TeamReadService::new(team_store.clone()));
        let team_mutations = Arc::new(TeamMutationService::new(
            team_store,
            entitlements.clone(),
            clock.clone(),
        ));
        let role_store = Arc::new(PostgresRoleStore::new(pool.clone()));
        let roles = Arc::new(RoleReadService::new(role_store.clone()));
        let role_mutations = Arc::new(RoleMutationService::new(
            role_store,
            entitlements.clone(),
            clock,
        ));
        let build_secrets = Arc::new(PostgresBuildSecretResolver::new(
            pool.clone(),
            secret_protector.clone(),
        )?);
        let build_registries = Arc::new(PostgresBuildRegistryCredentialResolver::new(pool.clone()));
        let local_builds = Arc::new(LocalDockerBuildExecutor::new(
            data_root.join("git-repositories"),
            std::env::var_os("CITADEL_DOCKER_PATH").unwrap_or_else(|| "docker".into()),
            build_secrets.clone(),
            build_registries.clone(),
            4 * 1024 * 1024,
            pool.clone(),
        ));
        let agent_builds = agent.clone().map(|client| {
            AgentDockerBuildExecutor::new(
                data_root.join("git-repositories"),
                client,
                build_secrets.clone(),
                build_registries.clone(),
                4 * 1024 * 1024,
            )
        });
        let builds = Arc::new(
            BuildService::new(
                Arc::new(citadel_server::api::builds::TrackedBuildTasks::new(
                    dynamic_tasks.clone(),
                )),
                cancellation.clone(),
                Arc::new(PostgresBuildRepository::new(pool.clone())),
                Arc::new(
                    PlatformBuildExecutor::new(pool.clone(), local_builds, agent_builds)
                        .with_git_source(git_execution.clone())
                        .with_edge(AgentDockerBuildExecutor::new_edge(
                            data_root.join("git-repositories"),
                            edge_registry.clone(),
                            build_secrets,
                            build_registries,
                            4 * 1024 * 1024,
                        )),
                ),
                chrono::Duration::minutes(10),
            )
            .with_alerts(alert_store.clone())
            .with_change_notifier(citadel_server::realtime::change_callback(
                realtime_hub.clone(),
                "Build",
            ))
            .with_entitlements(entitlements.clone())
            .with_pool_change_notifier(citadel_server::realtime::change_callback(
                realtime_hub.clone(),
                "BuildAgentPool",
            ))
            .with_pool_checker(Arc::new(
                citadel_adapters::build_pool_checker::AgentBuildPoolChecker {
                    agent: agent.clone(),
                    edge: edge_registry.clone(),
                },
            ))
            .with_log_notifier(citadel_server::realtime::build_log_callback(
                realtime_hub.clone(),
            )),
        );
        let platform_registrations = Arc::new(PlatformRegistrationService::new(
            Arc::new(PostgresPlatformRegistrationRepository::new(pool.clone())),
            Arc::new(PlatformRegistrationRuntimeRouter::new(
                docker.clone(),
                agent.clone(),
            )),
        ));
        let image_cache =
            Arc::new(citadel_adapters::image_digest_cache::ImageDigestCache::default());
        let swarm_runtime =
            SwarmServiceRuntimeRouter::new(pool.clone(), docker.clone(), agent.clone())
                .with_image_cache(image_cache.clone())
                .with_edge(edge_registry.clone());
        let stack_runtime = StackRuntimeRouter::new(pool.clone(), docker.clone(), agent.clone())
            .with_image_cache(image_cache.clone())
            .with_edge(edge_registry.clone());
        let image_scanner = Arc::new(citadel_adapters::image_scanner::ImageScanner::new(
            pool.clone(),
            Arc::new(
                DeploymentRuntimeRouter::new(pool.clone(), docker.clone(), agent.clone())
                    .with_edge(edge_registry.clone()),
            ),
            image_cache.clone(),
        ));
        let deployments = Arc::new(
            DeploymentService::new(
                Arc::new(
                    citadel_server::api::deployments::TrackedDeploymentTasks::new(
                        dynamic_tasks.clone(),
                    ),
                ),
                Arc::new(PostgresDeploymentRepository::new(pool.clone())),
                Arc::new(
                    DeploymentRuntimeRouter::new(pool.clone(), docker.clone(), agent.clone())
                        .with_image_cache(image_cache.clone())
                        .with_edge(edge_registry.clone()),
                ),
                entitlements.clone(),
                cancellation.clone(),
            )
            .with_notifier(Arc::new(
                citadel_server::api::deployments::DeploymentsRealtimeNotifier::new(
                    realtime_hub.clone(),
                ),
            ))
            .with_binding_resolver(Arc::new(PostgresDeploymentBindingResolver::new(
                pool.clone(),
                secret_protector.clone(),
            )?))
            .with_adoption(Arc::new(
                citadel_adapters::postgres::deployments::PostgresContainerAdoption::new(
                    pool.clone(),
                    container_runtime.clone(),
                    secret_protector.clone(),
                    config.identity.secret_encryption_key.expose(),
                ),
            ))
            .with_alerts(alert_store.clone()),
        );
        let swarm_services = Arc::new(
            SwarmServiceService::new(
                Arc::new(
                    citadel_server::api::swarm_services::TrackedSwarmServiceTasks::new(
                        dynamic_tasks.clone(),
                    ),
                ),
                Arc::new(PostgresSwarmServiceRepository::new(pool.clone())),
                Arc::new(swarm_runtime.clone()),
                cancellation.clone(),
            )
            .with_image_digests(Arc::new(swarm_runtime.clone()))
            .with_adoption(Arc::new(
                citadel_adapters::postgres::swarm_services::PostgresSwarmServiceAdoption::new(
                    pool.clone(),
                    swarm_runtime.clone(),
                    config.identity.secret_encryption_key.expose(),
                ),
            ))
            .with_entitlements(entitlements.clone())
            .with_notifier(Arc::new(
                swarm_services_http::SwarmServicesRealtimeNotifier::new(realtime_hub.clone()),
            ))
            .with_binding_resolver(Arc::new(PostgresSwarmServiceBindingResolver::new(
                pool.clone(),
                secret_protector.clone(),
            )?))
            .with_alerts(alert_store.clone()),
        );
        let stacks = Arc::new(
            StackService::new(
                Arc::new(citadel_server::api::stacks::TrackedStackTasks::new(
                    dynamic_tasks.clone(),
                )),
                Arc::new(PostgresStackRepository::new(pool.clone())),
                Arc::new(stack_runtime.clone()),
                Arc::new(PostgresStackBindingResolver::new(
                    pool.clone(),
                    secret_protector.clone(),
                )?),
                Arc::new(stacks_http::StacksRealtimeNotifier::new(
                    realtime_hub.clone(),
                )),
                cancellation.clone(),
            )
            .with_source_materializer(Arc::new(GitStackSourceMaterializer::new(Arc::clone(
                &git_execution,
            ))))
            .with_build_image_resolver(Arc::new(PostgresStackBuildImageResolver::new(pool.clone())))
            .with_update_scanner(Arc::new(
                citadel_adapters::stack_runtime::StackUpdateRuntime::new(
                    stack_runtime.clone(),
                    git_execution.clone(),
                ),
            ))
            .with_entitlements(entitlements.clone())
            .with_alerts(alert_store.clone()),
        );
        let container_hub = realtime_hub.clone();
        let container_mutations = Arc::new(container_runtime.clone().into_service().with_notifier(
            move |claim| {
                if let Some(hub) = &container_hub {
                    for target in &claim.targets {
                        hub.publish_runtime_change(
                            target.platform_id,
                            "container",
                            "update",
                            &target.docker_id,
                        );
                    }
                    for id in &claim.deployment_ids {
                        hub.publish_resource_change("Deployment", *id, "updated");
                    }
                    for id in &claim.stack_ids {
                        hub.publish_resource_change("Stack", *id, "updated");
                    }
                }
            },
        ));
        let platform_state = platforms_http::PlatformsHttpState {
            volume_content: Arc::new(citadel_adapters::volume_content::VolumeContentAdapter::new(
                pool.clone(),
                docker.clone(),
                agent.clone(),
                edge_registry.clone(),
                std::env::var("CITADEL_VOLUME_HELPER_IMAGE")
                    .unwrap_or_else(|_| "ghcr.io/citadel-p/citadel.agent:latest".into()),
            )),
            containers: container_mutations.clone(),
            identity: Arc::clone(&identity),
            platforms: Arc::clone(&platform_reads),
            registrations: platform_registrations,
            pool: pool.clone(),
            registries: Arc::clone(&registries),
            platform_metadata: Arc::new(
                citadel_adapters::postgres::platforms::PostgresPlatformMetadataRepository::new(
                    pool.clone(),
                ),
            ),
            docker: docker.clone(),
            agent: agent.clone(),
            edge: edge_registry.clone(),
            realtime: realtime_hub.clone(),
            stats_sample_max_age: config
                .probe_interval
                .saturating_mul(3)
                .max(std::time::Duration::from_secs(30)),
        };
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
    git_repositories: Arc::new(citadel_adapters::postgres::git::repositories::PostgresGitRepositoryPersistence::new(pool.clone())),
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
                    docker: platform_state.clone(),
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
        let agent_setup_context = platforms_http::AgentSetupContext {
            signer: agent_signer,
            image: agent_image.clone(),
            requires_tls: !citadel_server::config::agent_allows_insecure()?,
        };
        let edge_context = platforms_http::EdgeHttpContext {
            node_agent_ca_bundle: match std::env::var_os("CITADEL_NODE_AGENT_CA_CERTIFICATE_PATH") {
                None => None,
                Some(path) => {
                    use std::io::Read;
                    let mut data = Vec::new();
                    std::fs::File::open(path)?
                        .take(1024 * 1024 + 1)
                        .read_to_end(&mut data)?;
                    if data.is_empty() || data.len() > 1024 * 1024 {
                        return Err("Node-agent CA bundle must be between 1 byte and 1 MiB".into());
                    }
                    Some(data.into())
                }
            },
            store: citadel_adapters::edge::PostgresEdgeStore::new(pool.clone()),
            registry: edge_registry.clone(),
            core_url: config
                .transport
                .edge_agent_public_url
                .to_string()
                .trim_end_matches('/')
                .to_owned(),
            agent_image,
        };
        let search =
            Arc::new(citadel_adapters::global_search::PostgresGlobalSearchStore::new(pool.clone()));
        let actors = Arc::new(citadel_adapters::actor_store::PostgresActorStore::new(
            pool.clone(),
        ));
        let lookup = Arc::new(citadel_adapters::lookup_store::PostgresLookupStore::new(
            pool.clone(),
        ));
        let audit = Arc::new(PostgresActivityStore::new(pool.clone()));
        let jobs = crate::jobs::Jobs {
            last_used_worker,
            license_transition_monitor,
            license_realtime_hub,
        };
        Ok((
            Self {
                pool,
                docker,
                cancellation,
                dynamic_tasks,
                runtime_targets,
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
                git_accounts,
                git_execution,
                agent,
                agent_setup,
                agent_setup_context,
                edge_context,
                edge_registry,
                alert_store,
                alert_delivery,
                alert_deliveries,
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
                image_scanner,
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
