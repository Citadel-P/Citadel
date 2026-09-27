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
                    let read = tokio::time::timeout(service.inner.subscribe_timeout, reader.read_with_lease(&subscription.principal, &joined.group, None, &subscription.lease));
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
                // Background statistics and unrelated resources should not cause
                // a database authentication read for every open browser socket.
                if event.resource_type!="License" && !state.groups.values().any(|g|g.group.affected_by(&event)) {
                    continue;
                }
                // Readers without a generation contract retain their conservative
                // authentication path. Production connections are commit-fenced.
                if !subscription.lease.tracked() { subscription.recheck(service).await?; }
                if event.resource_type=="License" {
                    send_group_message(socket,service,&crate::realtime_groups::ClientEvent::new("LicenseStateChanged",vec![])).await?;
                    continue;
                }
                for joined in state.groups.values_mut().filter(|g|g.group.affected_by(&event)) {
                    // Inventory I/O must not pause this connection's active streams.
                    let snapshot={
                        let read=tokio::time::timeout(service.inner.subscribe_timeout,reader.read_with_lease(&subscription.principal,&joined.group,Some(&event),&subscription.lease));
                        tokio::pin!(read);
                        loop {
                            tokio::select! {
                                ()=cancellation.cancelled()=>return Ok(()),
                                result=&mut read=>break result.map_err(|_|RealtimeError::SubscribeTimeout)?.map_err(map_realtime_read_error)?,
                                Some((_,item))=state.streams.next(),if !state.streams.is_empty()=>{
                                    send_group_message(socket,service,&item.map_err(map_realtime_read_error)?).await?;
                                }
                            }
                        }
                    };
                    for update in joined.apply(snapshot,service.inner.snapshot_limit).map_err(map_realtime_read_error)? {
                        send_group_message(socket,service,&update).await?;
                    }
                }
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
    pub(super) streams: tokio_stream::StreamMap<String, crate::realtime_groups::GroupStream>,
    pub(super) stream_guards: std::collections::BTreeMap<String, tokio_util::sync::DropGuard>,
    pub(super) terminal_inputs:
        std::collections::BTreeMap<String, citadel_platforms::terminal::TerminalInputSender>,
}
