use super::*;

// Own one pair of independently reconnecting streams per persisted Direct Agent.
pub(super) async fn agent_subscriptions(
    cancellation: CancellationToken,
    base: AgentClient,
    sender: BoundedSender<InventoryEvent>,
    context: StatsWorkerContext,
    reconnect_delay: Duration,
) -> Result<(), std::convert::Infallible> {
    let mut active =
        std::collections::HashMap::<uuid::Uuid, (String, CancellationToken, tokio::task::Id)>::new(
        );
    let mut tasks = tokio::task::JoinSet::new();
    let mut ticker = tokio::time::interval(Duration::from_secs(5));
    loop {
        tokio::select! {
            biased;
            () = cancellation.cancelled() => break,
            result = tasks.join_next_with_id(), if !tasks.is_empty() => {
                let task_id = match result {
                    Some(Ok((id, ())))=>Some(id),
                    Some(Err(error))=>{tracing::warn!(%error,"Agent subscription task failed");Some(error.id())},
                    None=>None,
                };
                if let Some(platform) = active.iter().find(|(_,(_,_,id))|Some(*id)==task_id).map(|(platform,_)|*platform)
                    && let Some((_,token,_))=active.remove(&platform) { token.cancel(); }
            }
            _ = ticker.tick() => {
                let targets: Vec<_> = context.targets.snapshot().await.iter().filter(|target| target.connector_type == citadel_platforms::ConnectorKind::Agent).map(|target| (target.id, target.address.clone())).collect();
                active.retain(|id, (address, token, _)| {
                    let keep = targets.iter().any(|(target, endpoint)| target == id && endpoint == address);
                    if !keep { token.cancel(); }
                    keep
                });
                for (id, address) in targets {
                    if active.contains_key(&id) { continue; }
                    let token = cancellation.child_token();
                    let owned_token = token.clone();
                    let base = base.clone();
                    let context = context.clone();
                    let sender = sender.clone();
                    let task = tasks.spawn(async move {
                        // Endpoint failures stay inside this platform's retry loop.
                        loop {
                            let events = agent_event_source(token.clone(), id, base.clone(), context.targets.clone(), sender.clone(), context.metrics.clone(), reconnect_delay);
                            let stats = agent_container_stats(token.clone(), id, base.clone(), context.clone(), reconnect_delay);
                            let (_, result) = tokio::join!(events, stats);
                            if let Err(error) = result { tracing::warn!(%error, %id, "Agent subscription failed"); }
                            if wait_to_reconnect(&token, reconnect_delay).await { break; }
                        }
                    });
                    active.insert(id, (address, owned_token, task.id()));
                }
            }
        }
    }
    for (_, token, _) in active.values() {
        token.cancel();
    }
    while tasks.join_next().await.is_some() {}
    Ok(())
}

pub(super) async fn agent_subscription(
    targets: &PlatformRuntimeRegistry,
    platform: uuid::Uuid,
) -> Result<Option<(uuid::Uuid, AgentClient)>, RuntimeCapabilityError> {
    Ok(targets.agent(platform).await.map(|agent| (platform, agent)))
}
