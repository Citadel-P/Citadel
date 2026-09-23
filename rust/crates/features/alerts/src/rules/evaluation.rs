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
    match (rule.threshold, observation.value) {
        (Some(threshold), Some(value)) => value.is_finite() && value >= threshold,
        (Some(_), None) => false,
        (None, _) => observation.matched,
    }
}
