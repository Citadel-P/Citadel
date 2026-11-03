//! Auxiliary executable commands; these do not construct the running application.
use crate::composition::connect_database;
use citadel_adapters::connectors::agent::client::AgentClient;
use citadel_adapters::connectors::agent::client::AgentRequestSigner;
use citadel_adapters::external::backups::system_recovery::CitadelSystemRestoreOptions;
use citadel_adapters::external::backups::system_recovery::restore_citadel_system;
use citadel_adapters::persistence::postgres::platforms::PostgresAuthorizedPlatformReader;
use citadel_database::MigrationRunner;
use citadel_docker::client::DockerClient;
use citadel_platforms::AuthorizedPlatformReader;
use citadel_platforms::{ContainerInventoryPort, PlatformInfoPort, PlatformStatsPort};
use citadel_primitives::ActorId;
use citadel_server::config::{Config, DatabaseConfig};
use clap::{Parser, Subcommand};
use futures_util::StreamExt;
use std::time::Duration;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

#[derive(Parser)]
#[command(name = "citadel-server", about = "Citadel Core")]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Print embedded display and informational versions without starting Core.
    VersionJson,
    /// Print this build's upgrade descriptor without starting Core or accessing a database.
    UpgradeDescriptorJson,
    /// Read the initialized instance identity without starting Core or migrating its database.
    InstanceIdentityJson {
        /// Also require a complete matching migration journal and no owned update.
        #[arg(long)]
        verify_migrations: bool,
    },
    /// Inspect existing recovery keys without exposing them or creating replacements.
    #[cfg(target_os = "linux")]
    CheckUpdateRecoveryKeys,
    /// Verify a published Core image signature without installing or starting Core.
    VerifyUpdateRelease {
        /// Exact published version, without the v prefix (for example 0.1.2).
        #[arg(long)]
        version: String,
    },
    /// Verify and inspect a release in a disposable local Docker container; never installs it.
    #[cfg(target_os = "linux")]
    InspectUpdateRelease {
        /// Exact published version, without the v prefix.
        #[arg(long)]
        version: String,
        /// Target Linux architecture (amd64 or arm64).
        #[arg(long, value_parser = ["amd64", "arm64"])]
        architecture: String,
    },
    /// Validate installer Compose inputs without contacting Docker or changing services.
    #[cfg(target_os = "linux")]
    CheckUpdateCompose {
        /// Canonical host installation directory, mounted at the same path if containerized.
        #[arg(long)]
        directory: std::path::PathBuf,
        /// The explicit project name used to install Citadel.
        #[arg(long)]
        project: String,
        /// Original absolute Compose paths, in invocation order. Repeat for the Edge overlay.
        #[arg(long = "compose-file", required = true)]
        compose_files: Vec<std::path::PathBuf>,
    },
    /// Bind Docker Compose inputs to the running local installation and save private registration.
    #[cfg(target_os = "linux")]
    RegisterUpdateInstallation {
        #[arg(long)]
        directory: std::path::PathBuf,
        #[arg(long)]
        project: String,
        #[arg(long = "compose-file", required = true)]
        compose_files: Vec<std::path::PathBuf>,
        /// Existing private host directory, owned by the intended updater user.
        #[arg(long)]
        state_directory: std::path::PathBuf,
    },
    /// Revalidate a registered local installation without changing it or approving an update.
    #[cfg(target_os = "linux")]
    CheckUpdateInstallation {
        /// Existing registered private host directory; nothing is created or repaired.
        #[arg(long)]
        state_directory: std::path::PathBuf,
    },
    /// Internal bounded exchange executed inside the owned updater helper.
    #[cfg(target_os = "linux")]
    #[command(hide = true)]
    UpdateHelperHandoff,
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
    DockerSmoke {
        #[arg(long)]
        actor_id: Option<Uuid>,
    },
    /// Print the Base64 public key corresponding to a raw 32-byte Agent private key.
    AgentPublicKey {
        #[arg(long)]
        private_key_path: std::path::PathBuf,
    },
    /// Verify signed Agent handshake, reads, streams, cancellation, and Local equivalence.
    AgentSmoke {
        /// Platform or Build Pool UUID from the Agent setup command.
        #[arg(long)]
        agent_id: Uuid,
    },
    /// Probe an already-running server without curl in the image.
    Healthcheck {
        #[arg(long)]
        url: Option<String>,
    },
}

pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    match cli.command.unwrap_or(Command::Serve) {
        Command::VersionJson => {
            let mut info =
                serde_json::to_value(citadel_server::application_info_http::application_info())?;
            info["protocolVersion"] =
                serde_json::json!(citadel_contracts::EDGE_AGENT_PROTOCOL_VERSION);
            println!("{}", serde_json::to_string(&info)?);
            Ok(())
        }
        Command::UpgradeDescriptorJson => {
            let descriptor = citadel_updates::UpgradeDescriptor {
                schema_version: 1,
                product: "citadel".into(),
                target_version: env!("CITADEL_BUILD_VERSION").into(),
                target_catalog_sha256: MigrationRunner::catalog_sha256()?,
                installer_format: citadel_updates::INSTALLER_FORMAT,
                update_protocol: citadel_updates::UPDATE_PROTOCOL,
                // No source transition is qualified until executor/recovery
                // acceptance proves it against the actual source artifact.
                sources: vec![],
            };
            println!("{}", serde_json::to_string(&descriptor)?);
            Ok(())
        }
        Command::InstanceIdentityJson { verify_migrations } => {
            let database = DatabaseConfig::from_env()
                .map_err(|_| "Valid PostgreSQL configuration is required for the instance identity diagnostic.")?;
            let instance = tokio::time::timeout(Duration::from_secs(10), async {
                use citadel_adapters::persistence::postgres::{connection, updates};
                let pool =
                    connection::connect_pool(&database.database_url, 1, Duration::from_secs(5))
                        .await
                        .map_err(|_| "Cannot connect to the existing Citadel database.")?;
                let result = if verify_migrations {
                    updates::read_update_database_state(&pool, &CancellationToken::new())
                        .await
                        .and_then(|state| {
                            serde_json::to_value(state).map_err(|_| {
                                citadel_updates::UpdateError::Unavailable(
                                    "Cannot encode database validation.".into(),
                                )
                            })
                        })
                } else {
                    updates::read_instance_identity(&pool)
                        .await
                        .map(|instance| serde_json::json!({"instanceId": instance}))
                        .map_err(|_| {
                            citadel_updates::UpdateError::Unavailable(
                                "Cannot read an initialized Citadel instance identity.".into(),
                            )
                        })
                };
                pool.close().await;
                result.map_err(|error| error.to_string())
            })
            .await
            .map_err(|_| "Instance identity read timed out.")??;
            println!("{}", instance);
            Ok(())
        }
        #[cfg(target_os = "linux")]
        Command::CheckUpdateRecoveryKeys => {
            use zeroize::Zeroizing;
            // Read only the required settings: normal Config initialization may
            // generate keys and must never run in this diagnostic.
            let configured = |name| -> Result<Option<Zeroizing<String>>, &'static str> {
                match std::env::var(name) {
                    Ok(value) => Ok(Some(Zeroizing::new(value))),
                    Err(std::env::VarError::NotPresent) => Ok(None),
                    Err(_) => Err("Recovery-key configuration is not valid UTF-8."),
                }
            };
            if std::env::var_os("CITADEL_CORE_AGENT_PRIVATE_KEY_PATH").is_some()
                || std::env::var_os("CITADEL_CORE_AGENT_ADDRESS").is_some()
            {
                return Err(
                    "Remote Core Agent keys are outside the updater installation profile.".into(),
                );
            }
            let root = std::env::var_os("CITADEL_DATA_ROOT")
                .map(std::path::PathBuf::from)
                .unwrap_or_else(|| "/app/data".into());
            let jwt = configured("CITADEL_JWT_KEY")?;
            let encryption = configured("CITADEL_SECRETS_ENCRYPTION_KEY")?;
            let keys = citadel_adapters::external::updates::check_recovery_keys(
                &root,
                jwt.as_ref().map(|value| value.as_str()),
                encryption.as_ref().map(|value| value.as_str()),
            )?;
            println!("{}", serde_json::to_string(&keys)?);
            Ok(())
        }
        Command::VerifyUpdateRelease { version } => {
            use citadel_updates::ReleaseArtifactSource;
            if !citadel_server::config::release_checks_allowed_from_env()? {
                return Err("Release checks are disabled by server configuration.".into());
            }
            let source = citadel_adapters::external::updates::GithubReleaseArtifacts::new(
                std::sync::Arc::new(citadel_processes::SystemProcess),
            )?;
            let cancellation = CancellationToken::new();
            let verification = source.verify(&version, &cancellation);
            tokio::pin!(verification);
            let result = tokio::select! {
                result = &mut verification => result,
                () = crate::app::shutdown_signal(cancellation.clone()) => verification.await,
            }?;
            println!("{}", serde_json::to_string(&result)?);
            Ok(())
        }
        #[cfg(target_os = "linux")]
        Command::InspectUpdateRelease {
            version,
            architecture,
        } => {
            use citadel_adapters::external::updates::{
                DockerReleaseInspection, GithubReleaseArtifacts,
            };
            use citadel_updates::ReleaseInspectionService;
            use std::sync::Arc;
            if !citadel_server::config::release_checks_allowed_from_env()? {
                return Err("Release checks are disabled by server configuration.".into());
            }
            let process = Arc::new(citadel_processes::SystemProcess);
            let service = ReleaseInspectionService::new(
                Arc::new(GithubReleaseArtifacts::new(process.clone())?),
                Arc::new(DockerReleaseInspection::new(process)),
            );
            let cancellation = CancellationToken::new();
            let inspection = service.inspect(&version, &architecture, &cancellation);
            tokio::pin!(inspection);
            let result = tokio::select! {
                result = &mut inspection => result,
                () = crate::app::shutdown_signal(cancellation.clone()) => inspection.await,
            }?;
            println!("{}", serde_json::to_string(&result)?);
            Ok(())
        }
        #[cfg(target_os = "linux")]
        Command::CheckUpdateCompose {
            directory,
            project,
            compose_files,
        } => {
            let inputs = citadel_adapters::external::updates::InstallerComposeInputs {
                directory,
                project,
                compose_files,
            };
            let cancellation = CancellationToken::new();
            let check = inputs.check(&citadel_processes::SystemProcess, &cancellation);
            tokio::pin!(check);
            let result = tokio::select! {
                result = &mut check => result,
                () = crate::app::shutdown_signal(cancellation.clone()) => check.await,
            }?;
            println!("{}", serde_json::to_string(&result)?);
            Ok(())
        }
        #[cfg(target_os = "linux")]
        Command::RegisterUpdateInstallation {
            directory,
            project,
            compose_files,
            state_directory,
        } => {
            let inputs = citadel_adapters::external::updates::InstallerComposeInputs {
                directory,
                project,
                compose_files,
            };
            let cancellation = CancellationToken::new();
            let registration = citadel_adapters::external::updates::register_compose_installation(
                &inputs,
                state_directory,
                &citadel_processes::SystemProcess,
                &cancellation,
            );
            tokio::pin!(registration);
            let result = tokio::select! {
                result = &mut registration => result,
                () = crate::app::shutdown_signal(cancellation.clone()) => registration.await,
            }?;
            println!("{}", serde_json::to_string(&result)?);
            Ok(())
        }
        #[cfg(target_os = "linux")]
        Command::CheckUpdateInstallation { state_directory } => {
            let cancellation = CancellationToken::new();
            let check = citadel_adapters::external::updates::check_registered_compose_installation(
                &state_directory,
                &citadel_processes::SystemProcess,
                &cancellation,
            );
            tokio::pin!(check);
            let result = tokio::select! {
                result = &mut check => result,
                () = crate::app::shutdown_signal(cancellation.clone()) => check.await,
            }?;
            println!("{}", serde_json::to_string(&result)?);
            Ok(())
        }
        #[cfg(target_os = "linux")]
        Command::UpdateHelperHandoff => {
            use citadel_adapters::external::updates::{
                MAX_HANDOFF_REQUEST, execute_helper_handoff,
            };
            use std::io::Read;
            // A one-shot internal stdin command; the invoking Docker client owns
            // its deadline. No application tasks or database have been started.
            let mut input = Vec::new();
            std::io::stdin()
                .take(MAX_HANDOFF_REQUEST as u64 + 1)
                .read_to_end(&mut input)?;
            let cancellation = CancellationToken::new();
            let exchange =
                execute_helper_handoff(&input, &citadel_processes::SystemProcess, &cancellation);
            tokio::pin!(exchange);
            let output = tokio::select! {
                result = &mut exchange => result,
                () = crate::app::shutdown_signal(cancellation.clone()) => exchange.await,
            }?;
            println!("{}", std::str::from_utf8(&output)?);
            Ok(())
        }
        Command::Serve => crate::app::serve(Config::from_env()?).await,
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
        Command::DockerSmoke { actor_id } => docker_smoke(Config::from_env()?, actor_id).await,
        Command::AgentPublicKey { private_key_path } => {
            println!(
                "{}",
                AgentRequestSigner::from_file(&private_key_path)?.public_key_base64()
            );
            Ok(())
        }
        Command::AgentSmoke { agent_id } => agent_smoke(Config::from_env()?, agent_id).await,
        Command::Healthcheck { url } => {
            let mut client = reqwest::Client::builder().timeout(Duration::from_secs(2));
            let url = match url {
                Some(url) => url,
                None => {
                    let port: u16 = std::env::var("CITADEL_HTTP_LISTEN_PORT")
                        .unwrap_or_else(|_| "8000".into())
                        .parse()?;
                    let direct = std::env::var("CITADEL_TRANSPORT_MODE")
                        .is_ok_and(|mode| mode.trim().eq_ignore_ascii_case("Direct"));
                    if direct {
                        let certificate =
                            std::fs::read(std::env::var("CITADEL_TLS_CERTIFICATE_PATH")?)?;
                        // Local liveness probe: trust only Core's configured certificate chain.
                        // Its public DNS name need not include the loopback IP.
                        client = client
                            .no_proxy()
                            .tls_certs_only(reqwest::Certificate::from_pem_bundle(&certificate)?)
                            .tls_danger_accept_invalid_hostnames(true);
                    }
                    format!(
                        "{}://127.0.0.1:{port}/health",
                        if direct { "https" } else { "http" }
                    )
                }
            };
            client.build()?.get(url).send().await?.error_for_status()?;
            Ok(())
        }
    }
}

async fn restore_system(
    database: DatabaseConfig,
    bundle: std::path::PathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    let encryption_key = citadel_server::config::recovery_encryption_key_from_env()?;
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
    let _ = signal.await;
    result?;
    println!("Citadel system recovery bundle restored successfully.");
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

async fn docker_smoke(
    config: Config,
    actor_id: Option<Uuid>,
) -> Result<(), Box<dyn std::error::Error>> {
    let pool = connect_database(&config).await?;
    let docker = DockerClient::with_host_root(
        &config.docker_socket,
        config.docker_request_timeout,
        &config.execution.paths.host_root,
    )?;
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

async fn agent_smoke(config: Config, agent_id: Uuid) -> Result<(), Box<dyn std::error::Error>> {
    if agent_id.is_nil() {
        return Err("--agent-id must be the nonzero UUID from Agent setup".into());
    }
    let agent_config = config
        .agent
        .as_ref()
        .ok_or("CITADEL_CORE_AGENT_ADDRESS and CITADEL_CORE_AGENT_PRIVATE_KEY_PATH are required")?;
    let local = DockerClient::with_host_root(
        &config.docker_socket,
        config.docker_request_timeout,
        &config.execution.paths.host_root,
    )?;
    let agent = AgentClient::lazy(
        &agent_config.address,
        AgentRequestSigner::from_file(&agent_config.private_key_path)?.for_agent(agent_id),
        agent_config.operation_timeout,
        agent_config.allow_insecure,
    )?
    .with_ca_certificate(config.execution.edge_agent.ca_certificate.clone())?;
    let cancellation = CancellationToken::new();
    let local_info = local.get_info(&cancellation).await?;
    let agent_info = agent.get_info(&cancellation).await?;
    let local_containers =
        citadel_platforms::ContainerInventoryPort::list_containers(&local, &cancellation).await?;
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
