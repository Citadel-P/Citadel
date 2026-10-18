use super::*;
use futures_util::{Sink, task::AtomicWaker};
use std::{
    pin::Pin,
    sync::{
        Mutex,
        atomic::{AtomicBool, Ordering},
    },
    task::{Context, Poll},
};
use tokio::sync::Notify;

#[derive(Default)]
struct Probe {
    blocked: AtomicBool,
    failed: AtomicBool,
    polled: Notify,
    sent: Notify,
    waker: AtomicWaker,
    messages: Mutex<Vec<Message>>,
}
struct TestSink(Arc<Probe>);
impl Sink<Message> for TestSink {
    type Error = std::io::Error;
    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.0.waker.register(cx.waker());
        self.0.polled.notify_one();
        if self.0.failed.load(Ordering::Acquire) {
            Poll::Ready(Err(std::io::Error::other("fixture write failure")))
        } else if self.0.blocked.load(Ordering::Acquire) {
            Poll::Pending
        } else {
            Poll::Ready(Ok(()))
        }
    }
    fn start_send(self: Pin<&mut Self>, message: Message) -> Result<(), Self::Error> {
        self.0.messages.lock().unwrap().push(message);
        self.0.sent.notify_one();
        Ok(())
    }
    fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }
    fn poll_close(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }
}
fn queue(capacity: usize, bytes: usize) -> (OutboundSender, mpsc::Receiver<Outbound>) {
    let (sender, receiver) = mpsc::channel(capacity);
    (
        OutboundSender {
            sender,
            bytes: Arc::new(Semaphore::new(bytes)),
            cancellation: CancellationToken::new(),
            metrics: Arc::new(Metrics::default()),
        },
        receiver,
    )
}
fn text(sender: &OutboundSender, value: &str) -> Result<(), RealtimeError> {
    sender.enqueue(Message::Text(value.to_owned().into()), value.len())
}
fn writer(
    sender: &OutboundSender,
    receiver: mpsc::Receiver<Outbound>,
    probe: Arc<Probe>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(write_connection(
        TestSink(probe),
        receiver,
        sender.cancellation.clone(),
        Duration::from_secs(2),
        sender.metrics.clone(),
    ))
}

#[tokio::test(start_paused = true)]
async fn blocked_writer_retains_in_flight_bytes_and_shutdown_discards_pending_output() {
    let (sender, receiver) = queue(4, 16);
    let probe = Arc::new(Probe::default());
    probe.blocked.store(true, Ordering::Release);
    text(&sender, "12345678").unwrap();
    let task = writer(&sender, receiver, probe.clone());
    probe.polled.notified().await;
    assert_eq!(sender.bytes.available_permits(), 8);
    text(&sender, "abcdefgh").unwrap();
    assert_eq!(sender.bytes.available_permits(), 0);
    sender.cancellation.cancel();
    task.await.unwrap();
    assert_eq!(sender.bytes.available_permits(), 16);
    assert!(probe.messages.lock().unwrap().is_empty());
    assert!(matches!(text(&sender, "stale"), Err(RealtimeError::Closed)));
}

#[tokio::test(start_paused = true)]
async fn full_slow_connection_does_not_block_or_cancel_another_connection() {
    let (slow, receiver) = queue(1, 100);
    let blocked = Arc::new(Probe::default());
    blocked.blocked.store(true, Ordering::Release);
    text(&slow, "in-flight").unwrap();
    let slow_task = writer(&slow, receiver, blocked.clone());
    blocked.polled.notified().await;
    text(&slow, "queued").unwrap();
    assert!(matches!(
        text(&slow, "overflow"),
        Err(RealtimeError::Overloaded)
    ));

    let (fast, receiver) = queue(8, 100);
    let probe = Arc::new(Probe::default());
    let fast_task = writer(&fast, receiver, probe.clone());
    for message in [
        "completion",
        "snapshot",
        "update",
        "leave-completion",
        "other-topic",
    ] {
        text(&fast, message).unwrap();
    }
    loop {
        if probe.messages.lock().unwrap().len() == 5 {
            break;
        }
        probe.sent.notified().await;
    }
    let messages: Vec<_> = probe
        .messages
        .lock()
        .unwrap()
        .iter()
        .map(|m| m.to_text().unwrap().to_owned())
        .collect();
    assert_eq!(
        messages,
        [
            "completion",
            "snapshot",
            "update",
            "leave-completion",
            "other-topic"
        ]
    );
    assert!(!fast.cancellation.is_cancelled());
    fast.cancellation.cancel();
    fast_task.await.unwrap();
    slow_task.await.unwrap();
}

#[tokio::test(start_paused = true)]
async fn byte_limit_is_independent_of_message_count_and_accepts_large_snapshots() {
    let (sender, receiver) = queue(8, OUTBOUND_BYTES);
    // The previous group limit is 4 MiB; a large valid group must fit intact.
    let snapshot = "x".repeat(4 * 1024 * 1024);
    for _ in 0..4 {
        text(&sender, &snapshot).unwrap();
    }
    assert!(matches!(text(&sender, "x"), Err(RealtimeError::Overloaded)));
    assert!(sender.cancellation.is_cancelled());
    drop(receiver);
    assert_eq!(sender.bytes.available_permits(), OUTBOUND_BYTES);
}

#[tokio::test(start_paused = true)]
async fn write_timeout_and_failure_cancel_connection_and_release_payloads() {
    for failure in [false, true] {
        let (sender, receiver) = queue(8, 100);
        let probe = Arc::new(Probe::default());
        probe.blocked.store(!failure, Ordering::Release);
        probe.failed.store(failure, Ordering::Release);
        text(&sender, "payload").unwrap();
        let task = writer(&sender, receiver, probe);
        task.await.unwrap();
        assert!(sender.cancellation.is_cancelled());
        assert_eq!(sender.bytes.available_permits(), 100);
    }
}

#[tokio::test]
async fn dropped_recipient_releases_capacity_and_cannot_retarget_replacement() {
    let (old, receiver) = queue(2, 100);
    text(&old, "pending").unwrap();
    drop(receiver);
    assert_eq!(old.bytes.available_permits(), 100);
    let (new, mut receiver) = queue(2, 100);
    assert!(matches!(text(&old, "stale"), Err(RealtimeError::Closed)));
    text(&new, "fresh").unwrap();
    let OutboundMessage::Frame(message) = receiver.try_recv().unwrap().message else {
        panic!("expected application frame")
    };
    assert_eq!(message.to_text().unwrap(), "fresh");
    assert!(receiver.try_recv().is_err());
}

#[tokio::test(start_paused = true)]
async fn heartbeat_releases_a_silent_peer_and_rejects_unmatched_pongs() {
    for reply in [None, Some(vec![0; 8])] {
        let (sender, mut receiver) = queue(4, 100);
        let (pong, replies) = watch::channel(Vec::new());
        let owner = sender.clone();
        let task = tokio::spawn(async move { heartbeat(&owner, replies).await });
        tokio::time::advance(HEARTBEAT_INTERVAL).await;
        let ping = receiver.recv().await.unwrap();
        assert!(matches!(
            ping.message,
            OutboundMessage::Frame(Message::Ping(_))
        ));
        if let Some(reply) = reply {
            pong.send_replace(reply);
        }
        tokio::time::advance(PONG_TIMEOUT).await;
        task.await.unwrap();
        assert!(sender.cancellation.is_cancelled());
    }
}

#[tokio::test(start_paused = true)]
async fn heartbeat_keeps_a_responsive_idle_peer_and_rejects_replayed_pongs() {
    let (sender, mut receiver) = queue(4, 100);
    let (pong, replies) = watch::channel(Vec::new());
    let owner = sender.clone();
    let task = tokio::spawn(async move { heartbeat(&owner, replies).await });
    let OutboundMessage::Frame(Message::Ping(first)) = receiver.recv().await.unwrap().message
    else {
        panic!("expected ping");
    };
    pong.send_replace(first.to_vec());
    tokio::task::yield_now().await;
    let OutboundMessage::Frame(Message::Ping(second)) = receiver.recv().await.unwrap().message
    else {
        panic!("expected ping");
    };
    assert_ne!(first, second);
    assert!(!sender.cancellation.is_cancelled());
    pong.send_replace(first.to_vec());
    tokio::time::advance(PONG_TIMEOUT).await;
    task.await.unwrap();
    assert!(sender.cancellation.is_cancelled());
}

#[tokio::test]
async fn control_frames_are_read_while_application_messages_are_pending() {
    let (outbound, mut output) = queue(4, 100);
    let (input, messages) = mpsc::channel(4);
    let (incoming, mut pending) = mpsc::channel(1);
    let (pong, mut replies) = watch::channel(Vec::new());
    let owner = outbound.clone();
    let task = tokio::spawn(async move {
        read_connection(
            tokio_stream::wrappers::ReceiverStream::new(messages),
            incoming,
            &owner,
            pong,
        )
        .await;
    });
    input
        .send(Ok(Message::Text("busy invocation".into())))
        .await
        .unwrap();
    input.send(Ok(Message::Ping(vec![1].into()))).await.unwrap();
    input.send(Ok(Message::Pong(vec![2].into()))).await.unwrap();
    replies.changed().await.unwrap();
    assert_eq!(*replies.borrow(), [2]);
    assert!(matches!(
        output.recv().await.unwrap().message,
        OutboundMessage::Flush
    ));
    assert!(!outbound.cancellation.is_cancelled());
    assert!(pending.try_recv().is_ok());
    input.send(Ok(Message::Close(None))).await.unwrap();
    task.await.unwrap();
    assert!(outbound.cancellation.is_cancelled());
}

#[tokio::test]
async fn inbound_overflow_cancels_its_connection_without_unbounded_buffering() {
    let (outbound, _output) = queue(4, 100);
    let (incoming, _pending) = mpsc::channel(1);
    let (pong, _replies) = watch::channel(Vec::new());
    let messages = futures_util::stream::iter([
        Ok(Message::Text("first".into())),
        Ok(Message::Text("overflow".into())),
    ]);
    read_connection(messages, incoming, &outbound, pong).await;
    assert!(outbound.cancellation.is_cancelled());
}
