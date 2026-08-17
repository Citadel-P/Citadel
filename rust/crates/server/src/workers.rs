use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use citadel_adapters::agent::AgentClient;
use citadel_adapters::docker::{DockerClient, DockerError};
use citadel_application::{
    BoundedReceiver, BoundedSender, QueueOverflowPolicy, TaskSupervisor, bounded_channel,
};
use citadel_platforms::{PlatformRuntimePort, RuntimeCapabilityError};
use futures_util::StreamExt;
use sqlx::PgPool;
use tokio_util::sync::CancellationToken;

use crate::Readiness;
use crate::metrics::Metrics;
use crate::realtime::RealtimeHub;

pub struct WorkerSettings {
    pub queue_capacity: usize,
    pub probe_interval: Duration,
    pub agent_reconnect_delay: Duration,
}

pub struct WorkerDependencies {
    pub docker: DockerClient,
    pub pool: PgPool,
    pub readiness: Arc<Readiness>,
    pub metrics: Arc<Metrics>,
    pub agent: Option<AgentClient>,
    pub realtime: Option<RealtimeHub>,
}

pub fn register(
    supervisor: &mut TaskSupervisor,
    cancellation: &CancellationToken,
    dependencies: WorkerDependencies,
    settings: WorkerSettings,
) {
    let WorkerDependencies {
        docker,
        pool,
        readiness,
        metrics,
        agent,
        realtime,
    } = dependencies;
    let (sender, receiver) = bounded_channel(settings.queue_capacity, QueueOverflowPolicy::Wait);

    supervisor.spawn(
        "docker-event-source",
        event_source(
            cancellation.child_token(),
            docker.clone(),
            sender,
            Arc::clone(&metrics),
        ),
    );
    supervisor.spawn(
        "docker-event-consumer",
        event_consumer(
            cancellation.child_token(),
            receiver,
            Arc::clone(&metrics),
            realtime.clone(),
        ),
    );
    supervisor.spawn(
        "readiness-probe",
        readiness_probe(
            cancellation.child_token(),
            docker.clone(),
            pool,
            readiness,
            Arc::clone(&metrics),
            settings.probe_interval,
        ),
    );
    if let Some(agent) = agent {
        supervisor.spawn(
            "agent-stats-stream",
            agent_stats_stream(
                cancellation.child_token(),
                agent,
                Arc::clone(&metrics),
                settings.probe_interval,
                settings.agent_reconnect_delay,
            ),
        );
    }
    if let Some(realtime) = realtime {
        supervisor.spawn(
            "local-container-stats",
            local_container_stats(
                cancellation.child_token(),
                docker,
                Arc::clone(&metrics),
                realtime,
                settings.probe_interval,
            ),
        );
    }
}

async fn local_container_stats(
    cancellation: CancellationToken,
    docker: DockerClient,
    metrics: Arc<Metrics>,
    realtime: RealtimeHub,
    reconnect_delay: Duration,
) -> Result<(), std::convert::Infallible> {
    let _task = metrics.task_guard();
    loop {
        let containers = match PlatformRuntimePort::list_containers(&docker, &cancellation).await {
            Ok(containers) => containers,
            Err(error) => {
                if cancellation.is_cancelled() {
                    return Ok(());
                }
                tracing::warn!(%error, "local container inventory for statistics failed");
                if wait_to_reconnect(&cancellation, reconnect_delay).await {
                    return Ok(());
                }
                continue;
            }
        };
        let Some(container) = containers
            .into_iter()
            .find(|container| container.state == "running")
        else {
            if wait_to_reconnect(&cancellation, reconnect_delay).await {
                return Ok(());
            }
            continue;
        };
        let mut stream = match docker.container_stats(&container.id).await {
            Ok(stream) => stream,
            Err(error) => {
                tracing::warn!(%error, container_id = %container.id, "local container statistics stream failed to open");
                if wait_to_reconnect(&cancellation, reconnect_delay).await {
                    return Ok(());
                }
                continue;
            }
        };
        loop {
            let sample = tokio::select! {
                () = cancellation.cancelled() => return Ok(()),
                sample = stream.next() => sample,
            };
            match sample {
                Some(Ok(sample)) => {
                    metrics.local_stats_sampled();
                    realtime.publish_runtime_change("containerStats", "sample", sample.id);
                }
                Some(Err(error)) => {
                    tracing::warn!(%error, container_id = %container.id, "local container statistics stream was interrupted");
                    break;
                }
                None => break,
            }
        }
        if wait_to_reconnect(&cancellation, reconnect_delay).await {
            return Ok(());
        }
    }
}

async fn agent_stats_stream(
    cancellation: CancellationToken,
    agent: AgentClient,
    metrics: Arc<Metrics>,
    fetch_interval: Duration,
    reconnect_delay: Duration,
) -> Result<(), RuntimeCapabilityError> {
    let _task = metrics.task_guard();
    loop {
        if let Err(error) = agent.get_info(&cancellation).await {
            if cancellation.is_cancelled() {
                return Ok(());
            }
            metrics.agent_handshake_failed();
            if !error.retryable {
                return Err(error);
            }
            tracing::warn!(%error, "Agent handshake failed");
            if wait_to_reconnect(&cancellation, reconnect_delay).await {
                return Ok(());
            }
            continue;
        }
        let mut stream = match agent.stream_stats(fetch_interval, &cancellation).await {
            Ok(stream) => stream,
            Err(_error) if cancellation.is_cancelled() => return Ok(()),
            Err(error) if error.retryable => {
                metrics.agent_stream_reconnected();
                tracing::warn!(%error, "Agent stats stream connection failed");
                if wait_to_reconnect(&cancellation, reconnect_delay).await {
                    return Ok(());
                }
                continue;
            }
            Err(error) => return Err(error),
        };
        loop {
            match stream.next().await {
                Some(Ok(_sample)) => metrics.agent_stats_sampled(),
                Some(Err(error)) if error.retryable => {
                    metrics.agent_stream_reconnected();
                    tracing::warn!(%error, "Agent stats stream interrupted");
                    break;
                }
                Some(Err(error)) => return Err(error),
                None if cancellation.is_cancelled() => return Ok(()),
                None => {
                    metrics.agent_stream_reconnected();
                    break;
                }
            }
        }
        if wait_to_reconnect(&cancellation, reconnect_delay).await {
            return Ok(());
        }
    }
}

async fn wait_to_reconnect(cancellation: &CancellationToken, reconnect_delay: Duration) -> bool {
    tokio::select! {
        biased;
        () = cancellation.cancelled() => true,
        () = tokio::time::sleep(reconnect_delay) => false,
    }
}

async fn event_source(
    cancellation: CancellationToken,
    docker: DockerClient,
    sender: BoundedSender<citadel_adapters::docker::generated::DockerEvent>,
    metrics: Arc<Metrics>,
) -> Result<(), DockerError> {
    let _task = metrics.task_guard();
    let mut retry = Duration::from_millis(250);
    loop {
        let stream = tokio::select! {
            () = cancellation.cancelled() => return Ok(()),
            result = docker.events(None, None) => result,
        };
        let mut stream = match stream {
            Ok(stream) => {
                retry = Duration::from_millis(250);
                stream
            }
            Err(error) => {
                metrics.stream_reconnected();
                tracing::warn!(%error, "Docker event stream connection failed");
                tokio::select! {
                    () = cancellation.cancelled() => return Ok(()),
                    () = tokio::time::sleep(retry) => {}
                }
                retry = std::cmp::min(retry.saturating_mul(2), Duration::from_secs(5));
                continue;
            }
        };

        loop {
            let event = tokio::select! {
                () = cancellation.cancelled() => return Ok(()),
                event = stream.next() => event,
            };
            match event {
                Some(Ok(event)) => {
                    if sender.send(event, &cancellation).await.is_err() {
                        return Ok(());
                    }
                    metrics.event_enqueued();
                }
                Some(Err(error)) => {
                    metrics.stream_reconnected();
                    tracing::warn!(%error, "Docker event stream interrupted");
                    break;
                }
                None => {
                    metrics.stream_reconnected();
                    break;
                }
            }
        }
    }
}

async fn event_consumer(
    cancellation: CancellationToken,
    mut receiver: BoundedReceiver<citadel_adapters::docker::generated::DockerEvent>,
    metrics: Arc<Metrics>,
    realtime: Option<RealtimeHub>,
) -> Result<(), std::convert::Infallible> {
    let _task = metrics.task_guard();
    loop {
        let event = match receiver.recv(&cancellation).await {
            Some(event) => event,
            None => return Ok(()),
        };
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        let event_time = if event.time_nano > 0 {
            u128::try_from(event.time_nano).unwrap_or_default() / 1_000_000
        } else {
            u128::try_from(event.time).unwrap_or_default() * 1_000
        };
        let lag = now.saturating_sub(event_time).min(u128::from(u64::MAX)) as u64;
        metrics.event_consumed(lag);
        if let Some(realtime) = &realtime {
            realtime.publish_runtime_change(event.resource_type, event.action, event.actor.id);
        }
    }
}

async fn readiness_probe(
    cancellation: CancellationToken,
    docker: DockerClient,
    pool: PgPool,
    readiness: Arc<Readiness>,
    metrics: Arc<Metrics>,
    interval: Duration,
) -> Result<(), std::convert::Infallible> {
    let _task = metrics.task_guard();
    let mut ticker = tokio::time::interval(interval);
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        tokio::select! {
            () = cancellation.cancelled() => return Ok(()),
            _ = ticker.tick() => {
                let database_ready = sqlx::query_scalar::<_, i32>("SELECT 1")
                    .fetch_one(&pool)
                    .await
                    .is_ok();
                let docker_ready = docker.ping().await.is_ok();
                let setup = if readiness.is_setup() {
                    true
                } else {
                    sqlx::query_scalar::<_, bool>(
                        "SELECT initializedat IS NOT NULL FROM instancesetupstates ORDER BY id LIMIT 1"
                    )
                    .fetch_optional(&pool)
                    .await
                    .ok()
                    .flatten()
                    .unwrap_or(false)
                };
                readiness.set(database_ready, docker_ready);
                readiness.set_setup(setup);
                if !database_ready || !docker_ready {
                    metrics.readiness_failed();
                }
            }
        }
    }
}
