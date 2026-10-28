use crate::*;

/// Retained configuration can be edited or disabled after license expiry;
/// selecting Automatic placement requires the entitlement.
pub fn changes_paid_configuration(
    current: Option<&Script>,
    proposed: &ScriptConfiguration,
) -> bool {
    if matches!(proposed.executor, ScriptExecutor::Automatic)
        && current.is_none_or(|script| script.executor != proposed.executor)
    {
        return true;
    }
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
