use crate::*;

/// Code/description edits and disabling paid
/// triggers remain possible after license expiry.
pub fn changes_paid_trigger(
    current: Option<&AutomationAction>,
    proposed: &AutomationActionConfiguration,
) -> bool {
    let webhook = proposed
        .webhook
        .as_ref()
        .is_some_and(|config| config.enabled);
    let Some(current) = current else {
        return proposed.schedule_enabled || webhook;
    };
    proposed.enabled
        && ((!current.enabled && (proposed.schedule_enabled || webhook))
            || (proposed.schedule_enabled
                && (!current.schedule_enabled
                    || proposed.schedule_cron != current.schedule_cron
                    || proposed.schedule_time_zone.as_deref().unwrap_or("UTC")
                        != current.schedule_time_zone))
            || (webhook && proposed.webhook != current.webhook))
}
