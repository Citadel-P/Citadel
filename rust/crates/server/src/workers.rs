use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use citadel_adapters::docker::{DockerClient, DockerError};
use citadel_application::TaskSupervisor;
use futures_util::StreamExt;
use sqlx::PgPool;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::Readiness;
use crate::metrics::Metrics;

pub struct WorkerSettings {
    pub queue_capacity: usize,
    pub probe_interval: Duration,
}

pub fn register(
    supervisor: &mut TaskSupervisor,
    cancellation: &CancellationToken,
    docker: DockerClient,
    pool: PgPool,
    readiness: Arc<Readiness>,
    metrics: Arc<Metrics>,
    settings: WorkerSettings,
) {
    let (sender, receiver) = mpsc::channel(settings.queue_capacity);

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
        event_consumer(cancellation.child_token(), receiver, Arc::clone(&metrics)),
    );
    supervisor.spawn(
        "readiness-probe",
        readiness_probe(
            cancellation.child_token(),
            docker,
            pool,
            readiness,
            metrics,
            settings.probe_interval,
        ),
    );
}

async fn event_source(
    cancellation: CancellationToken,
    docker: DockerClient,
    sender: mpsc::Sender<citadel_adapters::docker::generated::DockerEvent>,
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
                    let permit = tokio::select! {
                        () = cancellation.cancelled() => return Ok(()),
                        permit = sender.reserve() => match permit {
                            Ok(permit) => permit,
                            Err(_) => return Ok(()),
                        }
                    };
                    metrics.event_enqueued();
                    permit.send(event);
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
    mut receiver: mpsc::Receiver<citadel_adapters::docker::generated::DockerEvent>,
    metrics: Arc<Metrics>,
) -> Result<(), std::convert::Infallible> {
    let _task = metrics.task_guard();
    loop {
        let event = tokio::select! {
            () = cancellation.cancelled() => return Ok(()),
            event = receiver.recv() => match event {
                Some(event) => event,
                None => return Ok(()),
            }
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
                readiness.set(database_ready, docker_ready);
                if !database_ready || !docker_ready {
                    metrics.readiness_failed();
                }
            }
        }
    }
}
