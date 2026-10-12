use super::{
    RealtimeError, RealtimeService,
    protocol::{ClientMessage, parse_client_message},
};
use crate::metrics::Metrics;
use axum::extract::ws::{Message, WebSocket};
use futures_util::StreamExt;
use std::{sync::Arc, time::Duration};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use tokio_util::sync::CancellationToken;
use tracing::Instrument;
use uuid::Uuid;

pub(super) async fn receive_initial_subscription(
    socket: &mut ConnectionIo,
    timeout: Duration,
) -> Result<ClientMessage, RealtimeError> {
    let message = tokio::time::timeout(timeout, socket.next())
        .await
        .map_err(|_| RealtimeError::SubscribeTimeout)?
        .ok_or(RealtimeError::Closed)?
        .map_err(|error| RealtimeError::Socket(error.to_string()))?;
    match message {
        Message::Text(text) => parse_client_message(text.as_str()),
        Message::Close(_) => Err(RealtimeError::Closed),
        _ => Err(RealtimeError::InvalidMessage(
            "the first message must be a text subscription".to_owned(),
        )),
    }
}

use futures_util::{SinkExt, stream::SplitStream};
use tokio::sync::mpsc;
use tokio_util::task::AbortOnDropHandle;

// Four maximum-size group payloads, including the message currently being written.
// This also bounds resource-mode snapshots; it does not retain a second payload copy.
const OUTBOUND_BYTES: usize = 16 * 1024 * 1024;

enum OutboundMessage {
    Frame(Message),
    // Tungstenite has already queued the automatic Pong. Flush it without
    // constructing a second Pong, even if the reader flushed it first.
    Flush,
}
struct Outbound {
    message: OutboundMessage,
    _bytes: OwnedSemaphorePermit,
}

pub(super) struct ConnectionIo {
    pub(super) id: Uuid,
    reader: SplitStream<WebSocket>,
    outbound: OutboundSender,
}

struct OutboundSender {
    sender: mpsc::Sender<Outbound>,
    bytes: Arc<Semaphore>,
    cancellation: CancellationToken,
    metrics: Arc<Metrics>,
}

impl OutboundSender {
    fn enqueue(&self, message: Message, size: usize) -> Result<(), RealtimeError> {
        self.enqueue_outbound(OutboundMessage::Frame(message), size)
    }
    fn enqueue_outbound(&self, message: OutboundMessage, size: usize) -> Result<(), RealtimeError> {
        if self.cancellation.is_cancelled() {
            return Err(RealtimeError::Closed);
        }
        let permit = u32::try_from(size.max(1))
            .ok()
            .and_then(|size| self.bytes.clone().try_acquire_many_owned(size).ok());
        let Some(permit) = permit else {
            return self.overloaded();
        };
        match self.sender.try_send(Outbound {
            message,
            _bytes: permit,
        }) {
            Ok(()) => Ok(()),
            Err(mpsc::error::TrySendError::Full(_)) => self.overloaded(),
            Err(mpsc::error::TrySendError::Closed(_)) => Err(RealtimeError::Closed),
        }
    }

    fn overloaded(&self) -> Result<(), RealtimeError> {
        self.metrics.realtime_overflowed();
        self.cancellation.cancel();
        Err(RealtimeError::Overloaded)
    }
}

impl ConnectionIo {
    pub(super) async fn next(&mut self) -> Option<Result<Message, axum::Error>> {
        // Reading queues automatic control responses inside Tungstenite. Wake the
        // sole writer even when there is no application traffic to flush them.
        let message = self.reader.next().await;
        if let Some(Ok(Message::Ping(_))) = &message {
            let _ = self.outbound.enqueue_outbound(OutboundMessage::Flush, 1);
        }
        message
    }

    pub(super) fn enqueue(&self, text: String) -> Result<(), RealtimeError> {
        let size = text.len();
        self.outbound.enqueue(Message::Text(text.into()), size)
    }
}

pub(super) async fn handle_connection(
    socket: WebSocket,
    service: RealtimeService,
    _permit: OwnedSemaphorePermit,
) {
    let _connection = service.inner.metrics.realtime_connection_guard();
    let cancellation = service.inner.shutdown.child_token();
    let _cancel_on_drop = cancellation.clone().drop_guard();
    let id = Uuid::now_v7();
    tracing::debug!(connection_id = %id, "realtime connection opened");
    let (sink, reader) = socket.split();
    let (sender, receiver) = mpsc::channel(service.inner.outbound_capacity);
    let mut io = ConnectionIo {
        id,
        reader,
        outbound: OutboundSender {
            sender,
            bytes: Arc::new(Semaphore::new(OUTBOUND_BYTES)),
            cancellation: cancellation.clone(),
            metrics: service.inner.metrics.clone(),
        },
    };
    let writer_task = write_connection(
        sink,
        receiver,
        cancellation.clone(),
        service.inner.send_timeout,
        service.inner.metrics.clone(),
    )
    .instrument(tracing::debug_span!("realtime_writer", connection_id = %id));
    let mut writer = AbortOnDropHandle::new(tokio::spawn(writer_task));
    let result = tokio::select! {
        biased;
        () = cancellation.cancelled() => Ok(()),
        result = super::subscription::run_connection(&mut io, &service, &cancellation) => result,
    };
    // All subscription-local guards have now dropped. Cancel before awaiting the
    // writer so authentication failure never drains pending application output.
    cancellation.cancel();
    drop(io);
    if tokio::time::timeout(service.inner.send_timeout, &mut writer)
        .await
        .is_err()
    {
        writer.abort();
        let _ = writer.await;
    }
    if let Err(error) = result {
        tracing::debug!(connection_id = %id, %error, "realtime reader closed");
    }
    tracing::debug!(connection_id = %id, "realtime connection closed");
}

async fn write_connection<S>(
    mut sink: S,
    mut receiver: mpsc::Receiver<Outbound>,
    cancellation: CancellationToken,
    timeout: Duration,
    metrics: Arc<Metrics>,
) where
    S: futures_util::Sink<Message> + Unpin,
    S::Error: std::fmt::Display,
{
    let _cancel_on_drop = cancellation.clone().drop_guard();
    loop {
        let item = tokio::select! {
            biased;
            () = cancellation.cancelled() => break,
            item = receiver.recv() => match item { Some(item) => item, None => break },
        };
        let application_message = matches!(item.message, OutboundMessage::Frame(Message::Text(_)));
        let write = async {
            match item.message {
                OutboundMessage::Frame(message) => sink.send(message).await,
                OutboundMessage::Flush => sink.flush().await,
            }
        };
        let result = tokio::select! {
            biased;
            () = cancellation.cancelled() => break,
            result = tokio::time::timeout(timeout, write) => result,
        };
        // Keep the byte permit until the entire send/flush finishes.
        drop(item._bytes);
        match result {
            Ok(Ok(())) => {
                if application_message {
                    metrics.realtime_message_sent();
                }
            }
            Ok(Err(error)) => {
                tracing::debug!(%error, "realtime writer failed");
                break;
            }
            Err(_) => {
                metrics.realtime_send_timed_out();
                tracing::debug!("realtime writer timed out");
                break;
            }
        }
    }
    cancellation.cancel();
    receiver.close();
    drop(receiver);
    let _ = tokio::time::timeout(timeout, sink.close()).await;
}

pub(super) async fn send_text(
    socket: &mut ConnectionIo,
    _service: &RealtimeService,
    text: String,
) -> Result<(), RealtimeError> {
    socket.enqueue(text)?;
    // Allow the writer to make progress during large snapshot batches rather
    // than filling the bounded queue in a loop with no scheduling points.
    tokio::task::yield_now().await;
    Ok(())
}

#[cfg(test)]
mod tests;
