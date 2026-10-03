use super::*;
use futures_util::stream;

#[tokio::test(start_paused = true)]
async fn six_events_and_barrier_preserve_order() {
    let cancellation = CancellationToken::new();
    let mut stream = Box::pin(stream::iter([1, 2, 3, 4, 5, 99, 6]));
    let (batch, pending) = gather(0, stream.as_mut(), |_, next| *next < 99, &cancellation)
        .await
        .unwrap();
    assert_eq!(batch, [0, 1, 2, 3, 4, 5]);
    assert_eq!(pending, Some(99));
    assert_eq!(stream.next().await, Some(6));
}

#[tokio::test(start_paused = true)]
async fn raw_duplicates_are_bounded() {
    let cancellation = CancellationToken::new();
    let mut stream = Box::pin(stream::repeat(1));
    let (batch, pending) = gather(1, stream.as_mut(), |_, _| true, &cancellation)
        .await
        .unwrap();
    assert_eq!(batch.len(), MAX_EVENTS);
    assert!(pending.is_none());
    assert_eq!(stream.next().await, Some(1));
}

#[tokio::test(start_paused = true)]
async fn deadline_is_fixed_and_stream_survives_window() {
    let cancellation = CancellationToken::new();
    let stream = async_stream::stream! {
        for value in 1..=4 {
            tokio::time::sleep(WINDOW * 2 / 5).await;
            yield value;
        }
    };
    tokio::pin!(stream);
    let start = tokio::time::Instant::now();
    let (batch, carry) = gather(0, stream.as_mut(), |_, _| true, &cancellation)
        .await
        .unwrap();
    assert_eq!(batch, [0, 1, 2]);
    assert!(carry.is_none());
    assert_eq!(start.elapsed(), WINDOW);
    assert_eq!(stream.next().await, Some(3));
}

#[tokio::test(start_paused = true)]
async fn cancellation_interrupts_window() {
    let cancellation = CancellationToken::new();
    let mut stream = Box::pin(stream::pending::<u8>());
    let cancel = async {
        tokio::time::sleep(Duration::from_millis(3)).await;
        cancellation.cancel();
    };
    let (batch, ()) = tokio::join!(
        gather(0, stream.as_mut(), |_, _| true, &cancellation),
        cancel
    );
    assert!(batch.is_none());
}

#[test]
fn newest_timestamp_wins_and_equal_timestamp_uses_arrival_order() {
    let events = vec![
        ("a", 30, "start"),
        ("a", 20, "die"),
        ("b", 10, "start"),
        ("b", 10, "die"),
    ];
    assert_eq!(
        coalesce(events, |e| e.0, |e| e.1),
        [("a", 30, "start"), ("b", 10, "die")]
    );
}
