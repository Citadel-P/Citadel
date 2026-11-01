use super::*;
use citadel_adapters::connectors::edge::EdgeTarget;
use citadel_contracts::citadel::{
    containers::v1::ContainersStatsResponse, edge::v1::core_envelope,
};
use futures_util::poll;
use prost::Message;
use uuid::Uuid;

async fn observe_pair(
    session: Arc<EdgeSession>,
    cancel: &CancellationToken,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut events = session.command(
        EdgeCommandKind::PlatformDaemonEventsStream,
        vec![],
        OBSERVATION_STREAM_TIMEOUT,
        true,
    )?;
    let mut stats =
        EdgeRuntime { session }.stream_container_stats(Duration::from_secs(10), cancel)?;
    loop {
        tokio::select! {
            event = events.next(cancel) => { event?.expect("event stream remains open"); },
            sample = stats.next() => { sample.expect("statistics stream remains open")?; },
        }
    }
}

#[tokio::test(start_paused = true)]
async fn healthy_observation_cycles_renew_before_deadline_and_release_commands() {
    let registry = EdgeRegistry::default();
    let (session, mut outbound) = registry
        .register(EdgeTarget::platform(Uuid::now_v7()), Uuid::now_v7())
        .unwrap();
    let cancel = CancellationToken::new();
    let mut previous = std::collections::HashSet::new();
    // Cross multiple hourly deadlines while retaining the authenticated session.
    for _ in 0..3 {
        let mut cycle = Box::pin(observation_cycle(observe_pair(session.clone(), &cancel)));
        assert!(poll!(cycle.as_mut()).is_pending());
        let mut ids = std::collections::HashSet::new();
        for _ in 0..2 {
            let envelope = outbound.try_recv().unwrap();
            let id = Uuid::parse_str(&envelope.command_id).unwrap();
            assert!(previous.insert(id));
            ids.insert(id);
            let Some(core_envelope::Body::Command(command)) = envelope.body else {
                panic!("expected observation command")
            };
            assert_eq!(
                command.timeout_ms,
                OBSERVATION_STREAM_TIMEOUT.as_millis() as i32
            );
            let payload = if command.kind == EdgeCommandKind::ContainersStatsStream as i32 {
                ContainersStatsResponse::default().encode_to_vec()
            } else {
                vec![]
            };
            session.output(id, payload);
        }
        assert!(poll!(cycle.as_mut()).is_pending());
        tokio::time::advance(OBSERVATION_STREAM_TIMEOUT - Duration::from_secs(301)).await;
        assert!(poll!(cycle.as_mut()).is_pending());
        tokio::time::advance(Duration::from_secs(1)).await;
        assert!(matches!(
            poll!(cycle.as_mut()),
            std::task::Poll::Ready(Ok(()))
        ));
        assert_eq!(session.pending_count(), 0);
        assert!(!session.is_closed());
        for _ in 0..2 {
            let envelope = outbound.try_recv().unwrap();
            assert!(ids.remove(&Uuid::parse_str(&envelope.command_id).unwrap()));
            assert!(matches!(
                envelope.body,
                Some(core_envelope::Body::CancelCommand(_))
            ));
        }
        assert!(ids.is_empty());
        assert!(outbound.try_recv().is_err());
    }
}

#[tokio::test(start_paused = true)]
async fn renewal_does_not_hide_real_timeouts_disconnects_or_cancellation() {
    for failure in ["deadline_exceeded", "unavailable", "disconnect", "cancel"] {
        let registry = EdgeRegistry::default();
        let (session, mut outbound) = registry
            .register(EdgeTarget::platform(Uuid::now_v7()), Uuid::now_v7())
            .unwrap();
        let cancel = CancellationToken::new();
        let mut cycle = Box::pin(observation_cycle(observe_pair(session.clone(), &cancel)));
        assert!(poll!(cycle.as_mut()).is_pending());
        let id = Uuid::parse_str(&outbound.try_recv().unwrap().command_id).unwrap();
        match failure {
            "disconnect" => session.close(),
            "cancel" => cancel.cancel(),
            code => {
                session.fail(id, code);
            }
        }
        assert!(cycle.await.is_err(), "{failure} must remain an error");
        assert_eq!(session.pending_count(), 0);
    }
}
