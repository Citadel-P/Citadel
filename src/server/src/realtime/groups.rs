use super::connection::ConnectionIo;
use super::{
    RealtimeError, RealtimeService,
    authorization::{Subscription, map_realtime_read_error},
};
use axum::extract::ws::Message;
use futures_util::StreamExt;
use serde::Serialize;
use serde_json::json;
use tokio::{
    sync::broadcast,
    time::{Instant, MissedTickBehavior},
};
use tokio_util::sync::CancellationToken;

pub(super) async fn run_group_connection(
    socket: &mut ConnectionIo,
    service: &RealtimeService,
    mut subscription: Subscription,
    cancellation: &CancellationToken,
) -> Result<(), RealtimeError> {
    let reader = service
        .inner
        .groups
        .as_ref()
        .ok_or(RealtimeError::Authorization)?;
    let mut state = GroupConnection::default();
    let mut receiver = service.inner.hub.subscribe();
    send_group_message(
        socket,
        service,
        &json!({"protocolVersion":1,"kind":"subscribed"}),
    )
    .await?;
    let mut recheck = tokio::time::interval_at(
        Instant::now() + service.inner.authorization_recheck_interval,
        service.inner.authorization_recheck_interval,
    );
    recheck.set_missed_tick_behavior(MissedTickBehavior::Skip);
    loop {
        tokio::select! {
            ()=cancellation.cancelled()=>return Ok(()),
            message=socket.next()=>{
                let text=match message {
                    Some(Ok(Message::Text(text)))=>text,
                    Some(Ok(Message::Close(_)))|None=>return Ok(()),
                    Some(Ok(Message::Ping(_)|Message::Pong(_)))=>continue,
                    _=>return Err(RealtimeError::InvalidMessage("Expected a text invocation".into())),
                };
                state.invoke(&text, socket, service, &mut subscription, cancellation).await?;
            },
            _=recheck.tick()=>{
                subscription.recheck(service).await?;
                for joined in state.groups.values() {
                    let read = tokio::time::timeout(service.inner.subscribe_timeout, reader.recheck(&subscription.principal, &joined.group, &subscription.lease));
                    tokio::pin!(read);
                    loop {
                        tokio::select! {
                            ()=cancellation.cancelled()=>return Ok(()),
                            result=&mut read=>{ result.map_err(|_|RealtimeError::SubscribeTimeout)?.map_err(map_realtime_read_error)?; break; },
                            Some((_,item))=state.streams.next(),if !state.streams.is_empty()=>{
                                send_group_message(socket,service,&item.map_err(map_realtime_read_error)?).await?;
                            }
                        }
                    }
                }
            },
            Some((_,item))=state.streams.next(),if !state.streams.is_empty()=>{
                let update=item.map_err(map_realtime_read_error)?;
                send_group_message(socket,service,&update).await?;
            },
            // Only dirty dashboard projections have a deadline. Burst events share one authorized read.
            name = next_dashboard_read(&state.dashboard_due) => {
                state.dashboard_due.remove(&name);
                if let Some(joined) = state.groups.get_mut(&name) {
                    if !subscription.lease.tracked() { subscription.recheck(service).await?; }
                    let snapshot = read_group_snapshot(socket, service, &subscription, &joined.group, None, &mut state.streams).await?.map_err(map_realtime_read_error)?;
                    for update in joined.apply(snapshot, service.inner.snapshot_limit).map_err(map_realtime_read_error)? {
                        send_group_message(socket,service,&update).await?;
                    }
                }
            },
            event=receiver.recv()=>{
                let event=match event {
                    Ok(event)=>event,
                    Err(broadcast::error::RecvError::Lagged(_))=>{
                        // Close rather than dropping changes. The existing reconnect
                        // lifecycle rejoins groups and refreshes authoritative reads.
                        service.inner.metrics.realtime_overflowed();
                        return Err(RealtimeError::InvalidMessage("Realtime resynchronization required".into()));
                    },
                    Err(broadcast::error::RecvError::Closed)=>return Ok(()),
                };
                if event.event_kind == "resyncRequired" { return Err(RealtimeError::Overloaded); }
                // Background statistics and unrelated resources should not cause
                // a database authentication read for every open browser socket.
                let administrator_event = match event.resource_type {
                    "ApplicationInfo" => Some("ApplicationInfoChanged"),
                    "InstallationSettings" => Some("InstallationSettingsChanged"),
                    _ => None,
                }.filter(|_| subscription.principal.is_administrator());
                let tag_catalog_changed = matches!(event.event_kind, "tagChanged" | "resourceTagsChanged" | "tagUsageChanged");
                if !tag_catalog_changed && event.resource_type!="License" && administrator_event.is_none() && !state.groups.values().any(|g|g.expects_event(&event, &subscription.lease)) {
                    continue;
                }
                // Readers without a generation contract retain their conservative
                // authentication path. Production connections are commit-fenced.
                if !subscription.lease.tracked() { subscription.recheck(service).await?; }
                if tag_catalog_changed {
                    send_group_message(socket,service,&crate::realtime_groups::ClientEvent::new("TagCatalogChanged",vec![])).await?;
                }
                if event.resource_type=="License" {
                    send_group_message(socket,service,&crate::realtime_groups::ClientEvent::new("LicenseStateChanged",vec![])).await?;
                }
                if let Some(target) = administrator_event.filter(|_| subscription.principal.is_administrator()) {
                    send_group_message(socket,service,&crate::realtime_groups::ClientEvent::new(target,vec![])).await?;
                }
                let mut oversized = Vec::new();
                for (name,joined) in state.groups.iter_mut().filter(|(_,g)|g.expects_event(&event, &subscription.lease)) {
                    if joined.group.dashboard.is_some() {
                        state.dashboard_due.entry(name.clone()).or_insert_with(||Instant::now()+std::time::Duration::from_secs(2));
                        continue;
                    }
                    // Inventory I/O must not pause this connection's active streams.
                    let snapshot = read_group_snapshot(socket, service, &subscription, &joined.group, Some(&event), &mut state.streams).await?;
                    let updates = match snapshot.and_then(|s| joined.apply(s,service.inner.snapshot_limit)) {
                        Err(super::RealtimeReadError::SnapshotCapacity) => {
                            oversized.push(name.clone());
                            send_group_message(socket,service,&crate::realtime_groups::ClientEvent::new("RealtimeGroupCapacity",vec![json!(name)])).await?;
                            continue;
                        }
                        result => result.map_err(map_realtime_read_error)?,
                    };
                    for update in updates {
                        send_group_message(socket,service,&update).await?;
                    }
                }
                for name in oversized {
                    state.remove_group(&name);
                }
            }
        }
    }
}

async fn next_dashboard_read(pending: &std::collections::BTreeMap<String, Instant>) -> String {
    let Some((name, deadline)) = pending.iter().min_by_key(|(_, due)| *due) else {
        return std::future::pending().await;
    };
    tokio::time::sleep_until(*deadline).await;
    name.clone()
}

async fn read_group_snapshot(
    socket: &mut ConnectionIo,
    service: &RealtimeService,
    subscription: &Subscription,
    group: &crate::realtime_groups::Group,
    event: Option<&super::PublishedRuntimeEvent>,
    streams: &mut tokio_stream::StreamMap<String, crate::realtime_groups::GroupStream>,
) -> Result<Result<crate::realtime_groups::GroupSnapshot, super::RealtimeReadError>, RealtimeError>
{
    let reader = service
        .inner
        .groups
        .as_ref()
        .ok_or(RealtimeError::Authorization)?;
    let read = tokio::time::timeout(
        service.inner.subscribe_timeout,
        reader.read_with_lease(&subscription.principal, group, event, &subscription.lease),
    );
    tokio::pin!(read);
    loop {
        tokio::select! {
            result = &mut read => match result {
                Ok(snapshot) => return Ok(snapshot),
                Err(_) => {
                    // Process snapshots are best effort. A slow Docker read clears just that panel.
                    let super::topic::Topic::ContainerProcesses(id) = group.topic() else {
                        return Err(RealtimeError::SubscribeTimeout);
                    };
                    return Ok(Ok(crate::realtime_groups::GroupSnapshot {
                        events: vec![crate::realtime_groups::ClientEvent::new("ContainerProcessesUpdated", vec![json!({"resourceId": id, "data": null})])],
                        ..Default::default()
                    }));
                }
            },
            Some((_,item)) = streams.next(), if !streams.is_empty() => {
                send_group_message(socket,service,&item.map_err(map_realtime_read_error)?).await?;
            }
        }
    }
}

pub(super) async fn send_group_message(
    socket: &mut ConnectionIo,
    service: &RealtimeService,
    message: &impl Serialize,
) -> Result<(), RealtimeError> {
    let text = serde_json::to_string(message)
        .map_err(|error| RealtimeError::Serialization(error.to_string()))?;
    if text.len() > 4 * 1024 * 1024 {
        return Err(RealtimeError::InvalidMessage(
            "Realtime payload limit exceeded".into(),
        ));
    }
    super::connection::send_text(socket, service, text).await
}

#[derive(Default)]
pub(super) struct GroupConnection {
    pub(super) groups:
        std::collections::BTreeMap<String, crate::realtime_groups::GroupSubscription>,
    dashboard_due: std::collections::BTreeMap<String, Instant>,
    pub(super) streams: tokio_stream::StreamMap<String, crate::realtime_groups::GroupStream>,
    pub(super) stream_guards: std::collections::BTreeMap<String, tokio_util::sync::DropGuard>,
    pub(super) terminal_inputs:
        std::collections::BTreeMap<String, citadel_platforms::terminal::TerminalInputSender>,
}

impl GroupConnection {
    pub(super) fn remove_group(&mut self, name: &str) {
        self.groups.remove(name);
        self.dashboard_due.remove(name);
        self.streams.remove(name);
        self.stream_guards.remove(name);
        self.terminal_inputs.remove(name);
    }
}
