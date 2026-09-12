#![forbid(unsafe_code)]

use std::sync::Arc;
use std::time::Duration;

use axum::Router;
use axum::http::header;
use axum::middleware;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use citadel_adapters::PostgresAuthorizedPlatformReader;
use citadel_adapters::activity_store::PostgresActivityStore;
use citadel_adapters::agent::{AgentClient, AgentRequestSigner};
use citadel_adapters::alert_store::{PostgresAlertStore, ShoutrrrAlertDelivery};
use citadel_adapters::automation_store::PostgresAutomationStore;
use citadel_adapters::automation_token::IdentityAutomationRunTokenIssuer;
use citadel_adapters::backup_authorization::IdentityBackupRunAuthorizer;
use citadel_adapters::backup_executor::{DockerResticBackupExecutor, PostgresBackupSecretResolver};
use citadel_adapters::backup_source_planner::PostgresBackupSourcePlanner;
use citadel_adapters::backup_store::PostgresBackupStore;
use citadel_adapters::build_executor::{
    AgentDockerBuildExecutor, LocalDockerBuildExecutor, PlatformBuildExecutor,
    PostgresBuildRegistryCredentialResolver, PostgresBuildSecretResolver,
};
use citadel_adapters::build_store::PostgresBuildStore;
use citadel_adapters::citadel_system_backup::{
    CitadelSystemRestoreOptions, PostgresCitadelSystemBackupBuilder, restore_citadel_system,
};
use citadel_adapters::crypto::{
    AesGcmSecretProtector, Argon2PasswordHasher, JwtSessionTokenCodec,
    OpaqueServiceAccountTokenCodec,
};
use citadel_adapters::deployment_bindings::PostgresDeploymentBindingResolver;
use citadel_adapters::deployment_runtime::DeploymentRuntimeRouter;
use citadel_adapters::deployment_store::PostgresDeploymentStore;
use citadel_adapters::docker::DockerClient;
use citadel_adapters::git_account_store::PostgresGitAccountStore;
use citadel_adapters::git_repository_execution_store::PostgresGitRepositoryExecutionStore;
use citadel_adapters::identity_store::PostgresIdentityStore;
use citadel_adapters::license::{
    Ed25519LicenseVerifier, PostgresLicenseEntitlementService, PostgresLicenseStore,
};
use citadel_adapters::mfa::{HmacRecoveryCodeService, PostgresMfaStore, Sha1TotpService};
use citadel_adapters::oidc_protocol::OidcHttpProtocol;
use citadel_adapters::oidc_store::PostgresOidcStore;
use citadel_adapters::platform_read_store::PostgresPlatformReadStore;
use citadel_adapters::platform_registration::{
    PlatformRegistrationRuntimeRouter, PostgresPlatformRegistrationStore,
};
use citadel_adapters::postgres_runtime;
use citadel_adapters::profile_store::PostgresProfileStore;
use citadel_adapters::resource_metadata_store::PostgresResourceMetadataStore;
use citadel_adapters::role_store::PostgresRoleStore;
use citadel_adapters::service_account_store::PostgresServiceAccountStore;
use citadel_adapters::stack_bindings::PostgresStackBindingResolver;
use citadel_adapters::stack_build_images::PostgresStackBuildImageResolver;
use citadel_adapters::stack_runtime::StackRuntimeRouter;
use citadel_adapters::stack_source_materializer::GitStackSourceMaterializer;
use citadel_adapters::stack_store::PostgresStackStore;
use citadel_adapters::swarm_service_bindings::PostgresSwarmServiceBindingResolver;
use citadel_adapters::swarm_service_runtime::SwarmServiceRuntimeRouter;
use citadel_adapters::swarm_service_store::PostgresSwarmServiceStore;
use citadel_adapters::team_store::PostgresTeamStore;
use citadel_adapters::user_store::PostgresUserReadStore;
use citadel_alerts::AlertDeliveryService;
use citadel_application::{
    ActivityService, LicenseService, LicenseTransitionMonitor, TaskSupervisor,
    service_account_last_used_channel,
};
use citadel_automation::{AutomationRuntimeConfig, AutomationService};
use citadel_backups::BackupService;
use citadel_builds::BuildService;
use citadel_database::MigrationRunner;
use citadel_deployments::DeploymentService;
use citadel_domain::ActorId;
use citadel_git::{GitAccountService, GitCli, GitRepositoryExecutionService};
use citadel_identity::{
    IdentityService, MfaConfiguration, MfaService, OidcService, ProfileService,
    RoleMutationService, RoleReadService, ServiceAccountService, SystemClock, TeamMutationService,
    TeamReadService, UserReadService,
};
use citadel_platforms::{
    AuthorizedPlatformReader, PlatformReadService, PlatformRegistrationService, PlatformRuntimePort,
};
use citadel_resources::ResourceMetadataService;
use citadel_server::config::{Config, DatabaseConfig};
use citadel_server::metrics::Metrics;
use citadel_server::realtime::{IdentityRealtimeReader, RealtimeHub, RealtimeService};
use citadel_server::{
    Readiness, activities_http, alerts_http, application_info_http, automation_http, backups_http,
    builds_http, deployments_http, git_accounts_http, git_repositories_http, identity_http,
    license_http, license_realtime, oidc_http, platforms_http, profile_http, resources_http,
    roles_http, service_accounts_http, stacks_http, swarm_services_http, teams_http, transport,
    users_http, webhooks_http, workers,
};
use citadel_stacks::StackService;
use citadel_swarm_services::ManagedSwarmServiceService;
use clap::{Parser, Subcommand};
use futures_util::StreamExt;
use sqlx::PgPool;
use tokio_util::sync::CancellationToken;
use tower_http::catch_panic::CatchPanicLayer;
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;
use uuid::Uuid;

#[derive(Parser)]
#[command(name = "citadel-server", about = "Citadel Rust migration server")]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Run the Rust foundation server and supervised workers.
    Serve,
    /// Apply the embedded Citadel schema migrations and exit.
    Migrate,
    /// Replace the configured database from a validated offline recovery bundle.
    RestoreSystem {
        #[arg(long)]
        bundle: std::path::PathBuf,
        #[arg(long)]
        confirm_instance_replacement: bool,
    },
    /// Print bounded process and cgroup diagnostics as JSON.
    Diagnostics,
    /// Print the effective non-secret configuration as JSON.
    PrintEffectiveConfig,
    /// Exercise the generated Docker subset and optional Actor-authorized read.
    Phase0Smoke {
        #[arg(long)]
        actor_id: Option<Uuid>,
    },
    /// Print the Base64 public key corresponding to a raw 32-byte Agent private key.
    AgentPublicKey {
        #[arg(long)]
        private_key_path: std::path::PathBuf,
    },
    /// Prove signed .NET Agent handshake, read, stream, cancellation, and Local equivalence.
    Phase0AgentSmoke,
    /// Probe an already-running Phase 0A server without curl in the image.
    Healthcheck {
        #[arg(long, default_value = "http://127.0.0.1:8000/health")]
        url: String,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_tracing();
    let cli = Cli::parse();
    match cli.command.unwrap_or(Command::Serve) {
        Command::Serve => serve(Config::from_env()?).await,
        Command::Migrate => migrate(DatabaseConfig::from_env()?).await,
        Command::RestoreSystem {
            bundle,
            confirm_instance_replacement,
        } => {
            if !confirm_instance_replacement {
                return Err(
                    "restore-system replaces the configured Citadel database; pass --confirm-instance-replacement after stopping every Citadel Core instance"
                        .into(),
                );
            }
            restore_system(DatabaseConfig::from_env()?, bundle).await
        }
        Command::Diagnostics => {
            println!(
                "{}",
                serde_json::to_string_pretty(&citadel_server::diagnostics::snapshot())?
            );
            Ok(())
        }
        Command::PrintEffectiveConfig => {
            println!(
                "{}",
                serde_json::to_string_pretty(&Config::from_env()?.effective()?)?
            );
            Ok(())
        }
        Command::Phase0Smoke { actor_id } => phase0_smoke(Config::from_env()?, actor_id).await,
        Command::AgentPublicKey { private_key_path } => {
            println!(
                "{}",
                AgentRequestSigner::from_file(&private_key_path)?.public_key_base64()
            );
            Ok(())
        }
        Command::Phase0AgentSmoke => phase0_agent_smoke(Config::from_env()?).await,
        Command::Healthcheck { url } => {
            reqwest::Client::builder()
                .timeout(Duration::from_secs(2))
                .build()?
                .get(url)
                .send()
                .await?
                .error_for_status()?;
            Ok(())
        }
    }
}

async fn restore_system(
    database: DatabaseConfig,
    bundle: std::path::PathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    let (_, encryption_key) = citadel_server::config::identity_keys_from_env()?;
    let cancellation = CancellationToken::new();
    let signal_token = cancellation.clone();
    let signal = tokio::spawn(async move {
        if tokio::signal::ctrl_c().await.is_ok() {
            signal_token.cancel();
        }
    });
    let result = restore_citadel_system(
        &CitadelSystemRestoreOptions {
            bundle,
            database_url: database.database_url,
            pg_restore: std::env::var_os("CITADEL_PG_RESTORE_PATH")
                .unwrap_or_else(|| "pg_restore".into()),
            maximum_output: 1024 * 1024,
            secret_encryption_key: Some(zeroize::Zeroizing::new(encryption_key.expose().to_vec())),
        },
        &cancellation,
    )
    .await;
    signal.abort();
    result?;
    println!("Citadel system recovery bundle restored successfully.");
    Ok(())
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("citadel_server=info,citadel_adapters=info"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .json()
        .with_current_span(false)
        .init();
}

async fn serve(config: Config) -> Result<(), Box<dyn std::error::Error>> {
    tracing::info!(
        effective_configuration = %serde_json::to_string(&config.effective()?)?,
        "starting Rust foundation server"
    );
    if config.transport.mode == citadel_server::config::TransportMode::Disabled {
        tracing::warn!(
            "Citadel transport security is disabled; API and Agent traffic is not encrypted"
        );
    }
    let migration = MigrationRunner::migrate(&config.database_url).await?;
    tracing::info!(
        applied = migration.applied,
        already_applied = migration.already_applied,
        "database migrations completed"
    );
    let pool = connect_database(&config).await?;
    let docker = DockerClient::new(&config.docker_socket, config.docker_request_timeout)?;
    let cancellation = CancellationToken::new();
    let readiness = Arc::new(Readiness::default());
    let metrics = Arc::new(Metrics::default());
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
    citadel_server::bootstrap::initialize_from_environment(&identity).await?;
    // Login must use persisted setup state before listeners open, not wait for
    // the first background probe after a fresh bootstrap or ordinary restart.
    readiness.set_setup(!identity.setup_status().await?.requires_setup);
    readiness.set(true, false);
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
            challenge_lifetime: chrono::Duration::from_std(config.identity.mfa.challenge_lifetime)?,
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
    let resource_metadata = Arc::new(PostgresResourceMetadataStore::new(pool.clone()));
    let resources = Arc::new(
        ResourceMetadataService::new(resource_metadata.clone(), secret_protector.clone())
            .with_secret_provider_tester(Arc::new(
                citadel_adapters::secret_value_resolver::PostgresSecretValueResolver::new(
                    pool.clone(),
                    secret_protector.clone(),
                )?,
            )),
    );
    let git_accounts = Arc::new(GitAccountService::new(
        Arc::new(PostgresGitAccountStore::new(pool.clone())),
        secret_protector.clone(),
    ));
    let data_root = std::env::var_os("CITADEL_DATA_ROOT")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from("/app/data"));
    let git_execution = Arc::new(
        GitRepositoryExecutionService::new(
            Arc::new(PostgresGitRepositoryExecutionStore::new(pool.clone())),
            Arc::clone(&git_accounts),
            Arc::new(GitCli::new(Duration::from_secs(120))),
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
    let agent_setup = Arc::new(citadel_platforms::agent_setup::AgentSetupView::new(
        agent_signer.public_key_base64(),
        agent_image.clone(),
        !citadel_server::config::agent_allows_insecure()?,
    ));
    let agent = if let Some(agent) = &config.agent {
        Some(
            AgentClient::connect(
                &agent.address,
                agent_signer.clone(),
                agent.operation_timeout,
                agent.allow_insecure,
            )
            .await?,
        )
    } else {
        None
    };
    let alert_store = Arc::new(
        PostgresAlertStore::new(pool.clone())
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
            Arc::new(PostgresAutomationStore::new(pool.clone())),
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
    let backups = Arc::new(
        BackupService::new(
            Arc::new(PostgresBackupStore::new(pool.clone())),
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
                    .with_runtime(
                        citadel_adapters::container_mutations::ContainerRuntimeRouter::new(
                            pool.clone(),
                            docker.clone(),
                            agent.clone(),
                            edge_registry.clone(),
                        ),
                    )
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
        PostgresPlatformReadStore::new(pool.clone()),
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
            Arc::new(PostgresBuildStore::new(pool.clone())),
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
        Arc::new(PostgresPlatformRegistrationStore::new(pool.clone())),
        Arc::new(PlatformRegistrationRuntimeRouter::new(
            docker.clone(),
            agent.clone(),
        )),
    ));
    let deployments = Arc::new(
        DeploymentService::new(
            Arc::new(PostgresDeploymentStore::new(pool.clone())),
            Arc::new(
                DeploymentRuntimeRouter::new(pool.clone(), docker.clone(), agent.clone())
                    .with_edge(edge_registry.clone()),
            ),
            entitlements.clone(),
            cancellation.clone(),
        )
        .with_notifier(Arc::new(
            deployments_http::DeploymentsRealtimeNotifier::new(realtime_hub.clone()),
        ))
        .with_binding_resolver(Arc::new(PostgresDeploymentBindingResolver::new(
            pool.clone(),
            secret_protector.clone(),
        )?))
        .with_adoption(Arc::new(
            citadel_adapters::deployment_store::PostgresContainerAdoption::new(
                pool.clone(),
                citadel_adapters::container_mutations::ContainerRuntimeRouter::new(
                    pool.clone(),
                    docker.clone(),
                    agent.clone(),
                    edge_registry.clone(),
                ),
                secret_protector.clone(),
                config.identity.secret_encryption_key.expose(),
            ),
        ))
        .with_alerts(alert_store.clone()),
    );
    let swarm_services = Arc::new(
        ManagedSwarmServiceService::new(
            Arc::new(PostgresSwarmServiceStore::new(pool.clone())),
            Arc::new(
                SwarmServiceRuntimeRouter::new(pool.clone(), docker.clone(), agent.clone())
                    .with_edge(edge_registry.clone()),
            ),
            cancellation.clone(),
        )
        .with_image_digests(Arc::new(
            SwarmServiceRuntimeRouter::new(pool.clone(), docker.clone(), agent.clone())
                .with_edge(edge_registry.clone()),
        ))
        .with_adoption(Arc::new(
            citadel_adapters::swarm_service_store::PostgresSwarmServiceAdoption::new(
                pool.clone(),
                SwarmServiceRuntimeRouter::new(pool.clone(), docker.clone(), agent.clone())
                    .with_edge(edge_registry.clone()),
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
            Arc::new(PostgresStackStore::new(pool.clone())),
            Arc::new(
                StackRuntimeRouter::new(pool.clone(), docker.clone(), agent.clone())
                    .with_edge(edge_registry.clone()),
            ),
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
                StackRuntimeRouter::new(pool.clone(), docker.clone(), agent.clone())
                    .with_edge(edge_registry.clone()),
                git_execution.clone(),
            ),
        ))
        .with_entitlements(entitlements.clone())
        .with_alerts(alert_store.clone()),
    );
    let container_hub = realtime_hub.clone();
    let container_mutations = Arc::new(
        citadel_adapters::container_mutations::ContainerRuntimeRouter::new(
            pool.clone(),
            docker.clone(),
            agent.clone(),
            edge_registry.clone(),
        )
        .into_service()
        .with_notifier(move |claim| {
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
        }),
    );
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
        resource_metadata,
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
                identity: Arc::clone(&identity),
                platforms: Arc::clone(&platform_reads),
                deployments: Arc::new(PostgresDeploymentStore::new(pool.clone())),
                stacks: Arc::new(PostgresStackStore::new(pool.clone())),
                services: Arc::new(PostgresSwarmServiceStore::new(pool.clone())),
                resources: Arc::clone(resources.store()),
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
    let mut supervisor = TaskSupervisor::new(cancellation.clone());
    supervisor.spawn(
        "edge-platform-inventory",
        workers::edge::run(
            cancellation.child_token(),
            edge_registry.clone(),
            pool.clone(),
            realtime_hub.clone(),
        ),
    );
    supervisor.spawn(
        "service-account-last-used",
        last_used_worker.run(cancellation.child_token()),
    );
    supervisor.spawn(
        "license-transition-monitor",
        license_realtime::run_license_transition_monitor(
            cancellation.child_token(),
            license_transition_monitor,
            license_realtime_hub,
            alert_store.clone(),
        ),
    );
    workers::register(
        &mut supervisor,
        &cancellation,
        workers::WorkerDependencies {
            volume_content: platform_state.volume_content.clone(),
            containers: container_mutations,
            docker: docker.clone(),
            pool: pool.clone(),
            readiness: Arc::clone(&readiness),
            metrics: Arc::clone(&metrics),
            agent: agent.clone(),
            realtime: realtime_hub.clone(),
            deployments: Arc::clone(&deployments),
            swarm_services: Arc::clone(&swarm_services),
            stacks: Arc::clone(&stacks),
            git: Arc::clone(&git_execution),
            automation: Arc::clone(&automation),
            builds: Arc::clone(&builds),
            backups: Arc::clone(&backups),
            alerts: alert_store.clone(),
            alert_deliveries,
        },
        workers::WorkerSettings {
            queue_capacity: config.event_queue_capacity,
            probe_interval: config.probe_interval,
            reconciliation_interval: config.reconciliation_interval,
            agent_reconnect_delay: config
                .agent
                .as_ref()
                .map_or(Duration::from_secs(10), |agent| agent.reconnect_delay),
        },
    );

    let app = citadel_server::diagnostics_http::router(
        citadel_server::diagnostics_http::DiagnosticsHttpState {
            readiness: Arc::clone(&readiness),
            metrics,
            pool: pool.clone(),
        },
    )
    .merge(identity_http::router(identity_http::IdentityHttpState {
        identity: Arc::clone(&identity),
        mfa,
        readiness: Arc::clone(&readiness),
        secure_cookies: config.transport.mode != citadel_server::config::TransportMode::Disabled,
    }))
    .merge(oidc_http::router(oidc_http::OidcHttpState {
        oidc,
        public_url: config.transport.public_url.clone(),
        allowed_return_origins: config.transport.cors_origins.clone(),
        secure_cookies: config.transport.mode != citadel_server::config::TransportMode::Disabled,
    }))
    .merge(application_info_http::router())
    .merge(citadel_server::search_http::router(Arc::new(
        citadel_adapters::global_search::PostgresGlobalSearchStore::new(pool.clone()),
    )))
    .merge(citadel_server::actors_http::router(Arc::new(
        citadel_adapters::actor_store::PostgresActorStore::new(pool.clone()),
    )))
    .merge(license_http::router(license_http::LicenseHttpState {
        identity: Arc::clone(&identity),
        licenses,
    }))
    .merge(activities_http::router(
        activities_http::ActivitiesHttpState { activities },
    ))
    .merge(users_http::router(users_http::UsersHttpState {
        identity: Arc::clone(&identity),
        users,
        mutations: user_mutations,
    }))
    .merge(teams_http::router(teams_http::TeamsHttpState {
        identity: Arc::clone(&identity),
        teams,
        mutations: team_mutations,
    }))
    .merge(roles_http::router(roles_http::RolesHttpState {
        identity: Arc::clone(&identity),
        roles,
        mutations: role_mutations,
    }))
    .merge(service_accounts_http::router(
        service_accounts_http::ServiceAccountHttpState {
            identity: Arc::clone(&identity),
            service_accounts,
        },
    ))
    .merge(git_accounts_http::router(
        git_accounts_http::GitAccountsHttpState {
            identity: Arc::clone(&identity),
            accounts: git_accounts,
            realtime: realtime_hub.clone(),
        },
    ))
    .merge(git_repositories_http::router(
        git_repositories_http::GitRepositoriesHttpState {
            identity: Arc::clone(&identity),
            resources: Arc::clone(&resources),
            execution: Arc::clone(&git_execution),
            realtime: realtime_hub.clone(),
            cancellation: cancellation.clone(),
        },
    ))
    .merge(webhooks_http::router(webhooks_http::WebhooksHttpState {
        git: Arc::clone(&git_execution),
        automation: Arc::clone(&automation),
        backups: Some(backups.clone()),
        builds: Some(builds.clone()),
        stacks: Some(stacks.clone()),
        services: Some(swarm_services.clone()),
        audit: Some(Arc::new(PostgresActivityStore::new(pool.clone()))),
        alerts: Some(alert_store.clone()),
    }))
    .merge(automation_http::router(
        automation_http::AutomationHttpState {
            identity: Arc::clone(&identity),
            automation,
        },
    ))
    .merge(builds_http::router(builds_http::BuildsHttpState {
        identity: Arc::clone(&identity),
        builds,
    }))
    .merge(backups_http::router(backups_http::BackupsHttpState {
        identity: Arc::clone(&identity),
        backups,
        cancellation: cancellation.clone(),
    }))
    .merge(alerts_http::router(alerts_http::AlertsHttpState {
        identity: Arc::clone(&identity),
        store: alert_store,
        delivery: alert_delivery,
    }))
    .merge(deployments_http::router(
        deployments_http::DeploymentsHttpState {
            identity: Arc::clone(&identity),
            deployments: Arc::clone(&deployments),
        },
    ))
    .merge(swarm_services_http::router(
        swarm_services_http::SwarmServicesHttpState {
            identity: Arc::clone(&identity),
            services: Arc::clone(&swarm_services),
        },
    ))
    .merge(stacks_http::router(stacks_http::StacksHttpState {
        identity: Arc::clone(&identity),
        stacks,
    }))
    .merge(resources_http::router(resources_http::ResourcesHttpState {
        identity: Arc::clone(&identity),
        resources,
        realtime: realtime_hub.clone(),
    }))
    .merge({
        platforms_http::router(platform_state.clone())
            .layer(axum::Extension(agent_setup))
            .layer(axum::Extension(
                citadel_server::platforms_http::AgentSetupContext {
                    signer: agent_signer,
                    image: agent_image.clone(),
                    requires_tls: !citadel_server::config::agent_allows_insecure()?,
                },
            ))
            .merge(citadel_server::lookup_http::router(
                citadel_server::lookup_http::LookupHttpState {
                    store: Arc::new(citadel_adapters::lookup_store::PostgresLookupStore::new(
                        pool.clone(),
                    )),
                    entitlements: entitlements.clone(),
                    platforms: platform_state,
                },
            ))
    })
    .merge(profile_http::router(profile_http::ProfileHttpState {
        profiles,
    }));
    let mut app = if let Some(realtime) = realtime {
        app.merge(realtime.router())
    } else if let Some(license_realtime) = license_realtime_service {
        app.merge(license_realtime.router())
    } else {
        app
    };
    if let Some(hub) = realtime_hub {
        app = app.layer(axum::Extension(hub));
    }
    if config.transport.openapi_enabled {
        app = app
            .route("/openapi/v1.json", get(openapi_full))
            .route("/openapi/public/v1.json", get(openapi_public));
    }
    let edge_service = citadel_contracts::citadel::edge::v1::edge_agent_service_server::EdgeAgentServiceServer::new(
        citadel_adapters::edge::EdgeIntake::new(citadel_adapters::edge::PostgresEdgeStore::new(pool.clone()), edge_registry.clone()),
    ).max_decoding_message_size(citadel_adapters::edge::MAX_PAYLOAD + 4096)
        .max_encoding_message_size(citadel_adapters::edge::MAX_PAYLOAD + 4096);
    let edge_routes = tonic::service::Routes::new(edge_service).into_axum_router();
    app = app.layer(axum::Extension(platforms_http::EdgeHttpContext {
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
        registry: edge_registry,
        core_url: config
            .transport
            .edge_agent_public_url
            .to_string()
            .trim_end_matches('/')
            .to_owned(),
        agent_image,
    }));
    let mut edge_transport = config.transport.clone();
    edge_transport.static_root = None;
    let edge_app = transport::secure_router_with_grpc(
        Router::new(),
        &edge_transport,
        Arc::clone(&readiness),
        Some(edge_routes),
    )?;
    let app = transport::secure_router(
        app.layer(middleware::from_fn_with_state(
            identity,
            identity_http::authentication_middleware,
        ))
        .layer(CatchPanicLayer::custom(transport::panic_response))
        .layer(TraceLayer::new_for_http()),
        &config.transport,
        Arc::clone(&readiness),
    )?;
    tracing::info!(
        address = %config.listen_address,
        mode = ?config.transport.mode,
        "Rust foundation server listening"
    );

    let server = run_server(
        config.listen_address,
        config.transport.clone(),
        app,
        edge_app,
        cancellation.clone(),
        config.shutdown_timeout,
    );
    tokio::pin!(server);
    let exit = tokio::select! {
        server_result = &mut server => ServerExit::Server(server_result),
        task_result = supervisor.wait_for_exit() => ServerExit::Task(task_result),
    };
    cancellation.cancel();
    if matches!(exit, ServerExit::Task(_)) {
        let _ = tokio::time::timeout(config.shutdown_timeout, &mut server).await;
    }
    let supervisor_result = supervisor.shutdown(config.shutdown_timeout).await;
    let pool_close_result = tokio::time::timeout(config.shutdown_timeout, pool.close()).await;

    match exit {
        ServerExit::Server(result) => result?,
        ServerExit::Task(result) => result?,
    }
    supervisor_result?;
    if pool_close_result.is_err() {
        return Err("PostgreSQL pool close exceeded the shutdown timeout".into());
    }
    Ok(())
}

async fn migrate(config: DatabaseConfig) -> Result<(), Box<dyn std::error::Error>> {
    let outcome = MigrationRunner::migrate(&config.database_url).await?;
    println!(
        "database ready: {} migration(s) applied, {} already applied",
        outcome.applied, outcome.already_applied
    );
    Ok(())
}

enum ServerExit {
    Server(std::io::Result<()>),
    Task(Result<(), citadel_application::SupervisedTaskError>),
}

async fn run_server(
    address: std::net::SocketAddr,
    transport_config: citadel_server::config::TransportConfig,
    app: Router,
    edge_app: Router,
    cancellation: CancellationToken,
    shutdown_timeout: Duration,
) -> std::io::Result<()> {
    let http = async {
        tokio::try_join!(
            transport::serve(
                address,
                &transport_config,
                app,
                cancellation.clone(),
                shutdown_timeout
            ),
            transport::serve(
                std::net::SocketAddr::new(address.ip(), transport_config.edge_grpc_port),
                &transport_config,
                edge_app,
                cancellation.clone(),
                shutdown_timeout
            ),
        )
        .map(|_| ())
    };
    tokio::pin!(http);
    tokio::select! {
        result = &mut http => result,
        () = shutdown_signal(cancellation.clone()) => http.await,
    }
}

async fn connect_database(config: &Config) -> Result<PgPool, sqlx::Error> {
    postgres_runtime::connect_pool(
        &config.database_url,
        config.database_max_connections,
        config.docker_request_timeout,
    )
    .await
}

async fn phase0_smoke(
    config: Config,
    actor_id: Option<Uuid>,
) -> Result<(), Box<dyn std::error::Error>> {
    let pool = connect_database(&config).await?;
    let docker = DockerClient::new(&config.docker_socket, config.docker_request_timeout)?;
    docker.ping().await?;
    let version = docker.version().await?;
    let negotiated = docker.negotiated_version().await?;
    let info = docker.info().await?;
    let containers = docker.list_containers(true).await?;
    let inspected = if let Some(container) = containers.first() {
        Some(docker.inspect_container(&container.id).await?.id)
    } else {
        None
    };
    let running = containers
        .iter()
        .find(|container| container.state == "running");
    let stats_sample = if let Some(container) = running {
        let mut stream = docker.container_stats(&container.id).await?;
        match tokio::time::timeout(Duration::from_secs(15), stream.next()).await {
            Ok(Some(Ok(sample))) => Some(sample.id),
            Ok(Some(Err(error))) => return Err(error.into()),
            _ => None,
        }
    } else {
        None
    };
    let event_stream_opened = docker.events(None, None).await.is_ok();
    let authorized_platform_count = if let Some(actor_id) = actor_id {
        let reader = PostgresAuthorizedPlatformReader::new(pool.clone());
        Some(reader.list_authorized(ActorId::new(actor_id)).await?.len())
    } else {
        None
    };

    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "dockerVersion": version.version,
            "daemonApiVersion": version.api_version,
            "negotiatedApiVersion": negotiated.to_string(),
            "daemonId": info.id,
            "containerCount": containers.len(),
            "inspectedContainerId": inspected,
            "statsContainerId": stats_sample,
            "eventStreamOpened": event_stream_opened,
            "authorizedPlatformCount": authorized_platform_count,
        }))?
    );
    pool.close().await;
    Ok(())
}

async fn phase0_agent_smoke(config: Config) -> Result<(), Box<dyn std::error::Error>> {
    let agent_config = config
        .agent
        .as_ref()
        .ok_or("CITADEL_RUST_AGENT_ADDRESS and CITADEL_RUST_AGENT_PRIVATE_KEY_PATH are required")?;
    let local = DockerClient::new(&config.docker_socket, config.docker_request_timeout)?;
    let agent = AgentClient::connect(
        &agent_config.address,
        AgentRequestSigner::from_file(&agent_config.private_key_path)?,
        agent_config.operation_timeout,
        agent_config.allow_insecure,
    )
    .await?;
    let cancellation = CancellationToken::new();
    let local_info = local.get_info(&cancellation).await?;
    let agent_info = agent.get_info(&cancellation).await?;
    let local_containers = PlatformRuntimePort::list_containers(&local, &cancellation).await?;
    let agent_containers = agent.list_containers(&cancellation).await?;
    let local_ids = local_containers
        .iter()
        .map(|container| container.id.as_str())
        .collect::<Vec<_>>();
    let agent_ids = agent_containers
        .iter()
        .map(|container| container.id.as_str())
        .collect::<Vec<_>>();
    let equivalent = local_info.daemon_id == agent_info.daemon_id
        && local_info.api_version == agent_info.api_version
        && local_info.minimum_api_version == agent_info.minimum_api_version
        && local_ids == agent_ids;
    if !equivalent {
        return Err("Local and Agent capability results differ".into());
    }

    let mut stream = agent
        .stream_stats(config.probe_interval, &cancellation)
        .await?;
    let stats = tokio::time::timeout(Duration::from_secs(15), stream.next())
        .await
        .map_err(|_| "Agent did not produce a stats sample within 15 seconds")?
        .ok_or("Agent ended the stats stream before producing a sample")??;

    let stream_cancellation = cancellation.child_token();
    let mut cancellation_stream = agent
        .stream_stats(config.probe_interval, &stream_cancellation)
        .await?;
    stream_cancellation.cancel();
    let cancellation_observed =
        tokio::time::timeout(Duration::from_secs(2), cancellation_stream.next())
            .await
            .is_ok_and(|item| item.is_none());
    if !cancellation_observed {
        return Err("Agent stats stream did not observe cancellation within two seconds".into());
    }

    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "agentVersion": agent_info.agent_version,
            "daemonId": agent_info.daemon_id,
            "apiVersion": agent_info.api_version,
            "minimumApiVersion": agent_info.minimum_api_version,
            "containerCount": agent_containers.len(),
            "localAndAgentEquivalent": equivalent,
            "streamSample": stats,
            "streamCancellationObserved": cancellation_observed,
        }))?
    );
    Ok(())
}

async fn openapi_full() -> Response {
    (
        [(header::CONTENT_TYPE, "application/json")],
        citadel_server::openapi::json_document(false),
    )
        .into_response()
}

async fn openapi_public() -> Response {
    (
        [(header::CONTENT_TYPE, "application/json")],
        citadel_server::openapi::json_document(true),
    )
        .into_response()
}

async fn shutdown_signal(cancellation: CancellationToken) {
    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut signal) => {
                signal.recv().await;
            }
            Err(error) => {
                tracing::error!(%error, "failed to install SIGTERM handler");
                std::future::pending::<()>().await;
            }
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        result = tokio::signal::ctrl_c() => {
            if let Err(error) = result {
                tracing::error!(%error, "failed to install Ctrl+C handler");
            }
        }
        () = terminate => {}
        () = cancellation.cancelled() => return,
    }
    cancellation.cancel();
}
