//! One release check and its persisted deadline. The shared coordinator owns the timer.
use chrono::{DateTime, Utc};
use citadel_updates::{ReleaseCheckPolicy, ReleaseCheckService, UpdateError};
use tokio_util::sync::CancellationToken;

pub(crate) async fn deadline(
    service: &ReleaseCheckService,
    enabled: bool,
    hours: i32,
) -> Result<Option<DateTime<Utc>>, String> {
    service
        .resolve_installed_alert()
        .await
        .map_err(|e| e.to_string())?;
    if !enabled {
        service.clear_notice();
        return Ok(None);
    }
    if service.available().is_none() {
        service.restore_notice().await.map_err(|e| e.to_string())?;
    }
    let state = service.status().await.map_err(|e| e.to_string())?;
    Ok(Some(state.last_attempt.map_or_else(Utc::now, |at| {
        at + chrono::Duration::hours(i64::from(hours))
    })))
}
pub(crate) async fn check(
    service: &ReleaseCheckService,
    enabled: bool,
    hours: i32,
    cancel: &CancellationToken,
) -> Result<(), String> {
    if !enabled {
        return Ok(());
    }
    let result = service
        .check_scheduled(
            ReleaseCheckPolicy {
                initialized: true,
                checks_allowed: true,
            },
            chrono::Duration::hours(i64::from(hours)),
            cancel,
        )
        .await;
    match result {
        Ok(_) | Err(UpdateError::Cancelled) => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}
