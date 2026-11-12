use crate::AlertObservation;
use crate::AlertRule;
use serde_json::Value;
use uuid::Uuid;
pub fn rule_applies(rule: &AlertRule, resource_id: Uuid) -> bool {
    rule.limited_to.is_empty()
        || rule.limited_to.iter().any(|scope| {
            ["resourceId", "ResourceId"]
                .iter()
                .find_map(|key| scope.get(*key).and_then(Value::as_str))
                .and_then(|value| Uuid::parse_str(value).ok())
                == Some(resource_id)
        })
}

pub fn observation_matches(rule: &AlertRule, observation: &AlertObservation) -> bool {
    if rule.alert_type == "BuildAgentPoolUnavailable" {
        return observation.matched
            && observation.value.is_some_and(|seconds| {
                seconds.is_finite() && seconds >= rule.threshold.unwrap_or(90.0)
            });
    }
    match (rule.threshold, observation.value) {
        (Some(threshold), Some(value)) => value.is_finite() && value >= threshold,
        (Some(_), None) => false,
        (None, _) => observation.matched,
    }
}

/// Rules arrive in severity order. Select the winner before cooldown so a
/// suppressed winner never falls through to a lower-severity duplicate.
/// Nonmatches still reset counters and resolve incidents.
pub fn select_observation_rules<'a>(
    rules: &'a [AlertRule],
    observation: &AlertObservation,
    advanced: bool,
) -> (Vec<&'a AlertRule>, Option<&'a AlertRule>) {
    let mut resets = Vec::new();
    let mut winner = None;
    for rule in rules.iter().filter(|rule| {
        (advanced || rule.audit.created_by_actor_id.value() == Uuid::from_u128(1))
            && rule_applies(rule, observation.resource_id)
            && !crate::is_in_quiet_hours(&rule.quiet_hours, observation.observed_at)
    }) {
        if observation_matches(rule, observation) {
            winner.get_or_insert(rule);
        } else {
            resets.push(rule);
        }
    }
    (resets, winner)
}
