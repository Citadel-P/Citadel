//! Supervised scheduling for the Updates feature service.
use citadel_identity::IdentityService;
use citadel_updates::{ReleaseCheckPolicy, ReleaseCheckService, UpdateError};
use std::{sync::Arc, time::Duration};
use tokio_util::sync::CancellationToken;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct CheckSchedule {
    enabled: bool,
    interval_hours: i32,
}

impl Default for CheckSchedule {
    fn default() -> Self {
        Self {
            enabled: false,
            interval_hours: 6,
        }
    }
}

fn remaining_delay(
    last_attempt: Option<chrono::DateTime<chrono::Utc>>,
    interval_hours: i32,
    now: chrono::DateTime<chrono::Utc>,
) -> Duration {
    last_attempt
        .map(|last| last + chrono::Duration::hours(i64::from(interval_hours)) - now)
        .and_then(|remaining| remaining.to_std().ok())
        .unwrap_or(Duration::ZERO)
}

pub async fn run(
    service: ReleaseCheckService,
    identity: Arc<IdentityService>,
    cancellation: CancellationToken,
) -> Result<(), std::convert::Infallible> {
    let (enabled, receiver) = tokio::sync::watch::channel(CheckSchedule::default());
    // Join both owned futures: policy shutdown must not drop a check that has
    // already begun persistence. No detached tasks are created.
    let (_, result) = tokio::join!(
        watch_policy(identity, enabled, cancellation.clone()),
        run_enabled(service, receiver, cancellation),
    );
    result
}

async fn run_enabled(
    service: ReleaseCheckService,
    mut enabled: tokio::sync::watch::Receiver<CheckSchedule>,
    cancellation: CancellationToken,
) -> Result<(), std::convert::Infallible> {
    loop {
        if cancellation.is_cancelled() {
            return Ok(());
        }
        let schedule = *enabled.borrow_and_update();
        if !schedule.enabled {
            service.clear_notice();
            tokio::select! {
                biased;
                () = cancellation.cancelled() => return Ok(()),
                result = enabled.changed() => if result.is_err() { return Ok(()); },
            }
            continue;
        }
        if service.available().is_none() {
            tokio::select! {
                biased;
                () = cancellation.cancelled() => return Ok(()),
                changed = enabled.changed() => {
                    if changed.is_err() { return Ok(()); }
                    continue;
                },
                _ = service.restore_notice() => {},
            }
        }
        // Persisted attempts anchor the schedule across restarts and manual checks.
        // Re-read after every wake: a manual check may have moved the due time.
        let state = tokio::select! {
            biased;
            () = cancellation.cancelled() => return Ok(()),
            result = enabled.changed() => {
                if result.is_err() { return Ok(()); }
                continue;
            },
            result = service.status() => result,
        };
        let delay = match state {
            Ok(state) => remaining_delay(
                state.last_attempt,
                schedule.interval_hours,
                chrono::Utc::now(),
            ),
            Err(error) => {
                tracing::warn!(%error, "Could not read Citadel release-check schedule");
                Duration::from_secs(30)
            }
        };
        if !delay.is_zero() {
            tokio::select! {
                biased;
                () = cancellation.cancelled() => return Ok(()),
                result = enabled.changed() => if result.is_err() { return Ok(()); },
                () = tokio::time::sleep(delay) => {},
            }
            continue;
        }
        let attempt_token = cancellation.child_token();
        let attempt = service.check_scheduled(
            ReleaseCheckPolicy {
                initialized: true,
                checks_allowed: true,
            },
            chrono::Duration::hours(i64::from(schedule.interval_hours)),
            &attempt_token,
        );
        tokio::pin!(attempt);
        let mut policy_changed = false;
        let result = tokio::select! {
            biased;
            () = cancellation.cancelled() => {
                attempt_token.cancel();
                attempt.await
            },
            _ = enabled.changed() => {
                policy_changed = true;
                attempt_token.cancel();
                attempt.await
            },
            result = &mut attempt => result,
        };
        let retry = result.is_err();
        if let Err(error) = result
            && !matches!(error, UpdateError::Cancelled)
        {
            tracing::warn!(%error, "Could not persist Citadel release check");
        }
        if cancellation.is_cancelled() {
            return Ok(());
        }
        if policy_changed {
            if enabled.has_changed().is_err() {
                return Ok(());
            }
            continue;
        }
        // A failed persistence attempt has no timestamp to anchor the interval.
        // Back off rather than spinning; successful attempts use persisted state.
        if retry {
            tokio::select! {
                biased;
                () = cancellation.cancelled() => return Ok(()),
                result = enabled.changed() => if result.is_err() { return Ok(()); },
                () = tokio::time::sleep(Duration::from_secs(30)) => {},
            }
        }
    }
}

async fn watch_policy(
    identity: Arc<citadel_identity::IdentityService>,
    enabled: tokio::sync::watch::Sender<CheckSchedule>,
    cancellation: CancellationToken,
) {
    let mut changes = identity.installation_changes();
    let mut poll = tokio::time::interval(Duration::from_secs(30));
    poll.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    poll.reset();
    loop {
        let settings = tokio::select! {
            biased;
            () = cancellation.cancelled() => return,
            result = identity.installation_settings() => result,
        };
        let next = match settings {
            Ok(settings) => CheckSchedule {
                enabled: settings.initialized && settings.update_checks_enabled,
                interval_hours: settings.update_check_interval_hours,
            },
            Err(error) => {
                tracing::debug!(%error, "Citadel update-check configuration unavailable");
                CheckSchedule::default()
            }
        };
        enabled.send_if_modified(|value| {
            if *value == next {
                return false;
            }
            *value = next;
            true
        });
        tokio::select! {
            biased;
            () = cancellation.cancelled() => return,
            result = changes.changed() => if result.is_err() { return; },
            _ = poll.tick() => {},
        }
    }
}

#[cfg(test)]
mod tests;
