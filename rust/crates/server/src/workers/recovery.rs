//! Startup recovery, delayed/coalescing claim notifications, and an independent
//! five-minute safety net. Notifications never postpone the safety deadline.
use std::time::Duration;
use tokio::{sync::watch, time::Instant};
use tokio_util::sync::CancellationToken;

pub(super) const FALLBACK: Duration = Duration::from_secs(5 * 60);

pub(super) struct ActiveWake {
    signals: watch::Receiver<()>,
    ticker: tokio::time::Interval,
    pub active: bool,
    startup: bool,
    closed: bool,
}

impl ActiveWake {
    pub fn new(signals: watch::Receiver<()>) -> Self {
        Self {
            signals,
            ticker: super::schedule::interval("service-observer", FALLBACK),
            active: false,
            startup: true,
            closed: false,
        }
    }

    pub async fn next(&mut self, token: &CancellationToken) -> bool {
        if token.is_cancelled() {
            return false;
        }
        if std::mem::take(&mut self.startup) {
            return true;
        }
        loop {
            tokio::select! {
                biased;
                () = token.cancelled() => return false,
                _ = self.ticker.tick() => return true,
                _ = tokio::time::sleep(Duration::from_secs(5)), if self.active => return true,
                changed = self.signals.changed(), if !self.closed => {
                    if changed.is_ok() { return true; }
                    self.closed = true;
                },
            }
        }
    }
}

pub(super) struct RecoveryWake {
    signals: watch::Receiver<()>,
    ticker: tokio::time::Interval,
    delay: Duration,
    pending: Option<Instant>,
    startup: bool,
    closed: bool,
}

impl RecoveryWake {
    pub fn new(name: &str, signals: watch::Receiver<()>, delay: Duration) -> Self {
        Self {
            signals,
            ticker: super::schedule::interval(name, FALLBACK),
            delay,
            pending: None,
            startup: true,
            closed: false,
        }
    }

    pub async fn next(&mut self, token: &CancellationToken) -> bool {
        if token.is_cancelled() {
            return false;
        }
        if std::mem::take(&mut self.startup) {
            return true;
        }
        loop {
            tokio::select! {
                biased;
                () = token.cancelled() => return false,
                _ = self.ticker.tick() => return true,
                () = async {
                    match self.pending {
                        Some(deadline) => tokio::time::sleep_until(deadline).await,
                        None => std::future::pending().await,
                    }
                } => {
                    self.pending = None;
                    return true;
                },
                changed = self.signals.changed(), if !self.closed => {
                    if changed.is_err() {
                        self.closed = true;
                    } else {
                        // The claim is not stale when committed. One deferred
                        // pass covers the burst without an idle retry loop.
                        self.pending.get_or_insert(Instant::now() + self.delay);
                    }
                },
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn active_observer_wakes_promptly_then_sleeps_when_rollouts_finish() {
        let (sender, receiver) = watch::channel(());
        let token = CancellationToken::new();
        let mut wake = ActiveWake::new(receiver);
        assert!(wake.next(&token).await);
        let start = Instant::now();
        sender.send_modify(|_| {});
        assert!(wake.next(&token).await);
        assert_eq!(Instant::now(), start);
        wake.active = true;
        assert!(wake.next(&token).await);
        assert_eq!(Instant::now() - start, Duration::from_secs(5));
        wake.active = false;
        assert!(wake.next(&token).await);
        assert!(Instant::now() >= start + FALLBACK);
        token.cancel();
        assert!(!wake.next(&token).await);
    }

    #[tokio::test(start_paused = true)]
    async fn idle_scans_only_at_startup_and_five_minute_fallback() {
        let (_sender, receiver) = watch::channel(());
        let token = CancellationToken::new();
        let mut wake = RecoveryWake::new("test", receiver, Duration::from_secs(65));
        let start = Instant::now();
        assert!(wake.next(&token).await);
        assert_eq!(Instant::now(), start);
        // This is also the recovery path when a committed notification is lost.
        assert!(wake.next(&token).await);
        assert!(Instant::now() >= start + FALLBACK);
        assert!(Instant::now() < start + FALLBACK + Duration::from_secs(1));
        token.cancel();
        assert!(!wake.next(&token).await);
    }

    #[tokio::test(start_paused = true)]
    async fn busy_signals_coalesce_and_closed_listener_cannot_spin() {
        let (sender, receiver) = watch::channel(());
        let token = CancellationToken::new();
        let mut wake = RecoveryWake::new("test", receiver, Duration::from_secs(65));
        assert!(wake.next(&token).await);
        for _ in 0..1000 {
            sender.send_modify(|_| {});
        }
        let start = Instant::now();
        assert!(wake.next(&token).await);
        assert_eq!(Instant::now() - start, Duration::from_secs(65));
        drop(sender);
        assert!(wake.next(&token).await);
        assert!(Instant::now() >= start + FALLBACK);
    }

    #[tokio::test(start_paused = true)]
    async fn signals_do_not_postpone_crash_recovery() {
        let (sender, receiver) = watch::channel(());
        let token = CancellationToken::new();
        let mut wake = RecoveryWake::new("test", receiver, Duration::from_secs(960));
        assert!(wake.next(&token).await);
        sender.send_modify(|_| {});
        let start = Instant::now();
        assert!(wake.next(&token).await);
        assert!(Instant::now() < start + FALLBACK + Duration::from_secs(1));
    }
}
