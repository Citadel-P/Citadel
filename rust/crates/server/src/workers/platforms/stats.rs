use super::*;

pub(super) async fn local_container_stats(
    cancellation: CancellationToken,
    docker: DockerClient,
    context: StatsWorkerContext,
) -> Result<(), std::convert::Infallible> {
    let _task = context.metrics.task_guard();
    let store = PostgresContainerStatsStore::new(context.pool.clone());
    let mut sampler = citadel_adapters::docker::LocalDockerSampler::new(docker, STATS_CONCURRENCY);
    let mut ticker = super::super::schedule::interval("local-stats", context.fetch_interval);
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        tokio::select! {
            biased;
            () = cancellation.cancelled() => return Ok(()),
            _ = ticker.tick() => {}
        }
        let _cycle = citadel_application::runtime_metrics::RuntimeWork::LocalStats.start();
        let Some(platform_id) = context
            .targets
            .snapshot()
            .await
            .iter()
            .find(|target| target.connector_type == "Local")
            .map(|target| target.id)
        else {
            continue;
        };
        let sample = match sampler.sample(&cancellation).await {
            Ok(sample) => sample,
            Err(error) => {
                if cancellation.is_cancelled() {
                    return Ok(());
                }
                tracing::warn!(%error, "Local statistics collection failed");
                continue;
            }
        };
        let batch = sample.containers;
        RuntimeWork::LocalStats.units((batch.stats.len() + batch.failed_samples) as u64);
        RuntimeWork::LocalStats.failures(batch.failed_samples as u64);
        if batch.failed_samples != 0 {
            tracing::warn!(
                failed_samples = batch.failed_samples,
                %platform_id,
                "some local container statistics samples failed"
            );
        }
        if batch.stats.is_empty() && batch.failed_samples != 0 {
            continue;
        }
        let stats = batch.stats;
        let disk = sample.platform;
        match persist_stats_retry(&store, platform_id, &stats, disk.as_ref(), &cancellation).await {
            Ok(0) if !stats.is_empty() => continue,
            Ok(_) => {
                context.metrics.local_stats_sampled();
                if let Some(realtime) = context.realtime.as_ref() {
                    realtime.publish_container_stats(platform_id, &stats);
                }
            }
            Err(error) => {
                tracing::warn!(%error, %platform_id, "local container statistics persistence failed");
            }
        }
    }
}

pub(super) async fn agent_container_stats(
    cancellation: CancellationToken,
    platform: uuid::Uuid,
    _agent: AgentClient,
    context: StatsWorkerContext,
    reconnect_delay: Duration,
) -> Result<(), RuntimeCapabilityError> {
    let _task = context.metrics.task_guard();
    let store = PostgresContainerStatsStore::new(context.pool.clone());
    loop {
        let (platform_id, agent) = match agent_subscription(&context.targets, platform).await {
            Ok(Some(target)) => target,
            Ok(None) => {
                if wait_to_reconnect(&cancellation, reconnect_delay).await {
                    return Ok(());
                }
                continue;
            }
            Err(error) => {
                tracing::warn!(%error, "Agent Platform lookup for statistics failed");
                if wait_to_reconnect(&cancellation, reconnect_delay).await {
                    return Ok(());
                }
                continue;
            }
        };
        if let Err(error) = agent.get_info(&cancellation).await {
            if cancellation.is_cancelled() {
                return Ok(());
            }
            context.metrics.agent_handshake_failed();
            tracing::warn!(%error, "Agent handshake failed");
            if wait_to_reconnect(&cancellation, reconnect_delay).await {
                return Ok(());
            }
            continue;
        }
        let mut stream = match agent
            .stream_container_stats(context.fetch_interval, &cancellation)
            .await
        {
            Ok(stream) => stream,
            Err(_error) if cancellation.is_cancelled() => return Ok(()),
            Err(error) => {
                context.metrics.agent_stream_reconnected();
                tracing::warn!(%error, "Agent stats stream connection failed");
                if wait_to_reconnect(&cancellation, reconnect_delay).await {
                    return Ok(());
                }
                continue;
            }
        };
        let mut disk = super::super::disk::LatestPlatformStats::new(context.fetch_interval);
        let mut disk_updates = super::super::disk::samples(
            agent.clone(),
            context.fetch_interval,
            cancellation.clone(),
        );
        let changed = context.targets.reconfigured(platform_id, agent.address());
        tokio::pin!(changed);
        loop {
            let next = tokio::select! {
                biased;
                () = cancellation.cancelled() => return Ok(()),
                () = &mut changed => break,
                update = disk_updates.next() => { disk.record(update.flatten()); continue; },
                next = stream.next() => next,
            };
            match next {
                Some(Ok(stats)) => {
                    let _iteration = RuntimeWork::AgentStats.start();
                    RuntimeWork::AgentStats.units(stats.len() as u64);
                    match persist_stats_retry(
                        &store,
                        platform_id,
                        &stats,
                        disk.get(),
                        &cancellation,
                    )
                    .await
                    {
                        Ok(0) if !stats.is_empty() => continue,
                        Ok(_) => {
                            context.metrics.agent_stats_sampled();
                            if let Some(realtime) = context.realtime.as_ref() {
                                realtime.publish_container_stats(platform_id, &stats);
                            }
                        }
                        Err(error) => {
                            tracing::warn!(%error, %platform_id, "Agent container statistics persistence failed");
                        }
                    }
                }
                Some(Err(error)) => {
                    context.metrics.agent_stream_reconnected();
                    tracing::warn!(%error, "Agent stats stream interrupted");
                    break;
                }
                None if cancellation.is_cancelled() => return Ok(()),
                None => {
                    context.metrics.agent_stream_reconnected();
                    break;
                }
            }
        }
        if wait_to_reconnect(&cancellation, reconnect_delay).await {
            return Ok(());
        }
    }
}
