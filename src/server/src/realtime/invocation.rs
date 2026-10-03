use super::{
    RealtimeError, RealtimeService,
    authorization::{Subscription, map_realtime_read_error},
    connection::ConnectionIo,
    groups::{GroupConnection, send_group_message},
};
use crate::realtime_groups::{Group, GroupSubscription, MAX_GROUPS};
use serde_json::json;
use tokio_util::sync::CancellationToken;

impl GroupConnection {
    pub(super) async fn invoke(
        &mut self,
        text: &str,
        socket: &mut ConnectionIo,
        service: &RealtimeService,
        subscription: &mut Subscription,
        cancellation: &CancellationToken,
    ) -> Result<(), RealtimeError> {
        let reader = service
            .inner
            .groups
            .as_ref()
            .ok_or(RealtimeError::Authorization)?;
        let Self {
            groups,
            streams,
            stream_guards,
            terminal_inputs,
        } = self;
        let request = super::protocol::parse_invocation(text)?;
        let invocation = request.id;
        subscription.recheck(service).await?;
        let name = request
            .arguments
            .as_ref()
            .and_then(|args| (args.len() == 1).then(|| args[0].as_str()).flatten());
        let mut updates = vec![];
        let error = match (request.target.as_deref(), name) {
            (Some("LeaveGroup"), Some(name)) => {
                // This coordinator is the sole enqueue producer. Removing the sources
                // before enqueueing completion creates a FIFO unsubscribe barrier: older
                // messages may precede completion, but none can follow it for this group.
                groups.remove(name);
                streams.remove(name);
                stream_guards.remove(name);
                terminal_inputs.remove(name);
                tracing::debug!(connection_id = %socket.id, topic = name, "realtime group left");
                None
            }
            (Some("JoinGroup"), Some(name)) => {
                if !groups.contains_key(name) && groups.len() >= MAX_GROUPS {
                    Some("Realtime group limit exceeded")
                } else if let Some(group) = Group::parse(name) {
                    match tokio::time::timeout(
                        service.inner.subscribe_timeout,
                        reader.read_with_lease(
                            &subscription.principal,
                            &group,
                            None,
                            &subscription.lease,
                        ),
                    )
                    .await
                    {
                        Ok(Ok(snapshot)) => {
                            if groups.contains_key(name) {
                                // Revalidate access even on an idempotent join.
                                send_group_message(socket,service,&json!({"protocolVersion":1,"kind":"completion","invocationId":invocation,"error":null})).await?;
                                return Ok(());
                            }
                            let mut joined = GroupSubscription::new(group);
                            updates = joined
                                .apply(snapshot, service.inner.snapshot_limit)
                                .map_err(map_realtime_read_error)?;
                            groups.insert(name.into(), joined);
                            tracing::debug!(connection_id = %socket.id, topic = name, "realtime group joined");
                            None
                        }
                        Ok(Err(_)) => Some(
                            "Not authorized to join this group, or the resource is unavailable.",
                        ),
                        Err(_) => Some("Realtime group initialization timed out"),
                    }
                } else {
                    Some("Not authorized to join this group.")
                }
            }
            (Some(target), _) => {
                let args = request.arguments.as_deref().unwrap_or_default();
                let terminal = tokio::time::timeout(
                    service.inner.subscribe_timeout,
                    reader.terminal_invocation(&subscription.principal, target, args),
                )
                .await;
                let terminal_error = match terminal {
                    Ok(Ok(Some(command))) => {
                        use crate::realtime_groups::TerminalAction;
                        let error = if !groups.contains_key(&command.group.name) {
                            Some("Join the terminal group before starting or sending input.")
                        } else {
                            match command.action {
                                TerminalAction::Input(input) => {
                                    match terminal_inputs.get(&command.group.name) {
                                        Some(sender) => sender.try_send(input).err().map(
                                            |_| "Terminal input is closed, invalid or overloaded.",
                                        ),
                                        None => Some(
                                            "This connection does not own an active terminal session.",
                                        ),
                                    }
                                }
                                TerminalAction::Start(shell) => {
                                    if terminal_inputs.contains_key(&command.group.name) {
                                        None
                                    }
                                    // The legacy SendContainerExec event has no session discriminator.
                                    else if !terminal_inputs.is_empty()
                                        || stream_guards.len() >= 4
                                    {
                                        Some(
                                            "An active terminal already exists on this connection.",
                                        )
                                    } else {
                                        let token = cancellation.child_token();
                                        let guard = token.clone().drop_guard();
                                        match tokio::time::timeout(
                                            service.inner.subscribe_timeout,
                                            reader.terminal(
                                                &subscription.principal,
                                                &command.group,
                                                shell,
                                                &token,
                                            ),
                                        )
                                        .await
                                        {
                                            Ok(Ok(session)) => {
                                                terminal_inputs.insert(
                                                    command.group.name.clone(),
                                                    session.input,
                                                );
                                                streams.insert(
                                                    command.group.name.clone(),
                                                    session.output,
                                                );
                                                stream_guards.insert(command.group.name, guard);
                                                None
                                            }
                                            _ => Some(
                                                "Terminal target is unauthorized, unavailable or timed out.",
                                            ),
                                        }
                                    }
                                }
                            }
                        };
                        Some(error)
                    }
                    Ok(Ok(None)) => None,
                    _ => Some(Some(
                        "Terminal request is invalid, unauthorized or unavailable.",
                    )),
                };
                if let Some(error) = terminal_error {
                    send_group_message(socket,service,&json!({"protocolVersion":1,"kind":"completion","invocationId":invocation,"error":error})).await?;
                    return Ok(());
                }
                match tokio::time::timeout(
                    service.inner.subscribe_timeout,
                    reader.invocation_group(&subscription.principal, target, args),
                )
                .await
                {
                    Ok(Ok(Some(group))) if groups.contains_key(&group.name) => {
                        if stream_guards.contains_key(&group.name) {
                            None
                        } else if stream_guards.len() >= 4 {
                            Some("Realtime stream limit exceeded")
                        } else {
                            let token = cancellation.child_token();
                            let guard = token.clone().drop_guard();
                            match tokio::time::timeout(
                                service.inner.subscribe_timeout,
                                reader.stream(&subscription.principal, &group, &token),
                            )
                            .await
                            {
                                Ok(Ok(Some(stream))) => {
                                    streams.insert(group.name.clone(), stream);
                                    stream_guards.insert(group.name, guard);
                                    None
                                }
                                Ok(Ok(None)) => Some("This resource does not support streaming."),
                                Ok(Err(_)) => Some(
                                    "Not authorized to stream this resource, or the resource is unavailable.",
                                ),
                                Err(_) => Some("Realtime stream initialization timed out"),
                            }
                        }
                    }
                    _ => Some(
                        "Not authorized to invoke this method, or its resource group has not been joined.",
                    ),
                }
            }
            _ => Some("This realtime method is not available in the Rust backend."),
        };
        send_group_message(socket,service,&json!({"protocolVersion":1,"kind":"completion","invocationId":invocation,"error":error})).await?;
        for update in updates {
            send_group_message(socket, service, &update).await?;
        }
        Ok(())
    }
}
