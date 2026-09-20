//! Auxiliary executable commands; these do not construct the running application.
use crate::state::connect_database;
use citadel_adapters::{
    PostgresAuthorizedPlatformReader,
    agent::{AgentClient, AgentRequestSigner},
    citadel_system_backup::{CitadelSystemRestoreOptions, restore_citadel_system},
    docker::DockerClient,
};
use citadel_database::MigrationRunner;
use citadel_platforms::{AuthorizedPlatformReader, PlatformRuntimePort};
use citadel_primitives::ActorId;
use citadel_server::config::{Config, DatabaseConfig};
use clap::{Parser, Subcommand};
use futures_util::StreamExt;
use std::time::Duration;
use tokio_util::sync::CancellationToken;
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

pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    match cli.command.unwrap_or(Command::Serve) {
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

async fn migrate(config: DatabaseConfig) -> Result<(), Box<dyn std::error::Error>> {
    let outcome = MigrationRunner::migrate(&config.database_url).await?;
    println!(
        "database ready: {} migration(s) applied, {} already applied",
        outcome.applied, outcome.already_applied
    );
    Ok(())
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
