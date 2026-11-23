//! Bounded event gathering with an explicit carry across ordering barriers.
use futures_util::{Stream, StreamExt};
use std::{pin::Pin, time::Duration};
use tokio_util::sync::CancellationToken;

pub const MAX_EVENTS: usize = 256;
// Leave headroom inside the 150 ms command confirmation deadline, while
// combining Docker completions that arrive tens of milliseconds apart.
pub const WINDOW: Duration = Duration::from_millis(75);

/// Never read past a barrier: return it to the caller before consuming more.
/// Count raw events, including duplicates, so a hot identity cannot starve others.
pub async fn gather<T, S: Stream<Item = T> + ?Sized>(
    first: T,
    mut stream: Pin<&mut S>,
    mut compatible: impl FnMut(&T, &T) -> bool,
    cancellation: &CancellationToken,
) -> Option<(Vec<T>, Option<T>)> {
    let deadline = tokio::time::sleep(WINDOW);
    tokio::pin!(deadline);
    let mut batch = vec![first];
    while batch.len() < MAX_EVENTS {
        let next = tokio::select! {
            biased;
            () = cancellation.cancelled() => return None,
            () = &mut deadline => break,
            next = stream.next() => next,
        };
        let Some(next) = next else { break };
        if !compatible(&batch[0], &next) {
            return Some((batch, Some(next)));
        }
        batch.push(next);
    }
    Some((batch, None))
}
